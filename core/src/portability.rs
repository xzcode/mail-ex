//! M7 local data portability. No file imports perform network I/O.
use super::{Store,Account,Uuid,parse,encrypt,decrypt,valid_host};
use anyhow::{anyhow,bail,Context,Result};
use rusqlite::{params,Connection,OpenFlags,TransactionBehavior};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{fs::{self,File,OpenOptions},io::{BufRead,BufReader,BufWriter,Read,Write},path::{Path,PathBuf}};
pub const ARCHIVE_ID:&str="local-archive";
const ARCHIVE_FOLDER:&str="Imported";
const DB:&str="mail.sqlite3";
const KEY:&str="vault.key";
const MANIFEST:&str="mail-ex-backup.json";
const SCHEMA:i64=8;

pub(crate) fn migrate(db:&Connection)->Result<()>{
    db.execute_batch("ALTER TABLE messages ADD COLUMN import_hash TEXT;
      CREATE UNIQUE INDEX messages_import_hash ON messages(account_id,import_hash) WHERE import_hash IS NOT NULL;
      PRAGMA user_version=7;")?;
    Ok(())
}
#[derive(Default,Serialize)]
pub struct ImportReport{pub added:usize,pub skipped:usize,pub invalid:usize}
#[derive(Serialize)]
pub struct StorageStats{
    pub directory:String,pub database_bytes:u64,pub wal_bytes:u64,pub shm_bytes:u64,pub raw_mail_bytes:u64,
    pub messages:i64,pub local_messages:i64,pub accounts:i64,pub contacts:i64,pub drafts:i64,pub outbox:i64,
}
#[derive(Serialize)]
pub struct CacheReport{pub messages:i64,pub raw_bytes:u64}
#[derive(Serialize,Deserialize)]
struct ConfigFile{format:String,version:u32,accounts:Vec<Account>}
#[derive(Serialize,Deserialize)]
struct Manifest{format:String,version:u32,schema:i64,db_sha256:String,key_sha256:String}
#[derive(Serialize)]
pub struct SnapshotReport{pub directory:String,pub suspended_sends:usize}
fn file_size(p:&Path)->u64{fs::metadata(p).map(|m|m.len()).unwrap_or(0)}
fn hash_bytes(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn hash_file(path:&Path)->Result<String>{
    let mut f=File::open(path)?;let mut sha=Sha256::new();let mut buffer=[0u8;65536];
    loop{let n=f.read(&mut buffer)?;if n==0{break}sha.update(&buffer[..n]);}Ok(format!("{:x}",sha.finalize()))
}
fn private_file(path:&Path)->Result<File>{
    let mut options=OpenOptions::new();options.write(true).create_new(true);
    #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    Ok(options.open(path).context("Choose a new file name; existing files are not overwritten")?)
}
fn private_dir(parent:&Path,prefix:&str)->Result<PathBuf>{
    if !parent.is_dir(){bail!("Choose an existing directory")}
    let path=parent.join(format!("{prefix}-{}",Uuid::new_v4()));
    let mut options=fs::DirBuilder::new();
    #[cfg(unix)] {use std::os::unix::fs::DirBuilderExt;options.mode(0o700);}
    options.create(&path)?;Ok(path)
}
fn copy_private(src:&Path,dst:&Path)->Result<()>{
    let mut input=File::open(src)?;let mut output=private_file(dst)?;
    std::io::copy(&mut input,&mut output)?;output.sync_all()?;Ok(())
}
fn create_archive(db:&Connection)->Result<()>{
    db.execute("INSERT OR IGNORE INTO accounts(id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color,secret,signature,auth_kind) VALUES(?1,'Local archive','','','',0,'',0,0,'#64748b',X'','', 'local')",[ARCHIVE_ID])?;
    let kind:String=db.query_row("SELECT auth_kind FROM accounts WHERE id=?",[ARCHIVE_ID],|r|r.get(0))?;
    if kind!="local"{bail!("Local archive identifier is reserved")}
    db.execute("INSERT OR IGNORE INTO folders(account_id,name,label,kind,available) VALUES(?1,?2,'Imported mail','local',1)",params![ARCHIVE_ID,ARCHIVE_FOLDER])?;
    Ok(())
}
fn import_message(db:&Connection,raw:&[u8],report:&mut ImportReport)->Result<()>{
    let parsed=match mailparse::parse_mail(raw){Ok(p)=>p,Err(_)=>{report.invalid+=1;return Ok(())}};
    if parsed.headers.is_empty(){report.invalid+=1;return Ok(())}
    let hash=hash_bytes(raw);
    let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE account_id=? AND import_hash=?)",params![ARCHIVE_ID,hash],|r|r.get(0))?;
    if exists{report.skipped+=1;return Ok(())}
    let uid:i64=db.query_row("SELECT coalesce(max(uid),0)+1 FROM messages WHERE account_id=? AND folder=?",params![ARCHIVE_ID,ARCHIVE_FOLDER],|r|r.get(0))?;
    let m=parse(raw);
    db.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred,import_hash) VALUES(?,?,1,?,?,?,?,?,1,?,?,?,0,?)",params![ARCHIVE_ID,ARCHIVE_FOLDER,uid,m.sender,m.recipients,m.subject,m.timestamp,m.body,m.html_only,raw,hash])?;
    report.added+=1;Ok(())
}
// mboxrd: one message buffer, unescape one leading > before From lines.
fn from_line(line:&[u8])->bool{
    let stripped=line.iter().position(|&c|c!=b'>').map(|i|&line[i..]).unwrap_or(&[]);
    stripped.starts_with(b"From ")
}
fn finish_mbox(db:&Connection,msg:&mut Vec<u8>,report:&mut ImportReport)->Result<()>{
    // Exporter writes a single separator LF. Remove that separator, not arbitrary whitespace.
    if msg.ends_with(b"\r\n\r\n"){msg.truncate(msg.len()-2)}
    else if msg.ends_with(b"\n\n"){msg.truncate(msg.len()-1)}
    if !msg.is_empty(){import_message(db,msg,report)?;}else{report.invalid+=1}
    msg.clear();Ok(())
}
fn read_mbox(db:&Connection,reader:impl BufRead,report:&mut ImportReport)->Result<()>{
    let mut reader=reader;let mut line=Vec::new();let mut msg=Vec::new();let mut started=false;
    loop{
        line.clear();if reader.read_until(b'\n',&mut line)?==0{break}
        if line.starts_with(b"From "){
            if started{finish_mbox(db,&mut msg,report)?}
            started=true;continue
        }
        if !started{if line.iter().all(|b|b.is_ascii_whitespace()){continue}bail!("MBOX must begin with a From separator (mboxrd or conventional mbox)")}
        if line.starts_with(b">")&&from_line(&line){msg.extend_from_slice(&line[1..])}else{msg.extend_from_slice(&line)}
    }
    if started{finish_mbox(db,&mut msg,report)?;}Ok(())
}
fn write_mbox(mut out:impl Write,raw:&[u8])->Result<()>{
    out.write_all(b"From MAILER-DAEMON Thu Jan  1 00:00:00 1970\n")?;
    for line in raw.split_inclusive(|&b|b==b'\n'){
        if from_line(line){out.write_all(b">")?}out.write_all(line)?;
    }
    if !raw.ends_with(b"\n"){out.write_all(b"\n")?}out.write_all(b"\n")?;Ok(())
}
fn suspend_outbox(db:&Connection)->Result<usize>{
    Ok(db.execute("UPDATE outbox SET state='uncertain',detail='Restored snapshot: delivery disabled; verify server before any new send request',updated_at=unixepoch() WHERE state IN ('queued','sending')",[])?)
}
fn schema_signature(db:&Connection)->Result<Vec<(String,String,String)>>{
    let mut q=db.prepare("SELECT type,name,coalesce(sql,'') FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name")?;
    let rows=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;Ok(rows)
}
fn check_snapshot(db:&Connection,expected:&[(String,String,String)])->Result<()>{
    let v:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0))?;
    if v!=SCHEMA||schema_signature(db)?!=expected{bail!("Backup schema differs from this Mail EX version")}
    let result:String=db.query_row("PRAGMA quick_check",[],|r|r.get(0))?;
    if result!="ok"{bail!("Backup database integrity check failed")}
    let bad:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",[],|r|r.get(0))?;
    if bad{bail!("Backup has invalid record references")}
    Ok(())
}
fn verify_vault(db:&Connection,key:&[u8;32])->Result<()>{
    let mut q=db.prepare("SELECT id,secret FROM accounts WHERE auth_kind<>'local'")?;
    let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Vec<u8>>(1)?)))?;
    for row in rows{let (id,secret)=row?;decrypt(key,&id,&secret).context("Backup credential key does not match account data")?;}
    let mut q=db.prepare("SELECT id,secret FROM calendar_sources")?;
    let rows=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Vec<u8>>(1)?)))?;
    for row in rows{let (id,secret)=row?;decrypt(key,&id,&secret).context("Backup credential key does not match calendar data")?;}
    Ok(())
}
impl Store{
    pub fn import_mail(&mut self,paths:&[PathBuf],format:&str)->Result<ImportReport>{
        if !["eml","mbox"].contains(&format){bail!("Unsupported mail import format")}
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut report=ImportReport::default();
        if !paths.is_empty(){create_archive(&tx)?;}
        for path in paths{
            let file=File::open(path).context("Cannot read selected mail file")?;
            if format=="eml"{
                let mut raw=Vec::new();BufReader::new(file).read_to_end(&mut raw)?;
                import_message(&tx,&raw,&mut report)?;
            }else{read_mbox(&tx,BufReader::new(file),&mut report)?;}
        }
        tx.commit()?;Ok(report)
    }
    pub fn export_eml(&self,id:i64,destination:&Path)->Result<()>{
        let raw:Vec<u8>=self.db.query_row("SELECT raw FROM messages WHERE id=?",[id],|r|r.get(0))?;
        let mut file=private_file(destination)?;file.write_all(&raw)?;file.sync_all()?;Ok(())
    }
    pub fn export_mbox(&self,account_id:Option<String>,folder:Option<String>,destination:&Path)->Result<usize>{
        let mut file=BufWriter::new(private_file(destination)?);
        let mut q=self.db.prepare("SELECT raw FROM messages WHERE (?1 IS NULL OR account_id=?1) AND (?2 IS NULL OR folder=?2) ORDER BY timestamp,id")?;
        let mut rows=q.query(params![account_id,folder])?;let mut count=0;
        while let Some(row)=rows.next()?{let raw:Vec<u8>=row.get(0)?;write_mbox(&mut file,&raw)?;count+=1}
        file.flush()?;file.get_ref().sync_all()?;Ok(count)
    }
    pub fn delete_import(&self,id:i64)->Result<()>{
        if self.db.execute("DELETE FROM messages WHERE id=? AND account_id=?",params![id,ARCHIVE_ID])?!=1{bail!("Only imported local mail can be deleted this way")}Ok(())
    }
    pub fn export_contacts(&self,path:&Path)->Result<usize>{
        let mut writer=csv::Writer::from_writer(private_file(path)?);
        writer.write_record(["email","name","group","notes"])?;
        let contacts=self.contacts()?;for c in &contacts{writer.write_record([&c.email,&c.name,&c.group_name,&c.notes])?;}
        writer.flush()?;Ok(contacts.len())
    }
    pub fn import_contacts(&mut self,path:&Path)->Result<ImportReport>{
        let mut reader=csv::ReaderBuilder::new().flexible(true).from_path(path)?;
        let headers=reader.headers()?.clone();
        let email=headers.iter().position(|s|s.trim_start_matches('\u{feff}').eq_ignore_ascii_case("email")).ok_or_else(||anyhow!("CSV requires an email header"))?;
        let name=headers.iter().position(|s|s.eq_ignore_ascii_case("name"));
        let group=headers.iter().position(|s|s.eq_ignore_ascii_case("group"));
        let notes=headers.iter().position(|s|s.eq_ignore_ascii_case("notes"));
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut report=ImportReport::default();
        for row in reader.records(){
            let row=match row{Ok(row)=>row,Err(_)=>{report.invalid+=1;continue}};
            let value=|i:Option<usize>|i.and_then(|i|row.get(i)).unwrap_or("");
            let a:lettre::Address=match value(Some(email)).trim().parse(){Ok(a)=>a,Err(_)=>{report.invalid+=1;continue}};
            let inserted=tx.execute("INSERT OR IGNORE INTO contacts(id,email,name,group_name,notes,updated_at) VALUES(?,?,?,?,?,unixepoch())",params![Uuid::new_v4().to_string(),a.to_string(),value(name),value(group),value(notes)])?;
            if inserted==1{report.added+=1}else{report.skipped+=1}
        }
        tx.commit()?;Ok(report)
    }
    pub fn export_account_config(&self,path:&Path)->Result<usize>{
        let accounts:Vec<Account>=self.accounts()?.into_iter().filter(|a|a.auth_kind!="local").collect();let count=accounts.len();
        let file=private_file(path)?;
        serde_json::to_writer_pretty(&file,&ConfigFile{format:"mail-ex-accounts".into(),version:1,accounts})?;
        file.sync_all()?;Ok(count)
    }
    pub fn import_account_config(&mut self,path:&Path)->Result<ImportReport>{
        let config:ConfigFile=serde_json::from_reader(BufReader::new(File::open(path)?))?;
        if config.format!="mail-ex-accounts"||config.version!=1{bail!("Unsupported account configuration file")}
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut report=ImportReport::default();
        for a in config.accounts{
            let valid_identity=a.email.parse::<lettre::Address>().is_ok()&&!a.username.is_empty()&&!a.username.chars().any(char::is_control);
            let valid_endpoint=if matches!(a.auth_kind.as_str(),"password"|"pop3"){valid_host(&a.imap_host)&&valid_host(&a.smtp_host)&&a.imap_port>0&&a.smtp_port>0}
            else{super::oauth::Provider::parse(&a.auth_kind).ok().is_some_and(|p|{let (imap,smtp,port,starttls)=p.servers();a.imap_host==imap&&a.imap_port==993&&a.smtp_host==smtp&&a.smtp_port==port&&a.smtp_starttls==starttls&&a.username==a.email})};
            if !valid_identity||!valid_endpoint{report.invalid+=1;continue}
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE lower(email)=lower(?1) AND username=?2 AND imap_host=?3 AND imap_port=?4)",params![a.email,a.username,a.imap_host,a.imap_port],|r|r.get(0))?;
            if exists{report.skipped+=1;continue}
            let id=Uuid::new_v4().to_string();let secret=encrypt(&self.key,&id,"")?;
            tx.execute("INSERT INTO accounts(id,name,email,username,imap_host,imap_port,smtp_host,smtp_port,smtp_starttls,color,secret,signature,auth_kind) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",params![id,a.name,a.email,a.username,a.imap_host,a.imap_port,a.smtp_host,a.smtp_port,a.smtp_starttls,a.color,secret,a.signature,a.auth_kind])?;
            tx.execute("INSERT INTO folders(account_id,name,label,kind) VALUES(?,'INBOX','INBOX','inbox')",[&id])?;
            report.added+=1;
        }
        tx.commit()?;Ok(report)
    }
    pub fn storage_stats(&self)->Result<StorageStats>{
        let count=|table:&str|->Result<i64>{Ok(self.db.query_row(&format!("SELECT count(*) FROM {table}"),[],|r|r.get(0))?)};
        Ok(StorageStats{directory:self.dir.to_string_lossy().into_owned(),database_bytes:file_size(&self.dir.join(DB)),wal_bytes:file_size(&self.dir.join("mail.sqlite3-wal")),shm_bytes:file_size(&self.dir.join("mail.sqlite3-shm")),raw_mail_bytes:self.db.query_row("SELECT coalesce(sum(length(raw)),0) FROM messages",[],|r|r.get(0))?,messages:count("messages")?,local_messages:self.db.query_row("SELECT count(*) FROM messages WHERE account_id=?",[ARCHIVE_ID],|r|r.get(0))?,accounts:count("accounts")?,contacts:count("contacts")?,drafts:count("drafts")?,outbox:count("outbox")?})
    }
    pub fn cache_preview(&self,account_id:Option<String>,before:Option<i64>)->Result<CacheReport>{
        Ok(self.db.query_row("SELECT count(*),coalesce(sum(length(raw)),0) FROM messages WHERE account_id IN (SELECT id FROM accounts WHERE auth_kind NOT IN ('local','pop3')) AND (?1 IS NULL OR account_id=?1) AND (?2 IS NULL OR timestamp<?2)",params![account_id,before],|r|Ok(CacheReport{messages:r.get(0)?,raw_bytes:r.get(1)?}))?)
    }
    pub fn clear_cache(&mut self,account_id:Option<String>,before:Option<i64>)->Result<CacheReport>{
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result=tx.query_row("SELECT count(*),coalesce(sum(length(raw)),0) FROM messages WHERE account_id IN (SELECT id FROM accounts WHERE auth_kind NOT IN ('local','pop3')) AND (?1 IS NULL OR account_id=?1) AND (?2 IS NULL OR timestamp<?2)",params![account_id,before],|r|Ok(CacheReport{messages:r.get(0)?,raw_bytes:r.get(1)?}))?;
        tx.execute("DELETE FROM messages WHERE account_id IN (SELECT id FROM accounts WHERE auth_kind NOT IN ('local','pop3')) AND (?1 IS NULL OR account_id=?1) AND (?2 IS NULL OR timestamp<?2)",params![account_id,before])?;
        tx.commit()?;Ok(result)
    }
    pub fn optimize_storage(&self)->Result<()>{
        self.db.execute_batch("PRAGMA optimize; PRAGMA wal_checkpoint(PASSIVE); VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")?;Ok(())
    }
    pub fn create_backup(&self,parent:&Path)->Result<SnapshotReport>{self.snapshot(parent,true)}
    pub fn copy_storage(&self,parent:&Path)->Result<SnapshotReport>{self.snapshot(parent,false)}
    fn snapshot(&self,parent:&Path,portable:bool)->Result<SnapshotReport>{
        let path=private_dir(parent,if portable{"mail-ex-backup"}else{"mail-ex-data"})?;
        let result=(||->Result<SnapshotReport>{
            let destination=path.join(DB);
            let destination_str=destination.to_str().ok_or_else(||anyhow!("Backup path must be valid Unicode"))?;
            self.db.execute("VACUUM INTO ?1",[destination_str])?;
            #[cfg(unix)]{use std::os::unix::fs::PermissionsExt;fs::set_permissions(&destination,fs::Permissions::from_mode(0o600))?;}
            let snapshot=Connection::open(&destination)?;
            let suspended_sends=suspend_outbox(&snapshot)?;
            snapshot.execute_batch("PRAGMA journal_mode=DELETE;")?;
            check_snapshot(&snapshot,&schema_signature(&self.db)?)?;
            verify_vault(&snapshot,&self.key)?;
            drop(snapshot);
            // Windows FlushFileBuffers requires a handle opened for writing.
            // The snapshot is our private newly-created database, never a live DB.
            OpenOptions::new().read(true).write(true).open(&destination)?
                .sync_all().context("Cannot flush snapshot database to disk")?;
            let mut key_file=private_file(&path.join(KEY))?;key_file.write_all(&self.key)?;key_file.sync_all()?;
            if portable{
                let manifest=Manifest{format:"mail-ex-backup".into(),version:1,schema:SCHEMA,db_sha256:hash_file(&destination)?,key_sha256:hash_file(&path.join(KEY))?};
                let file=private_file(&path.join(MANIFEST))?;serde_json::to_writer_pretty(&file,&manifest)?;file.sync_all()?;
            }
            Ok(SnapshotReport{directory:path.to_string_lossy().into_owned(),suspended_sends})
        })();
        if result.is_err(){let _=fs::remove_dir_all(&path);}
        result
    }
    pub fn restore_backup(&self,source:&Path,parent:&Path)->Result<SnapshotReport>{
        let manifest:Manifest=serde_json::from_reader(BufReader::new(File::open(source.join(MANIFEST))?))?;
        if manifest.format!="mail-ex-backup"||manifest.version!=1||manifest.schema!=SCHEMA{bail!("Unsupported backup format or schema")}
        let path=private_dir(parent,"mail-ex-data")?;
        let result=(||->Result<SnapshotReport>{
            for file in [DB,KEY]{copy_private(&source.join(file),&path.join(file))?;}
            if hash_file(&path.join(DB))?!=manifest.db_sha256||hash_file(&path.join(KEY))?!=manifest.key_sha256{bail!("Backup checksum mismatch")}
            let key:[u8;32]=fs::read(path.join(KEY))?.try_into().map_err(|_|anyhow!("Invalid backup credential key"))?;
            // Only accept a schema identical to the active application schema.
            // Read-only inspection before opening writable or running application migrations.
            let inspect=Connection::open_with_flags(path.join(DB),OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            check_snapshot(&inspect,&schema_signature(&self.db)?)?;verify_vault(&inspect,&key)?;drop(inspect);
            let db=Connection::open(path.join(DB))?;
            let suspended_sends=suspend_outbox(&db)?;db.execute_batch("PRAGMA journal_mode=DELETE;")?;drop(db);
            Ok(SnapshotReport{directory:path.to_string_lossy().into_owned(),suspended_sends})
        })();
        if result.is_err(){let _=fs::remove_dir_all(&path);}
        result
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn input()->super::super::NewAccount{super::super::NewAccount{name:"Test".into(),email:"fixture@example.invalid".into(),username:"fixture@example.invalid".into(),password:"fixture-secret".into(),imap_host:"imap.example.invalid".into(),imap_port:993,smtp_host:"smtp.example.invalid".into(),smtp_port:465,smtp_starttls:false,color:"#123456".into(),signature:String::new()}}
    const RAW:&[u8]=b"From: Alice <alice@example.invalid>\r\nTo: fixture@example.invalid\r\nSubject: Local fixture\r\n\r\nFrom original line\r\n>From quoted line\r\n";
    #[test]fn archives_and_csv_config(){
        let tmp=tempfile::tempdir().unwrap();let live=tmp.path().join("live");let mut s=Store::open(&live).unwrap();
        let account=s.add_account(input()).unwrap();
        let file=tmp.path().join("test.eml");fs::write(&file,RAW).unwrap();
        assert_eq!(s.import_mail(&[file.clone()],"eml").unwrap().added,1);
        assert_eq!(s.import_mail(&[file],"eml").unwrap().skipped,1);
        let id=s.messages(Some(ARCHIVE_ID.into()),None,false,false,"Local fixture",0).unwrap()[0].id;
        let eml=tmp.path().join("export.eml");s.export_eml(id,&eml).unwrap();assert_eq!(fs::read(&eml).unwrap(),RAW);
        let mbox=tmp.path().join("export.mbox");assert_eq!(s.export_mbox(Some(ARCHIVE_ID.into()),None,&mbox).unwrap(),1);
        let other=tmp.path().join("other");let mut other=Store::open(&other).unwrap();
        assert_eq!(other.import_mail(&[mbox],"mbox").unwrap().added,1);
        let other_id=other.messages(None,None,false,false,"Local fixture",0).unwrap()[0].id;
        let roundtrip=tmp.path().join("roundtrip.eml");other.export_eml(other_id,&roundtrip).unwrap();assert_eq!(fs::read(roundtrip).unwrap(),RAW);
        let csv=tmp.path().join("contacts.csv");fs::write(&csv,"email,name,group,notes\r\nalice@example.invalid,\"Alice, A\",Team,\"line 1\nline 2\"\r\n").unwrap();
        assert_eq!(s.import_contacts(&csv).unwrap().added,1);assert_eq!(s.import_contacts(&csv).unwrap().skipped,1);
        let exported=tmp.path().join("contacts-out.csv");s.export_contacts(&exported).unwrap();assert_eq!(other.import_contacts(&exported).unwrap().added,1);assert_eq!(other.contacts().unwrap()[0].notes,"line 1\nline 2");
        let config=tmp.path().join("accounts.json");assert_eq!(s.export_account_config(&config).unwrap(),1);
        let json=fs::read_to_string(&config).unwrap();assert!(!json.contains("fixture-secret"));
        assert_eq!(other.import_account_config(&config).unwrap().added,1);
        let imported=other.accounts().unwrap().into_iter().find(|a|a.auth_kind=="password").unwrap();assert!(other.credential(&imported).is_err());
        assert_eq!(s.cache_preview(None,None).unwrap().messages,0);
        assert_eq!(s.clear_cache(Some(account.id),None).unwrap().messages,0);assert_eq!(s.storage_stats().unwrap().local_messages,1);
    }
    #[test]fn snapshot_restore_and_queue_safety(){
        let tmp=tempfile::tempdir().unwrap();let mut s=Store::open(&tmp.path().join("live")).unwrap();let a=s.add_account(input()).unwrap();
        let mail=super::super::Compose{account_id:a.id.clone(),to:"receiver@example.invalid".into(),cc:String::new(),bcc:String::new(),subject:"Queued".into(),body:"Only fixture".into(),html:None,attachments:Vec::new(),in_reply_to:None,references:None,save_sent_copy:false};
        let id=Uuid::new_v4().to_string();s.enqueue_mail(id,mail,i64::MAX,None).unwrap();
        let snapshot=s.create_backup(tmp.path()).unwrap();assert_eq!(snapshot.suspended_sends,1);assert_eq!(s.outbox().unwrap()[0].state,"queued");
        let restored=s.restore_backup(Path::new(&snapshot.directory),tmp.path()).unwrap();
        let restored=Store::open(Path::new(&restored.directory)).unwrap();
        assert_eq!(restored.password(&a.id).unwrap(),"fixture-secret");assert_eq!(restored.outbox().unwrap()[0].state,"uncertain");
        let bad=Path::new(&snapshot.directory).join(KEY);fs::write(bad,[0u8;32]).unwrap();
        assert!(s.restore_backup(Path::new(&snapshot.directory),tmp.path()).is_err());
    }
}
