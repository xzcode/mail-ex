use aes_gcm::{aead::{Aead, KeyInit, OsRng, Payload, rand_core::RngCore}, Aes256Gcm, Nonce};
use anyhow::{anyhow, bail, Context, Result};
use lettre::{message::{header::ContentType, Mailbox}, transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use mailparse::{MailHeaderMap, ParsedMail};
use native_tls::{TlsConnector, TlsStream};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::{self, OpenOptions}, io::Write, net::{TcpStream, ToSocketAddrs}, path::Path, time::Duration};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String, pub name: String, pub email: String, pub username: String,
    pub imap_host: String, pub imap_port: u16, pub smtp_host: String, pub smtp_port: u16,
    pub smtp_starttls: bool, pub color: String,
}
#[derive(Deserialize)]
pub struct NewAccount {
    pub name: String, pub email: String, pub username: String, pub password: String,
    pub imap_host: String, pub imap_port: u16, pub smtp_host: String, pub smtp_port: u16,
    pub smtp_starttls: bool, pub color: String,
}
#[derive(Clone, Serialize)]
pub struct Summary {
    pub id: i64, pub account_id: String, pub sender: String, pub subject: String, pub timestamp: i64, pub seen: bool,
}
#[derive(Serialize)]
pub struct Mail {
    #[serde(flatten)] pub summary: Summary, pub body: String, pub html_only: bool, pub recipients: String,
}
#[derive(Deserialize)]
pub struct Compose {
    pub account_id: String, pub to: String, pub cc: String, pub bcc: String, pub subject: String, pub body: String,
}

const ACCOUNT_COLUMNS: &str = "id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color";
const SUMMARY_COLUMNS: &str = "id,account_id,sender,subject,timestamp,seen";
const TIMEOUT: Duration = Duration::from_secs(45);
type ImapSession = imap::Session<TlsStream<TcpStream>>;

fn account_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account { id:r.get(0)?, name:r.get(1)?, email:r.get(2)?, username:r.get(3)?,
        imap_host:r.get(4)?, imap_port:r.get(5)?, smtp_host:r.get(6)?, smtp_port:r.get(7)?, smtp_starttls:r.get(8)?, color:r.get(9)? })
}
fn summary_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Summary> {
    Ok(Summary { id:r.get(0)?, account_id:r.get(1)?, sender:r.get(2)?, subject:r.get(3)?, timestamp:r.get(4)?, seen:r.get(5)? })
}
fn valid_host(s: &str) -> bool {
    !s.is_empty() && s.len() <= 253 && s.chars().all(|c| c.is_ascii_alphanumeric() || c=='.' || c=='-' || c==':')
}
fn load_key(dir: &Path) -> Result<[u8;32]> {
    let path=dir.join("vault.key");
    if !path.exists() {
        let mut opts=OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; opts.mode(0o600); }
        match opts.open(&path) {
            Ok(mut file)=>{ let mut key=[0u8;32]; OsRng.fill_bytes(&mut key); file.write_all(&key)?; file.sync_all()?; },
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},
            Err(e)=>return Err(e.into()),
        }
    }
    fs::read(path)?.try_into().map_err(|_| anyhow!("Credential key is invalid; restore the original key"))
}
fn encrypt(key: &[u8;32], id: &str, secret: &str)->Result<Vec<u8>>{
    let cipher=Aes256Gcm::new_from_slice(key).map_err(|_|anyhow!("Invalid key"))?;
    let mut nonce=[0u8;12]; OsRng.fill_bytes(&mut nonce);
    let encrypted=cipher.encrypt(Nonce::from_slice(&nonce), Payload{msg:secret.as_bytes(),aad:id.as_bytes()}).map_err(|_|anyhow!("Credential encryption failed"))?;
    let mut bytes=nonce.to_vec(); bytes.extend(encrypted); Ok(bytes)
}
fn decrypt(key: &[u8;32], id: &str, bytes: &[u8])->Result<String>{
    if bytes.len()<28 { bail!("Invalid encrypted credential") }
    let cipher=Aes256Gcm::new_from_slice(key).map_err(|_|anyhow!("Invalid key"))?;
    let plain=cipher.decrypt(Nonce::from_slice(&bytes[..12]),Payload{msg:&bytes[12..],aad:id.as_bytes()}).map_err(|_|anyhow!("Credential decryption failed"))?;
    String::from_utf8(plain).context("Invalid credential encoding")
}

