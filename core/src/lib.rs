use aes_gcm::{aead::{Aead, KeyInit, OsRng, Payload, rand_core::RngCore}, Aes256Gcm, Nonce};
use anyhow::{anyhow, bail, Context, Result};
use lettre::{message::Mailbox, transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use mailparse::{MailHeaderMap, ParsedMail};
use native_tls::{TlsConnector, TlsStream};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::{self, OpenOptions}, io::Write, net::{TcpStream, ToSocketAddrs}, path::Path, time::Duration};
use uuid::Uuid;
mod mime;
mod oauth;
pub use oauth::OAuthRequest;
pub use mime::{AttachmentInfo, AttachmentPreview, OutgoingAttachment};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String, pub name: String, pub email: String, pub username: String,
    pub imap_host: String, pub imap_port: u16, pub smtp_host: String, pub smtp_port: u16,
    pub smtp_starttls: bool, pub color: String, pub signature: String, pub auth_kind: String,
}
#[derive(Deserialize)]
pub struct NewAccount {
    pub name: String, pub email: String, pub username: String, pub password: String,
    pub imap_host: String, pub imap_port: u16, pub smtp_host: String, pub smtp_port: u16,
    pub smtp_starttls: bool, pub color: String, #[serde(default)] pub signature: String,
}
#[derive(Clone, Serialize)]
pub struct Summary {
    pub id: i64, pub account_id: String, pub sender: String, pub subject: String, pub timestamp: i64, pub seen: bool,
    pub folder: String, pub starred: bool,
}
#[derive(Serialize)]
pub struct Mail {
    #[serde(flatten)] pub summary: Summary, pub body: String, pub html_only: bool, pub recipients: String,
    pub html: Option<String>, pub attachments: Vec<AttachmentInfo>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Compose {
    pub account_id: String, pub to: String, pub cc: String, pub bcc: String, pub subject: String, pub body: String,
    #[serde(default)] pub in_reply_to: Option<String>, #[serde(default)] pub references: Option<String>,
    #[serde(default)] pub html: Option<String>, #[serde(default)] pub attachments: Vec<OutgoingAttachment>,
    #[serde(default)] pub save_sent_copy: bool,
}

#[derive(Clone, Serialize)]
pub struct Folder {
    pub account_id: String, pub name: String, pub label: String, pub kind: String,
    pub available: bool, pub total: i64, pub unread: i64,
}
#[derive(Serialize)]
pub struct Draft {
    pub id: String, pub updated_at: i64, pub mail: Compose,
}
#[derive(Default, Serialize)]
pub struct SyncReport { pub added: usize, pub folders: usize }
#[derive(Serialize)]
pub struct SendReport { pub sent_copy: String }
const ACCOUNT_COLUMNS: &str = "id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color,signature,auth_kind";
#[derive(Default,Clone,Deserialize)]
#[serde(default)]
pub struct SearchFilters { pub sender:String, pub recipient:String, pub subject:String, pub since:Option<i64>,pub until:Option<i64> }
#[derive(Serialize)]
pub struct BatchFailure{pub id:i64,pub reason:String}
#[derive(Default,Serialize)]
pub struct BatchReport{pub succeeded:Vec<i64>,pub failed:Vec<BatchFailure>}
const SUMMARY_COLUMNS: &str = "id,account_id,sender,subject,timestamp,seen,folder,starred";
const TIMEOUT: Duration = Duration::from_secs(45);
type ImapSession = imap::Session<TlsStream<TcpStream>>;

fn account_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account { id:r.get(0)?, name:r.get(1)?, email:r.get(2)?, username:r.get(3)?,
        imap_host:r.get(4)?, imap_port:r.get(5)?, smtp_host:r.get(6)?, smtp_port:r.get(7)?, smtp_starttls:r.get(8)?, color:r.get(9)?, signature:r.get(10)?, auth_kind:r.get(11)? })
}
fn summary_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Summary> {
    Ok(Summary { id:r.get(0)?, account_id:r.get(1)?, sender:r.get(2)?, subject:r.get(3)?, timestamp:r.get(4)?, seen:r.get(5)?, folder:r.get(6)?, starred:r.get(7)? })
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
        let mut db=Connection::open(dir.join("mail.sqlite3"))?;
        db.busy_timeout(Duration::from_secs(15))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let initial_version:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0))?;
        if initial_version>5 { bail!("Database belongs to a newer Mail EX version") }
        if initial_version<5 {
        let migration=db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let version:i64=migration.query_row("PRAGMA user_version",[],|r|r.get(0))?;
        if version>5 { bail!("Database belongs to a newer Mail EX version") }
        if version==0 {
            migration.execute_batch("CREATE TABLE IF NOT EXISTS accounts(id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, username TEXT NOT NULL, imap_host TEXT NOT NULL, imap_port INTEGER NOT NULL, smtp_host TEXT NOT NULL, smtp_port INTEGER NOT NULL, smtp_starttls INTEGER NOT NULL, color TEXT NOT NULL, secret BLOB NOT NULL);
              CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE, folder TEXT NOT NULL, uid_validity INTEGER NOT NULL, uid INTEGER NOT NULL, sender TEXT NOT NULL, recipients TEXT NOT NULL, subject TEXT NOT NULL, timestamp INTEGER NOT NULL, seen INTEGER NOT NULL, body TEXT NOT NULL, html_only INTEGER NOT NULL, raw BLOB NOT NULL, UNIQUE(account_id,folder,uid_validity,uid));
              CREATE INDEX IF NOT EXISTS messages_order ON messages(timestamp DESC,id DESC);
              CREATE INDEX IF NOT EXISTS messages_account ON messages(account_id,folder,uid_validity,uid);
              PRAGMA user_version=1;")?;
        }
        if version<2 {
            // The initial application setup opens the store before IPC is registered.
            migration.execute_batch("ALTER TABLE messages ADD COLUMN starred INTEGER NOT NULL DEFAULT 0;
              CREATE TABLE folders(account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE, name TEXT NOT NULL, label TEXT NOT NULL, kind TEXT NOT NULL, available INTEGER NOT NULL DEFAULT 1, PRIMARY KEY(account_id,name));
              INSERT INTO folders(account_id,name,label,kind) SELECT id,'INBOX','INBOX','inbox' FROM accounts;
              CREATE INDEX messages_folder_flags ON messages(account_id,folder,starred,seen);
              CREATE TABLE drafts(id TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE, recipients TEXT NOT NULL, cc TEXT NOT NULL, bcc TEXT NOT NULL, subject TEXT NOT NULL, body TEXT NOT NULL, updated_at INTEGER NOT NULL, in_reply_to TEXT, refs TEXT);
              PRAGMA user_version=2;")?;
        }
        if version<3 {
            migration.execute_batch("ALTER TABLE accounts ADD COLUMN signature TEXT NOT NULL DEFAULT '';
              ALTER TABLE drafts ADD COLUMN html TEXT;
              ALTER TABLE drafts ADD COLUMN attachments TEXT NOT NULL DEFAULT '[]';
              ALTER TABLE drafts ADD COLUMN save_sent_copy INTEGER NOT NULL DEFAULT 0;
              PRAGMA user_version=3;")?;
        }
        if version<4 {
            migration.execute_batch("ALTER TABLE accounts ADD COLUMN auth_kind TEXT NOT NULL DEFAULT 'password'; PRAGMA user_version=4;")?;
        }
        if version<5 {
            migration.execute_batch("CREATE VIRTUAL TABLE message_search USING fts5(subject,sender,recipients,body,content='messages',content_rowid='id',tokenize='trigram');
              INSERT INTO message_search(message_search) VALUES('rebuild');
              CREATE TRIGGER message_search_insert AFTER INSERT ON messages BEGIN
                INSERT INTO message_search(rowid,subject,sender,recipients,body) VALUES(new.id,new.subject,new.sender,new.recipients,new.body);
              END;
              CREATE TRIGGER message_search_delete AFTER DELETE ON messages BEGIN
                INSERT INTO message_search(message_search,rowid,subject,sender,recipients,body) VALUES('delete',old.id,old.subject,old.sender,old.recipients,old.body);
              END;
              CREATE TRIGGER message_search_update AFTER UPDATE OF subject,sender,recipients,body ON messages BEGIN
                INSERT INTO message_search(message_search,rowid,subject,sender,recipients,body) VALUES('delete',old.id,old.subject,old.sender,old.recipients,old.body);
                INSERT INTO message_search(rowid,subject,sender,recipients,body) VALUES(new.id,new.subject,new.sender,new.recipients,new.body);
              END;
              PRAGMA user_version=5;")?;
            // Index text fallback for already-cached HTML-only messages.
            let updates={
                let mut q=migration.prepare("SELECT id,raw FROM messages WHERE html_only=1 AND body=''")?;
                let rows=q.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,Vec<u8>>(1)?)))?;
                let mut updates=Vec::new();
                for row in rows {let (id,raw)=row?;if let Ok(mail)=mailparse::parse_mail(&raw){if let Some(html)=mime::body_part(&mail,"text/html"){updates.push((id,mime::plain_from_html(&html)))}}}
                updates
            };
            for (id,body) in updates{migration.execute("UPDATE messages SET body=? WHERE id=?",params![body,id])?;}
        }
        migration.commit()?;
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
    fn credential(&self,a:&Account)->Result<String>{
        if a.auth_kind=="password"{return self.password(&a.id)}
        let provider=oauth::Provider::parse(&a.auth_kind)?;
        let (imap,smtp,port,starttls)=provider.servers();
        if a.imap_host!=imap||a.imap_port!=993||a.smtp_host!=smtp||a.smtp_port!=port||a.smtp_starttls!=starttls||a.username!=a.email{bail!("OAuth account endpoint does not match its provider")}
        // Serialize refreshes across store connections in the current process.
        static REFRESH:std::sync::Mutex<()>=std::sync::Mutex::new(());
        let _guard=REFRESH.lock().map_err(|_|anyhow!("OAuth refresh lock unavailable"))?;
        let current:Vec<u8>=self.db.query_row("SELECT secret FROM accounts WHERE id=? AND auth_kind=?",params![a.id,a.auth_kind],|r|r.get(0))?;
        let secret=decrypt(&self.key,&a.id,&current)?;
        let mut tokens:oauth::Tokens=serde_json::from_str(&secret).map_err(|_|anyhow!("OAuth credential is invalid; reauthorize the account"))?;
        if tokens.expiring(){
            oauth::refresh(provider,&mut tokens)?;
            let updated=encrypt(&self.key,&a.id,&serde_json::to_string(&tokens)?)?;
            let changed=self.db.execute("UPDATE accounts SET secret=? WHERE id=? AND auth_kind=? AND secret=?",params![updated,a.id,a.auth_kind,current])?;
            if changed!=1{bail!("OAuth account changed during refresh; try again")}
        }
        Ok(tokens.access_token)
    }
    pub fn authorize_account(&mut self,request:OAuthRequest,cancel:&std::sync::atomic::AtomicBool,open_browser:impl FnOnce(&str)->Result<()>)->Result<Account>{
        let provider=oauth::Provider::parse(&request.provider)?;
        let email=request.email.trim().to_owned();
        let _:Mailbox=email.parse().map_err(|_|anyhow!("Invalid email address"))?;
        if email.chars().any(char::is_control){bail!("Invalid email address")}
        let (imap_host,smtp_host,smtp_port,smtp_starttls)=provider.servers();
        let existing=match &request.account_id{
            Some(id)=>{
                let a=self.account(id)?;
                if a.auth_kind!=provider.id()||a.email!=email{bail!("Reauthorization must use the same provider and mailbox")}
                Some(a)
            },None=>None
        };
        let tokens=oauth::authorize(&request,cancel,open_browser)?;
        let a=existing.unwrap_or_else(||Account{
            id:Uuid::new_v4().to_string(),name:if request.name.trim().is_empty(){email.clone()}else{request.name.trim().to_owned()},
            email:email.clone(),username:email.clone(),imap_host:imap_host.into(),imap_port:993,
            smtp_host:smtp_host.into(),smtp_port,smtp_starttls,color:if request.color.is_empty(){"#507ed8".into()}else{request.color.clone()},signature:String::new(),auth_kind:provider.id().into()
        });
        // Verify mailbox access before replacing existing credentials or creating an account.
        let mut session=connect_imap(&a,&tokens.access_token)?;
        session.examine("INBOX").context("OAuth authorized but IMAP inbox is not accessible")?;
        let _=session.logout();
        if cancel.load(std::sync::atomic::Ordering::Relaxed){bail!("OAuth authorization canceled")}
        let secret=encrypt(&self.key,&a.id,&serde_json::to_string(&tokens)?)?;
        let tx=self.db.transaction()?;
        if request.account_id.is_some(){
            let changed=tx.execute("UPDATE accounts SET secret=? WHERE id=? AND auth_kind=?",params![secret,a.id,a.auth_kind])?;
            if changed!=1{bail!("Account was removed during authorization")}
        }else{
            tx.execute("INSERT INTO accounts(id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color,secret,signature,auth_kind) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",params![a.id,a.name,a.email,a.username,a.imap_host,a.imap_port,a.smtp_host,a.smtp_port,a.smtp_starttls,a.color,secret,a.signature,a.auth_kind])?;
            tx.execute("INSERT INTO folders(account_id,name,label,kind) VALUES(?,'INBOX','INBOX','inbox')",[&a.id])?;
        }
        tx.commit()?;Ok(a)
    }
    pub fn add_account(&self,input:NewAccount)->Result<Account>{
        let email=input.email.trim().to_string();
        let _:Mailbox=email.parse().map_err(|_|anyhow!("Invalid email address"))?;
        if !valid_host(&input.imap_host)||!valid_host(&input.smtp_host)||input.imap_port==0||input.smtp_port==0 {bail!("Invalid mail server host or port")}
        if input.password.is_empty() {bail!("Password or authorization code is required")}
        if input.username.contains(['\r','\n'])||input.password.contains(['\r','\n']) {bail!("Invalid credential characters")}
        let name=if input.name.trim().is_empty(){email.clone()}else{input.name.trim().to_string()};
        let username=if input.username.trim().is_empty(){email.clone()}else{input.username.trim().to_string()};
        let a=Account{id:Uuid::new_v4().to_string(),name,email,username,imap_host:input.imap_host,imap_port:input.imap_port,smtp_host:input.smtp_host,smtp_port:input.smtp_port,smtp_starttls:input.smtp_starttls,color:input.color,signature:input.signature,auth_kind:"password".into()};
        let secret=encrypt(&self.key,&a.id,&input.password)?;
        self.db.execute("INSERT INTO accounts(id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color,secret,signature) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![a.id,a.name,a.email,a.username,a.imap_host,a.imap_port,a.smtp_host,a.smtp_port,a.smtp_starttls,a.color,secret,a.signature])?;
        self.db.execute("INSERT OR IGNORE INTO folders(account_id,name,label,kind) VALUES(?,'INBOX','INBOX','inbox')",[&a.id])?;
        Ok(a)
    }
    pub fn update_account(&mut self,id:&str,input:NewAccount)->Result<Account>{
        let old=self.account(id)?;
        // Changing identity or incoming endpoint would make the previous cache ambiguous.
        let username=if input.username.trim().is_empty(){input.email.trim().to_string()}else{input.username.trim().to_string()};
        if old.email!=input.email.trim() || old.username!=username || old.imap_host!=input.imap_host || old.imap_port!=input.imap_port {
            bail!("Incoming mailbox identity cannot be changed; add another account instead")
        }
        if !valid_host(&input.smtp_host)||input.smtp_port==0 {bail!("Invalid SMTP host or port")}
        if input.password.contains(['\r','\n']) {bail!("Invalid credential characters")}
        let name=if input.name.trim().is_empty(){old.email.clone()}else{input.name.trim().to_string()};
        if old.auth_kind!="password" && (!input.password.is_empty()||input.smtp_host!=old.smtp_host||input.smtp_port!=old.smtp_port||input.smtp_starttls!=old.smtp_starttls){
            bail!("OAuth credentials and server endpoints cannot be changed here; use Reauthorize")
        }
        let secret=if input.password.is_empty(){None}else{Some(encrypt(&self.key,id,&input.password)?)};
        self.db.execute("UPDATE accounts SET name=?,smtp_host=?,smtp_port=?,smtp_starttls=?,color=?,secret=coalesce(?,secret),signature=? WHERE id=?",params![name,input.smtp_host,input.smtp_port,input.smtp_starttls,input.color,secret,input.signature,id])?;
        self.account(id)
    }
    pub fn folders(&self)->Result<Vec<Folder>>{
        let mut q=self.db.prepare("SELECT f.account_id,f.name,f.label,f.kind,f.available,count(m.id),coalesce(sum(CASE WHEN m.seen=0 THEN 1 ELSE 0 END),0) FROM folders f LEFT JOIN messages m ON m.account_id=f.account_id AND m.folder=f.name GROUP BY f.account_id,f.name ORDER BY f.account_id,CASE f.kind WHEN 'inbox' THEN 0 WHEN 'sent' THEN 1 WHEN 'drafts' THEN 2 WHEN 'archive' THEN 3 WHEN 'trash' THEN 4 WHEN 'junk' THEN 5 ELSE 6 END,f.label")?;
        let rows=q.query_map([],|r|Ok(Folder{account_id:r.get(0)?,name:r.get(1)?,label:r.get(2)?,kind:r.get(3)?,available:r.get(4)?,total:r.get(5)?,unread:r.get(6)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn remove_account(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM accounts WHERE id=?",[id])?;Ok(())}
    pub fn test_account(&self,id:&str)->Result<String>{
        let a=self.account(id)?;let pw=self.credential(&a)?;
        let mut session=connect_imap(&a,&pw)?;
        session.examine("INBOX").context("Cannot access INBOX")?;
        let _=session.logout();
        let transport=smtp(&a,&pw)?;
        if !transport.test_connection().map_err(|_|anyhow!("SMTP connection/authentication failed"))? {bail!("SMTP server rejected connection")}
        Ok("IMAP and SMTP connection checks succeeded.".into())
    }
    pub fn sync_account(&mut self,id:&str,folder:Option<String>,all_folders:bool)->Result<SyncReport>{
        let a=self.account(id)?;let pw=self.credential(&a)?;
        let mut session=connect_imap(&a,&pw)?;
        let listed=discover(&mut session)?;
        {
            let tx=self.db.transaction()?;
            tx.execute("UPDATE folders SET available=0 WHERE account_id=?",[id])?;
            for (name,label,kind) in &listed {
                tx.execute("INSERT INTO folders(account_id,name,label,kind,available) VALUES(?,?,?,?,1) ON CONFLICT(account_id,name) DO UPDATE SET label=excluded.label,kind=excluded.kind,available=1",params![id,name,label,kind])?;
            }
            tx.commit()?;
        }
        let targets:Vec<String>=if all_folders{listed.iter().map(|f|f.0.clone()).collect()}else{vec![folder.unwrap_or_else(||"INBOX".into())]};
        let mut report=SyncReport::default();
        for folder in targets{
            if !listed.iter().any(|f|f.0==folder){bail!("Selected folder is unavailable on the server; cached mail was retained")}
            report.added+=self.sync_folder(&mut session,id,&folder)?;
            report.folders+=1;
        }
        let _=session.logout();
        Ok(report)
    }
    fn sync_folder(&mut self,session:&mut ImapSession,id:&str,folder:&str)->Result<usize>{
        let mailbox=session.examine(folder).context("Cannot open mailbox")?;
        let validity=mailbox.uid_validity.ok_or_else(||anyhow!("Server did not provide UIDVALIDITY"))?;
        let all=session.uid_search("UNDELETED").context("Cannot list mailbox UIDs")?;
        let unseen=session.uid_search("UNSEEN UNDELETED").context("Cannot list unread UIDs")?;
        let flagged=session.uid_search("FLAGGED UNDELETED").context("Cannot list starred UIDs")?;
        let tx=self.db.transaction()?;
        let existing:HashSet<u32>={
            let mut s=tx.prepare("SELECT uid FROM messages WHERE account_id=? AND folder=? AND uid_validity=?")?;
            let rows=s.query_map(params![id,folder,validity],|r|r.get::<_,u32>(0))?.collect::<rusqlite::Result<HashSet<_>>>()?; rows
        };
        let mut missing:Vec<u32>=all.difference(&existing).copied().collect();missing.sort_unstable_by(|a,b|b.cmp(a));
        let mut added=0;
        for batch in missing.chunks(20){
            let set=batch.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
            let fetched=session.uid_fetch(set,"(UID BODY.PEEK[])").context("Cannot fetch messages")?;
            let mut received=HashSet::new();
            for item in fetched.iter(){
                let Some(uid)=item.uid else{continue};
                if !batch.contains(&uid)||!received.insert(uid){continue}
                let raw=item.body().ok_or_else(||anyhow!("Server omitted message content"))?;
                let parsed=parse(raw);
                tx.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
                    params![id,folder,validity,uid,parsed.sender,parsed.recipients,parsed.subject,parsed.timestamp,!unseen.contains(&uid),parsed.body,parsed.html_only,raw,flagged.contains(&uid)])?;
                added+=1;
            }
            if received.len()!=batch.len(){bail!("Mailbox changed during sync; retry to reconcile")}
        }
        for uid in existing.intersection(&all){
            let seen=!unseen.contains(uid);let starred=flagged.contains(uid);
            tx.execute("UPDATE messages SET seen=?,starred=? WHERE account_id=? AND folder=? AND uid_validity=? AND uid=? AND (seen<>? OR starred<>?)",params![seen,starred,id,folder,validity,uid,seen,starred])?;
        }
        for uid in existing.difference(&all){
            tx.execute("DELETE FROM messages WHERE account_id=? AND folder=? AND uid_validity=? AND uid=?",params![id,folder,validity,uid])?;
        }
        tx.execute("DELETE FROM messages WHERE account_id=? AND folder=? AND uid_validity<>?",params![id,folder,validity])?;
        tx.commit()?;
        Ok(added)
    }
    pub fn messages(&self,account_id:Option<String>,folder:Option<String>,unread:bool,starred:bool,query:&str,offset:u32)->Result<Vec<Summary>>{
        self.search_messages(account_id,folder,unread,starred,query,offset,SearchFilters::default())
    }
    pub fn search_messages(&self,account_id:Option<String>,folder:Option<String>,unread:bool,starred:bool,query:&str,offset:u32,filters:SearchFilters)->Result<Vec<Summary>>{
        let query=query.trim();
        // A quoted FTS phrase treats user input literally, including FTS operators.
        // Trigram supports substring searches in languages without word boundaries.
        let indexed=query.chars().count()>=3&&!query.contains('\0');
        let phrase=format!("\"{}\"",query.replace('"',"\"\""));
        let fulltext=if indexed{" AND id IN (SELECT rowid FROM message_search WHERE message_search MATCH ?5)"}else{" AND (?5='' OR instr(lower(subject),lower(?5))>0 OR instr(lower(sender),lower(?5))>0 OR instr(lower(recipients),lower(?5))>0 OR instr(lower(body),lower(?5))>0)"};
        let sql=format!("SELECT {SUMMARY_COLUMNS} FROM messages WHERE (?1 IS NULL OR account_id=?1) AND (?2 IS NULL OR folder=?2) AND (NOT ?3 OR seen=0) AND (NOT ?4 OR starred=1){fulltext} AND (?7='' OR instr(lower(sender),lower(?7))>0) AND (?8='' OR instr(lower(recipients),lower(?8))>0) AND (?9='' OR instr(lower(subject),lower(?9))>0) AND (?10 IS NULL OR timestamp>=?10) AND (?11 IS NULL OR timestamp<?11) ORDER BY timestamp DESC,id DESC LIMIT 100 OFFSET ?6");
        let mut q=self.db.prepare(&sql)?;
        let result=q.query_map(params![account_id,folder,unread,starred,if indexed{phrase.as_str()}else{query},offset,filters.sender.trim(),filters.recipient.trim(),filters.subject.trim(),filters.since,filters.until],summary_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(result)
    }
    pub fn batch(&self,ids:Vec<i64>,operation:&str,destination:Option<String>)->Result<BatchReport>{
        if !["read","unread","star","unstar","move","archive","trash"].contains(&operation){bail!("Unsupported batch operation")}
        let mut report=BatchReport::default();
        let mut seen=HashSet::new();
        for id in ids{
            if !seen.insert(id){continue}
            let result=(||->Result<()>{
                match operation{
                    "read"=>self.set_seen(id,true),"unread"=>self.set_seen(id,false),
                    "star"=>self.set_starred(id,true),"unstar"=>self.set_starred(id,false),
                    "move"=>{
                        let dest=destination.as_deref().ok_or_else(||anyhow!("Choose a destination folder"))?;
                        self.move_message(id,dest)
                    },
                    "archive"|"trash"=>{
                        let (account,folder,_,_)=self.message_location(id)?;
                        let mut q=self.db.prepare("SELECT name FROM folders WHERE account_id=? AND kind=? AND available=1 AND name<>?")?;
                        let rows=q.query_map(params![account,operation,folder],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
                        if rows.len()!=1{bail!("No unambiguous server-declared destination folder")}
                        self.move_message(id,&rows[0])
                    },
                    _=>unreachable!()
                }
            })();
            match result{Ok(())=>report.succeeded.push(id),Err(e)=>report.failed.push(BatchFailure{id,reason:e.to_string()})}
        }
        Ok(report)
    }
    pub fn message(&self,id:i64)->Result<Mail>{
        let (summary,mut body,html_only,recipients,raw):(Summary,String,bool,String,Vec<u8>)=self.db.query_row(&format!("SELECT {SUMMARY_COLUMNS},body,html_only,recipients,raw FROM messages WHERE id=?"),[id],|r|Ok((summary_row(r)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?)))?;
        let mut html=None;let mut attachments=Vec::new();
        if let Ok(parsed)=mailparse::parse_mail(&raw){
            html=mime::body_part(&parsed,"text/html").map(|s|mime::sanitize_html(&s));
            if body.is_empty(){if let Some(h)=&html{body=mime::plain_from_html(h)}}
            for (i,part) in mime::attachment_parts(&parsed).iter().enumerate(){
                if let Ok(info)=mime::metadata(part,i){attachments.push(info)}
            }
        }
        Ok(Mail{summary,body,html_only,recipients,html,attachments})
    }
    fn with_attachment<T>(&self,id:i64,index:usize,action:impl FnOnce(&ParsedMail<'_>)->Result<T>)->Result<T>{
        let raw:Vec<u8>=self.db.query_row("SELECT raw FROM messages WHERE id=?",[id],|r|r.get(0))?;
        let parsed=mailparse::parse_mail(&raw).context("Cannot decode original MIME")?;
        let parts=mime::attachment_parts(&parsed);
        let part=parts.get(index).ok_or_else(||anyhow!("Attachment no longer exists"))?;
        action(part)
    }
    pub fn attachment_preview(&self,id:i64,index:usize)->Result<AttachmentPreview>{
        self.with_attachment(id,index,|p|mime::preview(p,index))
    }
    pub fn attachment_download(&self,id:i64,index:usize)->Result<(String,Vec<u8>)>{
        self.with_attachment(id,index,|p|Ok((mime::attachment_name(p,index),mime::attachment_bytes(p)?)))
    }
    fn message_location(&self,id:i64)->Result<(String,String,u32,u32)>{
        Ok(self.db.query_row("SELECT account_id,folder,uid_validity,uid FROM messages WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?)
    }
    pub fn set_seen(&self,id:i64,seen:bool)->Result<()>{self.set_flag(id,"seen",seen)}
    pub fn set_starred(&self,id:i64,starred:bool)->Result<()>{self.set_flag(id,"starred",starred)}
    fn set_flag(&self,id:i64,flag:&str,value:bool)->Result<()>{
        let (aid,folder,validity,uid)=self.message_location(id)?;
        let a=self.account(&aid)?;let pw=self.credential(&a)?;
        let mut session=connect_imap(&a,&pw)?;
        let mailbox=session.select(&folder).context("Cannot select mailbox")?;
        if mailbox.uid_validity!=Some(validity){bail!("Mailbox identity changed; sync before updating")}
        let (name,variant)=match flag {"seen"=>("\\Seen",imap::types::Flag::Seen),"starred"=>("\\Flagged",imap::types::Flag::Flagged),_=>bail!("Unsupported flag")};
        let operation=if value{"+"}else{"-"};
        let query=format!("{operation}FLAGS ({name})");
        let changed=session.uid_store(uid.to_string(),query).context("Cannot update message flag")?;
        if !changed.iter().any(|f|f.uid==Some(uid)&&f.flags().iter().any(|flag|flag==&variant)==value){bail!("Server did not confirm message flag; sync again")}
        let sql=if flag=="seen"{"UPDATE messages SET seen=? WHERE id=? AND account_id=? AND folder=? AND uid_validity=? AND uid=?"}else{"UPDATE messages SET starred=? WHERE id=? AND account_id=? AND folder=? AND uid_validity=? AND uid=?"};
        self.db.execute(sql,params![value,id,aid,folder,validity,uid])?;
        let _=session.logout();Ok(())
    }
    pub fn move_message(&self,id:i64,destination:&str)->Result<()>{
        let (aid,folder,validity,uid)=self.message_location(id)?;
        if folder==destination {bail!("Source and destination folders are identical")}
        let available:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM folders WHERE account_id=? AND name=? AND available=1)",params![aid,destination],|r|r.get(0))?;
        if !available{bail!("Destination is not an available folder of this account")}
        let a=self.account(&aid)?;let pw=self.credential(&a)?;
        let mut session=connect_imap(&a,&pw)?;
        let caps=session.capabilities().context("Cannot check IMAP capabilities")?;
        if !caps.has_str("MOVE"){bail!("Server does not support UID MOVE; no message was changed")}
        drop(caps);
        let mailbox=session.select(&folder).context("Cannot select source mailbox")?;
        if mailbox.uid_validity!=Some(validity){bail!("Mailbox identity changed; sync before moving")}
        let search=format!("UID {uid}");
        if !session.uid_search(&search)?.contains(&uid){bail!("Source message no longer exists; sync again")}
        session.uid_mv(uid.to_string(),destination).map_err(|_|anyhow!("Move failed or its result is uncertain; sync source and destination before retrying"))?;
        if session.uid_search(&search).map_err(|_|anyhow!("Move may have succeeded; sync both folders before retrying"))?.contains(&uid){
            bail!("Server did not confirm source removal; sync before retrying")
        }
        self.db.execute("DELETE FROM messages WHERE id=? AND account_id=? AND folder=? AND uid_validity=? AND uid=?",params![id,aid,folder,validity,uid])?;
        let _=session.logout();Ok(())
    }
    pub fn drafts(&self)->Result<Vec<Draft>>{
        let mut q=self.db.prepare("SELECT id,updated_at,account_id,recipients,cc,bcc,subject,body,in_reply_to,refs,html,save_sent_copy FROM drafts ORDER BY updated_at DESC,id DESC")?;
        let rows=q.query_map([],|r|Ok(Draft{id:r.get(0)?,updated_at:r.get(1)?,mail:Compose{account_id:r.get(2)?,to:r.get(3)?,cc:r.get(4)?,bcc:r.get(5)?,subject:r.get(6)?,body:r.get(7)?,in_reply_to:r.get(8)?,references:r.get(9)?,html:r.get(10)?,attachments:Vec::new(),save_sent_copy:r.get(11)?}}))?.collect::<rusqlite::Result<Vec<_>>>()?;Ok(rows)
    }
    pub fn draft(&self,id:&str)->Result<Draft>{
        let (mut draft,data)=self.db.query_row("SELECT id,updated_at,account_id,recipients,cc,bcc,subject,body,in_reply_to,refs,html,save_sent_copy,attachments FROM drafts WHERE id=?",[id],|r|Ok((Draft{id:r.get(0)?,updated_at:r.get(1)?,mail:Compose{account_id:r.get(2)?,to:r.get(3)?,cc:r.get(4)?,bcc:r.get(5)?,subject:r.get(6)?,body:r.get(7)?,in_reply_to:r.get(8)?,references:r.get(9)?,html:r.get(10)?,attachments:Vec::new(),save_sent_copy:r.get(11)?}},r.get::<_,String>(12)?)))?;
        draft.mail.attachments=serde_json::from_str(&data).context("Cannot decode saved draft attachments")?;
        Ok(draft)
    }
    pub fn save_draft(&self,id:Option<String>,mail:Compose)->Result<String>{
        self.account(&mail.account_id)?;
        let id=match id{Some(id)=>{Uuid::parse_str(&id).context("Invalid draft ID")?;id},None=>Uuid::new_v4().to_string()};
        let attachments=serde_json::to_string(&mail.attachments)?;
        let html=mail.html.map(|s|mime::sanitize_html(&s));
        self.db.execute("INSERT INTO drafts(id,account_id,recipients,cc,bcc,subject,body,updated_at,in_reply_to,refs,html,attachments,save_sent_copy) VALUES(?,?,?,?,?,?,?,unixepoch(),?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET account_id=excluded.account_id,recipients=excluded.recipients,cc=excluded.cc,bcc=excluded.bcc,subject=excluded.subject,body=excluded.body,updated_at=excluded.updated_at,in_reply_to=excluded.in_reply_to,refs=excluded.refs,html=excluded.html,attachments=excluded.attachments,save_sent_copy=excluded.save_sent_copy",params![id,mail.account_id,mail.to,mail.cc,mail.bcc,mail.subject,mail.body,mail.in_reply_to,mail.references,html,attachments,mail.save_sent_copy])?;
        Ok(id)
    }
    pub fn delete_draft(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM drafts WHERE id=?",[id])?;Ok(())}
    pub fn compose_from(&self,id:i64,mode:&str)->Result<Compose>{
        let m=self.message(id)?;
        let account=self.account(&m.summary.account_id)?;
        let raw:Vec<u8>=self.db.query_row("SELECT raw FROM messages WHERE id=?",[id],|r|r.get(0))?;
        let parsed=mailparse::parse_mail(&raw).context("Cannot read original message headers")?;
        let h=parsed.get_headers();
        let forward=mode=="forward";
        if !forward&&mode!="reply"&&mode!="reply_all"{bail!("Unknown compose mode")}
        let from=h.get_first_value("Reply-To").or_else(||h.get_first_value("From")).unwrap_or_default();
        let to=if forward{String::new()}else{mail_addresses(&from)?.join(", ")};
        let cc=if mode=="reply_all"{
            let mut excluded:HashSet<String>=mail_addresses(&to)?.iter().map(|s|s.to_ascii_lowercase()).collect();
            excluded.insert(account.email.to_ascii_lowercase());
            let mut cc=Vec::new();
            for header in ["To","Cc"]{
                for addr in mail_addresses(&h.get_first_value(header).unwrap_or_default())?{
                    if excluded.insert(addr.to_ascii_lowercase()){cc.push(addr)}
                }
            }
            cc.join(", ")
        }else{String::new()};
        let prefix=if forward{"Fwd:"}else{"Re:"};
        let subject=if m.summary.subject.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()){m.summary.subject.clone()}else{format!("{prefix} {}",m.summary.subject)};
        let quoted=if forward{format!("\n\n---------- Forwarded message ----------\nFrom: {}\nTo: {}\nSubject: {}\n\n{}",m.summary.sender,m.recipients,m.summary.subject,m.body)}else{format!("\n\n{} wrote:\n{}",m.summary.sender,m.body.lines().map(|s|format!("> {s}")).collect::<Vec<_>>().join("\n"))};
        let message_id=if forward{None}else{h.get_first_value("Message-ID").filter(|s|valid_message_ids(s))};
        let references=message_id.as_ref().map(|mid|{
            let previous=h.get_first_value("References").filter(|s|valid_message_ids(s)).or_else(||h.get_first_value("In-Reply-To").filter(|s|valid_message_ids(s))).unwrap_or_default();
            if previous.is_empty(){mid.clone()}else{format!("{previous} {mid}")}
        });
        let attachments=if forward{mime::attachment_parts(&parsed).iter().enumerate().map(|(i,p)|mime::outgoing(p,i)).collect::<Result<Vec<_>>>()?}else{Vec::new()};
        Ok(Compose{account_id:account.id,to,cc,bcc:String::new(),subject,body:quoted,in_reply_to:message_id,references,html:None,attachments,save_sent_copy:false})
    }
    pub fn send(&self,mail:Compose)->Result<SendReport>{
        let a=self.account(&mail.account_id)?;let pw=self.credential(&a)?;
        let mut builder=Message::builder().from(a.email.parse().map_err(|_|anyhow!("Invalid sender address"))?).subject(mail.subject);
        let recipients=addresses(&mail.to)?;
        if recipients.is_empty(){bail!("At least one recipient is required")}
        if let Some(v)=mail.in_reply_to {if valid_message_ids(&v){builder=builder.in_reply_to(v)}}
        if let Some(v)=mail.references {if valid_message_ids(&v){builder=builder.references(v)}}
        for recipient in recipients{builder=builder.to(recipient)}
        for recipient in addresses(&mail.cc)?{builder=builder.cc(recipient)}
        for recipient in addresses(&mail.bcc)?{builder=builder.bcc(recipient)}
        let message=mime::build_message(builder,mail.body,mail.html,mail.attachments).context("Cannot construct message")?;
        smtp(&a,&pw)?.send(&message).map_err(|_|anyhow!("SMTP send failed or delivery status is uncertain; check server before retrying"))?;
        // SMTP accepted is authoritative even if the optional IMAP sent-copy fails.
        let sent_copy=if mail.save_sent_copy {
            match self.append_sent(&a,&pw,&message){Ok(())=>"saved",Err(_)=>"failed_or_uncertain"}
        }else{"not_requested"};
        Ok(SendReport{sent_copy:sent_copy.into()})
    }
    fn append_sent(&self,a:&Account,pw:&str,message:&Message)->Result<()>{
        let mut session=connect_imap(a,pw)?;
        let folders=discover(&mut session)?;
        let sent=folders.iter().find(|f|f.2=="sent").ok_or_else(||anyhow!("Server does not declare a Sent folder"))?;
        // imap 2.4.1 APPEND does not escape mailbox names internally.
        if sent.0.contains(['"','\\','\r','\n']){bail!("Unsupported Sent mailbox wire name")}
        session.append_with_flags(&sent.0,message.formatted(),&[imap::types::Flag::Seen]).context("Sent copy append failed")?;
        let _=session.logout();Ok(())
    }
}

fn discover(session:&mut ImapSession)->Result<Vec<(String,String,String)>>{
    use imap::types::NameAttribute;
    let names=session.list(Some(""),Some("*")).context("Cannot discover mailbox folders")?;
    let mut folders=Vec::new();
    let mut seen=HashSet::new();
    for entry in names.iter(){
        if entry.attributes().iter().any(|a|matches!(a,NameAttribute::NoSelect)||matches!(a,NameAttribute::Custom(s) if s.eq_ignore_ascii_case("\\Noselect")||s.eq_ignore_ascii_case("\\NonExistent"))){continue}
        let name=if entry.name().eq_ignore_ascii_case("INBOX"){"INBOX".to_string()}else{entry.name().to_string()};
        if !seen.insert(name.clone()){continue}
        let kind=folder_kind(&name,entry.attributes());
        let label=decode_mailbox(&name);
        folders.push((name,label,kind));
    }
    if !seen.contains("INBOX"){folders.push(("INBOX".into(),"INBOX".into(),"inbox".into()))}
    folders.sort_by(|a,b| (a.0!="INBOX").cmp(&(b.0!="INBOX")).then(a.0.cmp(&b.0)));
    Ok(folders)
}
fn folder_kind(name:&str,attributes:&[imap::types::NameAttribute<'_>])->String{
    if name.eq_ignore_ascii_case("INBOX"){return "inbox".into()}
    // Only use server-advertised special-use roles for potentially destructive shortcuts.
    for a in attributes {
        if let imap::types::NameAttribute::Custom(s)=a{
            let k=match s.to_ascii_lowercase().as_str(){"\\sent"=>"sent","\\drafts"=>"drafts","\\archive"=>"archive","\\trash"=>"trash","\\junk"=>"junk","\\all"=>"all","\\flagged"=>"flagged",_=>continue};
            return k.into()
        }
    }
    "custom".into()
}
// IMAP modified UTF-7 mailbox names: keep original wire name as the ID, decode only for display.
fn decode_mailbox(name:&str)->String{
    use base64::{engine::general_purpose::STANDARD_NO_PAD,Engine};
    let mut rest=name;let mut out=String::new();
    while let Some(start)=rest.find('&'){
        out.push_str(&rest[..start]);let tail=&rest[start+1..];
        let Some(end)=tail.find('-')else{out.push_str(&rest[start..]);return out};
        let part=&tail[..end];
        if part.is_empty(){out.push('&')}else{
            let decoded=STANDARD_NO_PAD.decode(part.replace(',',"/")).ok().filter(|b|b.len()%2==0).and_then(|b|String::from_utf16(&b.chunks_exact(2).map(|c|u16::from_be_bytes([c[0],c[1]])).collect::<Vec<_>>()).ok());
            if let Some(text)=decoded{out.push_str(&text)}else{out.push('&');out.push_str(part);out.push('-')}
        }
        rest=&tail[end+1..];
    }
    out.push_str(rest);out
}
fn valid_message_ids(s:&str)->bool{
    !s.trim().is_empty()&&!s.contains(['\r','\n'])&&s.split_whitespace().all(|p|p.starts_with('<')&&p.ends_with('>')&&p.len()>2)
}
fn mail_addresses(s:&str)->Result<Vec<String>>{
    if s.trim().is_empty(){return Ok(Vec::new())}
    if s.contains(['\r','\n']){bail!("Invalid recipient header")}
    let parsed=mailparse::addrparse(s).map_err(|_|anyhow!("Invalid recipient address"))?;
    let mut out=Vec::new();
    for item in parsed.iter(){
        match item{mailparse::MailAddr::Single(x)=>out.push(x.addr.clone()),mailparse::MailAddr::Group(g)=>out.extend(g.addrs.iter().map(|a|a.addr.clone()))}
    }
    Ok(out)
}
fn addresses(s:&str)->Result<Vec<Mailbox>>{
    mail_addresses(s)?.into_iter().map(|s|s.parse::<Mailbox>().map_err(|_|anyhow!("Invalid recipient address"))).collect()
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
    if a.auth_kind=="password"{
        client.login(&a.username,pw).map_err(|_|anyhow!("IMAP authentication failed; check provider authorization requirements"))
    }else{
        client.authenticate("XOAUTH2",&oauth::Xoauth2::new(&a.username,pw)).map_err(|_|anyhow!("IMAP OAuth authentication failed; check provider policy or reauthorize"))
    }
}
fn smtp(a:&Account,pw:&str)->Result<SmtpTransport>{
    let builder=if a.smtp_starttls{SmtpTransport::starttls_relay(&a.smtp_host)?}else{SmtpTransport::relay(&a.smtp_host)?};
    let builder=builder.port(a.smtp_port).timeout(Some(TIMEOUT)).credentials(Credentials::new(a.username.clone(),pw.to_owned()));
    let builder=if a.auth_kind=="password"{builder}else{builder.authentication(vec![lettre::transport::smtp::authentication::Mechanism::Xoauth2])};
    Ok(builder.build())
}
#[derive(Default)]
struct Parsed { sender:String,recipients:String,subject:String,timestamp:i64,body:String,html_only:bool }
fn text_part(mail:&ParsedMail<'_>)->Option<String>{mime::body_part(mail,"text/plain")}
fn has_html(mail:&ParsedMail<'_>)->bool{mime::body_part(mail,"text/html").is_some()}
fn parse(raw:&[u8])->Parsed{
    let Ok(mail)=mailparse::parse_mail(raw)else{return Parsed{body:"Unable to decode this message. Original MIME cached.".into(),..Default::default()}};
    let headers=mail.get_headers();let body=text_part(&mail);let html_only=body.is_none()&&has_html(&mail);let body=body.or_else(||mime::body_part(&mail,"text/html").map(|h|mime::plain_from_html(&h)));
    Parsed{sender:headers.get_first_value("From").unwrap_or_default(),recipients:headers.get_first_value("To").unwrap_or_default(),subject:headers.get_first_value("Subject").unwrap_or_default(),timestamp:headers.get_first_value("Date").and_then(|s|mailparse::dateparse(&s).ok()).unwrap_or(0),html_only,body:body.unwrap_or_default()}
}

#[cfg(test)]
mod tests{
    use super::*;
    fn input()->NewAccount{NewAccount{name:"Test".into(),email:"test@example.org".into(),username:"test@example.org".into(),password:"test-only-secret".into(),imap_host:"imap.example.org".into(),imap_port:993,smtp_host:"smtp.example.org".into(),smtp_port:465,smtp_starttls:false,color:"#123456".into(),signature:"Signature".into()}}
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
    #[test] fn folder_views_and_drafts(){
        let dir=tempfile::tempdir().unwrap();let db=Store::open(dir.path()).unwrap();let a=db.add_account(input()).unwrap();
        db.db.execute("INSERT INTO folders(account_id,name,label,kind) VALUES(?,'Sent','Sent','sent')",[&a.id]).unwrap();
        for (folder,uid,seen,starred) in [("INBOX",1,false,true),("Sent",1,true,false)]{
            db.db.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred) VALUES(?,?,1,?,'sender','recipient','Hello',1,?,'Body',0,X'00',?)",params![a.id,folder,uid,seen,starred]).unwrap();
        }
        assert_eq!(db.folders().unwrap().len(),2);
        assert_eq!(db.messages(None,Some("INBOX".into()),false,false,"",0).unwrap().len(),1);
        assert_eq!(db.messages(None,None,false,true,"",0).unwrap().len(),1);
        assert_eq!(db.messages(None,None,false,false,"",0).unwrap().len(),2);
        let m=Compose{account_id:a.id.clone(),to:"".into(),cc:"".into(),bcc:"".into(),subject:"draft".into(),body:"first".into(),in_reply_to:None,references:None,html:None,attachments:Vec::new(),save_sent_copy:false};
        let id=db.save_draft(None,m.clone()).unwrap();let mut m=m;m.body="updated".into();db.save_draft(Some(id.clone()),m).unwrap();
        assert_eq!(db.drafts().unwrap()[0].mail.body,"updated");
        assert_eq!(db.draft(&id).unwrap().mail.body,"updated");
        assert_eq!(db.accounts().unwrap()[0].signature,"Signature");
        db.delete_draft(&id).unwrap();assert!(db.drafts().unwrap().is_empty());
        assert_eq!(decode_mailbox("&ZeVnLIqe-"),"日本語");
        assert_eq!(decode_mailbox("A&-B"),"A&B");
    }
    #[test] fn indexed_search_and_filters(){
        let dir=tempfile::tempdir().unwrap();let db=Store::open(dir.path()).unwrap();let a=db.add_account(input()).unwrap();
        for (uid,subject,sender,body) in [(1,"Quarterly report","Alice","项目开发进度"),(2,"Another subject","Bob","report body")]{
            db.db.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred) VALUES(?,'INBOX',1,?,?,'team@example.invalid',?,100,0,?,0,X'00',0)",params![a.id,uid,sender,subject,body]).unwrap();
        }
        assert_eq!(db.messages(None,None,false,false,"report",0).unwrap().len(),2);
        assert_eq!(db.messages(None,None,false,false,"开发进",0).unwrap().len(),1);
        assert_eq!(db.messages(None,None,false,false,"项目",0).unwrap().len(),1);
        assert_eq!(db.messages(None,None,false,false,"report OR Bob",0).unwrap().len(),0);
        let results=db.search_messages(None,None,false,false,"report",0,SearchFilters{sender:"alice".into(),since:Some(99),until:Some(101),..Default::default()}).unwrap();
        assert_eq!(results.len(),1);
        db.db.execute("UPDATE messages SET body='updated text' WHERE id=?",[results[0].id]).unwrap();
        assert!(db.messages(None,None,false,false,"开发进",0).unwrap().is_empty());
        assert_eq!(db.messages(None,None,false,false,"updated",0).unwrap().len(),1);
        db.remove_account(&a.id).unwrap();
        assert!(db.messages(None,None,false,false,"report",0).unwrap().is_empty());
    }
    #[test] fn mime_text_and_html(){
        let m=parse(b"From: Alice <a@example.org>\r\nTo: b@example.org\r\nSubject: Hello\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nWorld");
        assert_eq!(m.subject,"Hello");assert_eq!(m.body,"World");assert!(!m.html_only);
        let h=parse(b"Subject: HTML\r\nContent-Type: text/html\r\n\r\n<script>alert(1)</script>");
        assert!(h.html_only);assert!(h.body.is_empty());
    }
}
