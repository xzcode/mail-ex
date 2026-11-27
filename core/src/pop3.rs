//! POP3 over implicit TLS. UIDL is mandatory; server messages are never deleted.
use super::{Store,Account,NewAccount,SyncReport,parse,TIMEOUT};
use anyhow::{anyhow,bail,Context,Result};
use native_tls::{TlsConnector,TlsStream};
use rusqlite::{params,TransactionBehavior};
use sha2::{Digest,Sha256};
use std::{collections::HashSet,io::{BufRead,BufReader,Write},net::{TcpStream,ToSocketAddrs}};

struct PopSession{io:BufReader<TlsStream<TcpStream>>}
impl PopSession{
    fn connect(a:&Account,password:&str)->Result<Self>{
        if a.username.is_empty()||password.is_empty()||a.username.chars().any(char::is_control)||password.chars().any(char::is_control){bail!("Invalid POP3 credentials")}
        let mut tcp=None;
        for addr in (a.imap_host.as_str(),a.imap_port).to_socket_addrs().context("POP3 host resolution failed")?{
            if let Ok(s)=TcpStream::connect_timeout(&addr,TIMEOUT){tcp=Some(s);break}
        }
        let tcp=tcp.ok_or_else(||anyhow!("POP3 connection failed"))?;
        tcp.set_read_timeout(Some(TIMEOUT))?;tcp.set_write_timeout(Some(TIMEOUT))?;
        let tls=TlsConnector::new()?.connect(&a.imap_host,tcp).map_err(|_|anyhow!("POP3 TLS or certificate validation failed"))?;
        let mut session=Self{io:BufReader::new(tls)};
        session.status()?;
        session.command(&format!("USER {}",a.username)).map_err(|_|anyhow!("POP3 USER rejected"))?;
        session.command(&format!("PASS {password}")).map_err(|_|anyhow!("POP3 authentication failed"))?;
        Ok(session)
    }
    fn line(&mut self)->Result<Vec<u8>>{
        let mut line=Vec::new();
        if self.io.read_until(b'\n',&mut line)?==0||!line.ends_with(b"\r\n"){bail!("Incomplete POP3 response")}
        Ok(line)
    }
    fn status(&mut self)->Result<()>{
        let line=self.line()?;
        if line.starts_with(b"+OK ")||line==b"+OK\r\n"{Ok(())}else{bail!("POP3 command rejected")}
    }
    fn command(&mut self,command:&str)->Result<()>{
        if command.contains(['\r','\n']){bail!("Invalid POP3 command")}
        let stream=self.io.get_mut();stream.write_all(command.as_bytes())?;stream.write_all(b"\r\n")?;stream.flush()?;
        self.status()
    }
    fn multiline(&mut self,command:&str,mut consume:impl FnMut(&[u8])->Result<()>)->Result<()>{
        self.command(command)?;
        loop{
            let line=self.line()?;
            if line==b".\r\n"{break}
            let content=if line.starts_with(b".."){&line[1..]}else{&line[..]};
            consume(content)?;
        }
        Ok(())
    }
    fn uidl(&mut self)->Result<Vec<(u32,String)>>{
        let mut rows=Vec::new();let mut numbers=HashSet::new();let mut ids=HashSet::new();
        self.multiline("UIDL",|line|{
            let text=std::str::from_utf8(line).context("Invalid UIDL response")?;
            let mut parts=text.split_ascii_whitespace();
            let number:u32=parts.next().ok_or_else(||anyhow!("Invalid UIDL response"))?.parse()?;
            let uid=parts.next().ok_or_else(||anyhow!("Missing UIDL identifier"))?;
            if number==0||uid.is_empty()||uid.len()>70||!uid.bytes().all(|b|(33..=126).contains(&b))||parts.next().is_some()||!numbers.insert(number)||!ids.insert(uid.to_owned()){bail!("Invalid or duplicate POP3 UIDL")}
            rows.push((number,uid.to_owned()));Ok(())
        }).context("Server must support stable POP3 UIDL identifiers")?;
        Ok(rows)
    }
    fn retrieve(&mut self,number:u32)->Result<Vec<u8>>{
        let mut bytes=Vec::new();self.multiline(&format!("RETR {number}"),|line|{bytes.extend_from_slice(line);Ok(())})?;Ok(bytes)
    }
}
fn uid_hash(uid:&str)->String{format!("pop3:{:x}",Sha256::digest(uid.as_bytes()))}
impl Store{
    pub fn add_pop_account(&mut self,input:NewAccount)->Result<Account>{self.add_password_account(input,"pop3")}
    pub(crate) fn test_pop(&self,a:&Account,pw:&str)->Result<()>{
        let mut session=PopSession::connect(a,pw)?;
        session.uidl()?;session.command("QUIT")?;Ok(())
    }
    pub(crate) fn sync_pop(&mut self,a:&Account,pw:&str)->Result<SyncReport>{
        let mut session=PopSession::connect(a,pw)?;
        let entries=session.uidl()?;
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut next:i64=tx.query_row("SELECT coalesce(max(uid),0)+1 FROM messages WHERE account_id=? AND folder='INBOX'",[&a.id],|r|r.get(0))?;
        let mut report=SyncReport{added:0,folders:1};
        for (number,uid) in entries{
            let key=uid_hash(&uid);
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE account_id=? AND import_hash=?)",params![a.id,key],|r|r.get(0))?;
            if exists{continue}
            let raw=session.retrieve(number)?;let m=parse(&raw);
            tx.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred,import_hash) VALUES(?,'INBOX',1,?,?,?,?,?,0,?,?,?,0,?)",params![a.id,next,m.sender,m.recipients,m.subject,m.timestamp,m.body,m.html_only,raw,key])?;
            next+=1;report.added+=1;
        }
        // A message absent from the POP3 server remains in the local archive.
        session.command("QUIT")?;tx.commit()?;Ok(report)
    }
    pub(crate) fn append_pop_sent(&self,a:&Account,message:&lettre::Message)->Result<()>{
        let raw=message.formatted();let m=parse(&raw);
        let tx=self.db.unchecked_transaction()?;
        tx.execute("INSERT OR IGNORE INTO folders(account_id,name,label,kind) VALUES(?,'Sent','Sent (local)','sent')",[&a.id])?;
        tx.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred) VALUES(?1,'Sent',1,(SELECT coalesce(max(uid),0)+1 FROM messages WHERE account_id=?1 AND folder='Sent'),?2,?3,?4,?5,1,?6,?7,?8,0)",params![a.id,m.sender,m.recipients,m.subject,m.timestamp,m.body,m.html_only,raw])?;
        tx.commit()?;Ok(())
    }
}