pub struct Store { db: Connection, key: [u8;32] }
impl Store {
    pub fn open(dir: &Path)->Result<Self>{
        fs::create_dir_all(dir)?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(dir,fs::Permissions::from_mode(0o700))?; }
        let db=Connection::open(dir.join("mail.sqlite3"))?;
        db.busy_timeout(Duration::from_secs(15))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let version:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0))?;
        if version>1 { bail!("Database belongs to a newer Mail EX version") }
        if version==0 {
            db.execute_batch("BEGIN IMMEDIATE;
              CREATE TABLE IF NOT EXISTS accounts(id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, username TEXT NOT NULL, imap_host TEXT NOT NULL, imap_port INTEGER NOT NULL, smtp_host TEXT NOT NULL, smtp_port INTEGER NOT NULL, smtp_starttls INTEGER NOT NULL, color TEXT NOT NULL, secret BLOB NOT NULL);
              CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE, folder TEXT NOT NULL, uid_validity INTEGER NOT NULL, uid INTEGER NOT NULL, sender TEXT NOT NULL, recipients TEXT NOT NULL, subject TEXT NOT NULL, timestamp INTEGER NOT NULL, seen INTEGER NOT NULL, body TEXT NOT NULL, html_only INTEGER NOT NULL, raw BLOB NOT NULL, UNIQUE(account_id,folder,uid_validity,uid));
              CREATE INDEX IF NOT EXISTS messages_order ON messages(timestamp DESC,id DESC);
              CREATE INDEX IF NOT EXISTS messages_account ON messages(account_id,folder,uid_validity,uid);
              PRAGMA user_version=1; COMMIT;")?;
        }
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; fs::set_permissions(dir.join("mail.sqlite3"),fs::Permissions::from_mode(0o600))?; }
        if !dir.join("vault.key").exists() {
            let count:i64=db.query_row("SELECT count(*) FROM accounts",[],|r|r.get(0))?;
            if count>0 {bail!("Credential key is missing; restore the original key")}
        }
        let key=load_key(dir)?; Ok(Self{db,key})
    }
    pub fn accounts(&self)->Result<Vec<Account>>{
        let mut s=self.db.prepare(&format!("SELECT {ACCOUNT_COLUMNS} FROM accounts ORDER BY name,id"))?;
        let rows=s.query_map([],account_row)?.collect::<rusqlite::Result<Vec<_>>>()?; Ok(rows)
    }
    fn account(&self,id:&str)->Result<Account>{
        Ok(self.db.query_row(&format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id=?"),[id],account_row)?)
    }
    fn password(&self,id:&str)->Result<String>{
        let bytes:Vec<u8>=self.db.query_row("SELECT secret FROM accounts WHERE id=?",[id],|r|r.get(0))?;
        decrypt(&self.key,id,&bytes)
    }
    pub fn add_account(&self,input:NewAccount)->Result<Account>{
        let email=input.email.trim().to_string();
        let _:Mailbox=email.parse().map_err(|_|anyhow!("Invalid email address"))?;
        if !valid_host(&input.imap_host)||!valid_host(&input.smtp_host)||input.imap_port==0||input.smtp_port==0 {bail!("Invalid mail server host or port")}
        if input.password.is_empty() {bail!("Password or authorization code is required")}
        if input.username.contains(['\r','\n'])||input.password.contains(['\r','\n']) {bail!("Invalid credential characters")}
        let name=if input.name.trim().is_empty(){email.clone()}else{input.name.trim().to_string()};
        let username=if input.username.trim().is_empty(){email.clone()}else{input.username.trim().to_string()};
        let a=Account{id:Uuid::new_v4().to_string(),name,email,username,imap_host:input.imap_host,imap_port:input.imap_port,smtp_host:input.smtp_host,smtp_port:input.smtp_port,smtp_starttls:input.smtp_starttls,color:input.color};
        let secret=encrypt(&self.key,&a.id,&input.password)?;
        self.db.execute("INSERT INTO accounts(id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color,secret) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![a.id,a.name,a.email,a.username,a.imap_host,a.imap_port,a.smtp_host,a.smtp_port,a.smtp_starttls,a.color,secret])?;
        Ok(a)
    }
    pub fn remove_account(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM accounts WHERE id=?",[id])?;Ok(())}
    pub fn test_account(&self,id:&str)->Result<String>{
        let a=self.account(id)?;let pw=self.password(id)?;
        let mut session=connect_imap(&a,&pw)?;
        session.examine("INBOX").context("Cannot access INBOX")?;
        let _=session.logout();
        let transport=smtp(&a,&pw)?;
        if !transport.test_connection().map_err(|_|anyhow!("SMTP connection/authentication failed"))? {bail!("SMTP server rejected connection")}
        Ok("IMAP and SMTP connection checks succeeded.".into())
    }
    pub fn sync_account(&mut self,id:&str)->Result<usize>{
        let a=self.account(id)?;let pw=self.password(id)?;
        let mut session=connect_imap(&a,&pw)?;
        let mailbox=session.examine("INBOX").context("Cannot open INBOX")?;
        let validity=mailbox.uid_validity.ok_or_else(||anyhow!("Server did not provide UIDVALIDITY"))?;
        let all=session.uid_search("ALL").context("Cannot list inbox UIDs")?;
        let unseen=session.uid_search("UNSEEN").context("Cannot list unread UIDs")?;
        let tx=self.db.transaction()?;
        let existing:HashSet<u32>={
            let mut s=tx.prepare("SELECT uid FROM messages WHERE account_id=? AND folder='INBOX' AND uid_validity=?")?;
            let rows=s.query_map(params![id,validity],|r|r.get::<_,u32>(0))?.collect::<rusqlite::Result<HashSet<_>>>()?; rows
        };
        let mut missing:Vec<u32>=all.difference(&existing).copied().collect();missing.sort_unstable_by(|a,b|b.cmp(a));
        let mut added=0;
        for batch in missing.chunks(20){
            let set=batch.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
            let fetched=session.uid_fetch(set,"(UID BODY.PEEK[])").context("Cannot fetch inbox messages")?;
            let mut received=HashSet::new();
            for item in fetched.iter(){
                let Some(uid)=item.uid else{continue};
                if !batch.contains(&uid)||!received.insert(uid){continue}
                let raw=item.body().ok_or_else(||anyhow!("Server omitted message content"))?;
                let parsed=parse(raw);
                tx.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw) VALUES(?,'INBOX',?,?,?,?,?,?,?,?,?,?)",
                    params![id,validity,uid,parsed.sender,parsed.recipients,parsed.subject,parsed.timestamp,!unseen.contains(&uid),parsed.body,parsed.html_only,raw])?;
                added+=1;
            }
            if received.len()!=batch.len(){bail!("Mailbox changed during sync; retry to reconcile")}
        }
        for uid in existing.intersection(&all){
            tx.execute("UPDATE messages SET seen=? WHERE account_id=? AND folder='INBOX' AND uid_validity=? AND uid=? AND seen<>?",params![!unseen.contains(uid),id,validity,uid,!unseen.contains(uid)])?;
        }
        for uid in existing.difference(&all){
            tx.execute("DELETE FROM messages WHERE account_id=? AND folder='INBOX' AND uid_validity=? AND uid=?",params![id,validity,uid])?;
        }
        tx.execute("DELETE FROM messages WHERE account_id=? AND folder='INBOX' AND uid_validity<>?",params![id,validity])?;
        tx.commit()?;
        let _=session.logout();
        Ok(added)
    }
    pub fn messages(&self,account_id:Option<String>,unread:bool,query:&str,offset:u32)->Result<Vec<Summary>>{
        let sql=format!("SELECT {SUMMARY_COLUMNS} FROM messages WHERE (?1 IS NULL OR account_id=?1) AND (NOT ?2 OR seen=0) AND (?3='' OR instr(lower(subject),lower(?3))>0 OR instr(lower(sender),lower(?3))>0 OR instr(lower(body),lower(?3))>0) ORDER BY timestamp DESC,id DESC LIMIT 100 OFFSET ?4");
        let mut s=self.db.prepare(&sql)?;
        let result=s.query_map(params![account_id,unread,query,offset],summary_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(result)
    }
    pub fn message(&self,id:i64)->Result<Mail>{
        Ok(self.db.query_row(&format!("SELECT {SUMMARY_COLUMNS},body,html_only,recipients FROM messages WHERE id=?"),[id],|r|Ok(Mail{summary:summary_row(r)?,body:r.get(6)?,html_only:r.get(7)?,recipients:r.get(8)?}))?)
    }
    pub fn set_seen(&self,id:i64,seen:bool)->Result<()>{
        let (aid,validity,uid):(String,u32,u32)=self.db.query_row("SELECT account_id,uid_validity,uid FROM messages WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let a=self.account(&aid)?;let pw=self.password(&aid)?;
        let mut session=connect_imap(&a,&pw)?;
        let mailbox=session.select("INBOX").context("Cannot select inbox")?;
        if mailbox.uid_validity!=Some(validity){bail!("Mailbox identity changed; sync before updating")}
        let query=if seen{"+FLAGS (\\Seen)"}else{"-FLAGS (\\Seen)"};
        let changed=session.uid_store(uid.to_string(),query).context("Cannot update read state")?;
        if !changed.iter().any(|f|f.uid==Some(uid)&&f.flags().iter().any(|flag|matches!(flag,imap::types::Flag::Seen))==seen){bail!("Server did not confirm read state; sync again")}
        self.db.execute("UPDATE messages SET seen=? WHERE id=? AND account_id=? AND uid_validity=? AND uid=?",params![seen,id,aid,validity,uid])?;
        let _=session.logout();Ok(())
    }
    pub fn send(&self,mail:Compose)->Result<()>{
        let a=self.account(&mail.account_id)?;let pw=self.password(&a.id)?;
        let mut builder=Message::builder().from(a.email.parse().map_err(|_|anyhow!("Invalid sender address"))?).subject(mail.subject);
        if mail.to.trim().is_empty(){bail!("At least one recipient is required")}
        for recipient in addresses(&mail.to)?{builder=builder.to(recipient)}
        for recipient in addresses(&mail.cc)?{builder=builder.cc(recipient)}
        for recipient in addresses(&mail.bcc)?{builder=builder.bcc(recipient)}
        let message=builder.header(ContentType::TEXT_PLAIN).body(mail.body).context("Cannot construct message")?;
        smtp(&a,&pw)?.send(&message).map_err(|_|anyhow!("SMTP send failed or delivery status is uncertain; check server before retrying"))?;
        Ok(())
    }
}

fn addresses(s:&str)->Result<Vec<Mailbox>>{
    s.split([',',';']).map(str::trim).filter(|s|!s.is_empty()).map(|s|s.parse::<Mailbox>().map_err(|_|anyhow!("Invalid recipient address"))).collect()
}
fn connect_imap(a:&Account,pw:&str)->Result<ImapSession>{
    let addresses=(a.imap_host.as_str(),a.imap_port).to_socket_addrs().context("IMAP hostname resolution failed")?;
    let mut stream=None;
    for addr in addresses { if let Ok(s)=TcpStream::connect_timeout(&addr,TIMEOUT){stream=Some(s);break} }
    let stream=stream.ok_or_else(||anyhow!("IMAP TCP connection failed"))?;
    stream.set_read_timeout(Some(TIMEOUT))?;stream.set_write_timeout(Some(TIMEOUT))?;
    let tls=TlsConnector::new()?.connect(&a.imap_host,stream).map_err(|_|anyhow!("IMAP TLS handshake or certificate validation failed"))?;
    let mut client=imap::Client::new(tls);
    client.read_greeting().context("IMAP server greeting failed")?;
    client.login(&a.username,pw).map_err(|_|anyhow!("IMAP authentication failed; check provider authorization requirements"))
}
fn smtp(a:&Account,pw:&str)->Result<SmtpTransport>{
    let builder=if a.smtp_starttls{SmtpTransport::starttls_relay(&a.smtp_host)?}else{SmtpTransport::relay(&a.smtp_host)?};
    Ok(builder.port(a.smtp_port).timeout(Some(TIMEOUT)).credentials(Credentials::new(a.username.clone(),pw.to_owned())).build())
}
#[derive(Default)]
struct Parsed { sender:String,recipients:String,subject:String,timestamp:i64,body:String,html_only:bool }
fn text_part(mail:&ParsedMail<'_>)->Option<String>{
    if mail.get_headers().get_first_value("Content-Disposition").is_some_and(|s|s.to_lowercase().starts_with("attachment")){return None}
    if mail.ctype.mimetype.eq_ignore_ascii_case("text/plain"){return mail.get_body().ok()}
    mail.subparts.iter().find_map(text_part)
}
fn has_html(mail:&ParsedMail<'_>)->bool{mail.ctype.mimetype.eq_ignore_ascii_case("text/html")||mail.subparts.iter().any(has_html)}
fn parse(raw:&[u8])->Parsed{
    let Ok(mail)=mailparse::parse_mail(raw)else{return Parsed{body:"Unable to decode this message. Original MIME cached.".into(),..Default::default()}};
    let headers=mail.get_headers();let body=text_part(&mail);
    Parsed{sender:headers.get_first_value("From").unwrap_or_default(),recipients:headers.get_first_value("To").unwrap_or_default(),subject:headers.get_first_value("Subject").unwrap_or_default(),timestamp:headers.get_first_value("Date").and_then(|s|mailparse::dateparse(&s).ok()).unwrap_or(0),html_only:body.is_none()&&has_html(&mail),body:body.unwrap_or_default()}
}

#[cfg(test)]
mod tests{
    use super::*;
    fn input()->NewAccount{NewAccount{name:"Test".into(),email:"test@example.org".into(),username:"test@example.org".into(),password:"test-only-secret".into(),imap_host:"imap.example.org".into(),imap_port:993,smtp_host:"smtp.example.org".into(),smtp_port:465,smtp_starttls:false,color:"#123456".into()}}
    #[test] fn credential_roundtrip_and_tamper(){
        let key=[42u8;32];let x=encrypt(&key,"a","secret").unwrap();let y=encrypt(&key,"a","secret").unwrap();
        assert_ne!(x,y);assert_eq!(decrypt(&key,"a",&x).unwrap(),"secret");assert!(decrypt(&key,"b",&x).is_err());
        let mut bad=x;bad[15]^=1;assert!(decrypt(&key,"a",&bad).is_err());
    }
    #[test] fn account_persists_and_deletes(){
        let dir=tempfile::tempdir().unwrap();let db=Store::open(dir.path()).unwrap();let a=db.add_account(input()).unwrap();
        let encrypted:Vec<u8>=db.db.query_row("SELECT secret FROM accounts",[],|r|r.get(0)).unwrap();
        assert!(!encrypted.windows(16).any(|w|w==b"test-only-secret"));
        drop(db);let db=Store::open(dir.path()).unwrap();assert_eq!(db.accounts().unwrap().len(),1);assert_eq!(db.password(&a.id).unwrap(),"test-only-secret");
        db.remove_account(&a.id).unwrap();assert!(db.accounts().unwrap().is_empty());
    }
    #[test] fn mime_text_and_html(){
        let m=parse(b"From: Alice <a@example.org>\r\nTo: b@example.org\r\nSubject: Hello\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nWorld");
        assert_eq!(m.subject,"Hello");assert_eq!(m.body,"World");assert!(!m.html_only);
        let h=parse(b"Subject: HTML\r\nContent-Type: text/html\r\n\r\n<script>alert(1)</script>");
        assert!(h.html_only);assert!(h.body.is_empty());
    }
}
