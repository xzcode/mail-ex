//! Local organization data and durable, explicitly requested delivery queue.
use super::{Store,Compose,SearchFilters,Uuid,mime};
use anyhow::{anyhow,bail,Context,Result};
use rusqlite::{params,Connection,OptionalExtension,TransactionBehavior};
use serde::{Deserialize,Serialize};
use std::time::{SystemTime,UNIX_EPOCH};

pub(crate) fn migrate(db:&Connection)->Result<()>{
    db.execute_batch("CREATE TABLE contacts(id TEXT PRIMARY KEY,email TEXT NOT NULL COLLATE NOCASE UNIQUE,name TEXT NOT NULL,group_name TEXT NOT NULL,notes TEXT NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE templates(id TEXT PRIMARY KEY,name TEXT NOT NULL,subject TEXT NOT NULL,body TEXT NOT NULL,html TEXT,updated_at INTEGER NOT NULL);
      CREATE TABLE saved_searches(id TEXT PRIMARY KEY,name TEXT NOT NULL,account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,mode TEXT NOT NULL,folder TEXT NOT NULL,query TEXT NOT NULL,filters TEXT NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE mail_rules(id TEXT PRIMARY KEY,name TEXT NOT NULL,account_id TEXT REFERENCES accounts(id) ON DELETE CASCADE,enabled INTEGER NOT NULL,sender TEXT NOT NULL,recipient TEXT NOT NULL,subject TEXT NOT NULL,body TEXT NOT NULL,label TEXT NOT NULL,match_any INTEGER NOT NULL,updated_at INTEGER NOT NULL);
      CREATE TABLE rule_labels(message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,rule_id TEXT NOT NULL REFERENCES mail_rules(id) ON DELETE CASCADE,PRIMARY KEY(message_id,rule_id));
      CREATE INDEX rule_labels_by_rule ON rule_labels(rule_id,message_id);
      CREATE TRIGGER auto_label_inbox AFTER INSERT ON messages WHEN new.folder='INBOX' BEGIN
        INSERT OR IGNORE INTO rule_labels(message_id,rule_id)
        SELECT new.id,r.id FROM mail_rules r WHERE r.enabled=1 AND (r.account_id IS NULL OR r.account_id=new.account_id) AND
        ((r.match_any=0 AND (r.sender='' OR instr(lower(new.sender),lower(r.sender))>0) AND (r.recipient='' OR instr(lower(new.recipients),lower(r.recipient))>0) AND (r.subject='' OR instr(lower(new.subject),lower(r.subject))>0) AND (r.body='' OR instr(lower(new.body),lower(r.body))>0))
        OR (r.match_any=1 AND ((r.sender<>'' AND instr(lower(new.sender),lower(r.sender))>0) OR (r.recipient<>'' AND instr(lower(new.recipients),lower(r.recipient))>0) OR (r.subject<>'' AND instr(lower(new.subject),lower(r.subject))>0) OR (r.body<>'' AND instr(lower(new.body),lower(r.body))>0))));
      END;
      CREATE TABLE outbox(id TEXT PRIMARY KEY,account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,recipients TEXT NOT NULL,subject TEXT NOT NULL,payload TEXT NOT NULL,due_at INTEGER NOT NULL,state TEXT NOT NULL CHECK(state IN ('queued','sending','accepted','uncertain','canceled')),detail TEXT NOT NULL DEFAULT '',sent_copy TEXT NOT NULL DEFAULT '',created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL);
      CREATE INDEX outbox_due ON outbox(state,due_at,created_at,id);
      CREATE TRIGGER block_account_pending BEFORE DELETE ON accounts WHEN EXISTS(SELECT 1 FROM outbox WHERE account_id=old.id AND state IN ('queued','sending')) BEGIN
        SELECT RAISE(ABORT,'Cancel pending outbox messages or resolve interrupted sends before removing this account');
      END;
      PRAGMA user_version=6;")?;
    Ok(())
}
static DISPATCH:std::sync::Mutex<()>=std::sync::Mutex::new(());
fn now()->i64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64}
fn id(value:&str)->Result<String>{
    if value.is_empty(){return Ok(Uuid::new_v4().to_string())}
    Uuid::parse_str(value).map_err(|_|anyhow!("Invalid record ID"))?;
    Ok(value.to_owned())
}
fn name(value:&str)->Result<String>{
    let v=value.trim();if v.is_empty(){bail!("Name is required")}Ok(v.to_owned())
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct Contact{pub id:String,pub email:String,pub name:String,pub group_name:String,pub notes:String}
impl Default for Contact{fn default()->Self{Self{id:String::new(),email:String::new(),name:String::new(),group_name:String::new(),notes:String::new()}}}
#[derive(Default,Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct MailTemplate{pub id:String,pub name:String,pub subject:String,pub body:String,pub html:Option<String>}
#[derive(Default,Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct SavedSearch{pub id:String,pub name:String,pub account_id:Option<String>,pub mode:String,pub folder:String,pub query:String,pub filters:SearchFilters}
#[derive(Default,Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct MailRule{pub id:String,pub name:String,pub account_id:Option<String>,pub enabled:bool,pub sender:String,pub recipient:String,pub subject:String,pub body:String,pub label:String,pub match_any:bool}
#[derive(Serialize)]
pub struct LabelCount{pub name:String,pub total:i64,pub unread:i64}
#[derive(Serialize)]
pub struct RuleReport{pub matches:usize,pub newly_labeled:usize}
#[derive(Clone,Serialize)]
pub struct OutboxItem{
    pub id:String,pub account_id:String,pub recipients:String,pub subject:String,pub due_at:i64,pub state:String,pub detail:String,pub sent_copy:String,pub created_at:i64,pub updated_at:i64,
}
fn outbox_row(r:&rusqlite::Row<'_>)->rusqlite::Result<OutboxItem>{
    Ok(OutboxItem{id:r.get(0)?,account_id:r.get(1)?,recipients:r.get(2)?,subject:r.get(3)?,due_at:r.get(4)?,state:r.get(5)?,detail:r.get(6)?,sent_copy:r.get(7)?,created_at:r.get(8)?,updated_at:r.get(9)?})
}
const OUTBOX_COLUMNS:&str="id,account_id,recipients,subject,due_at,state,detail,sent_copy,created_at,updated_at";
const RULE_PREDICATE:&str="m.folder='INBOX' AND (?1 IS NULL OR m.account_id=?1) AND ((?2=0 AND (?3='' OR instr(lower(m.sender),lower(?3))>0) AND (?4='' OR instr(lower(m.recipients),lower(?4))>0) AND (?5='' OR instr(lower(m.subject),lower(?5))>0) AND (?6='' OR instr(lower(m.body),lower(?6))>0)) OR (?2=1 AND ((?3<>'' AND instr(lower(m.sender),lower(?3))>0) OR (?4<>'' AND instr(lower(m.recipients),lower(?4))>0) OR (?5<>'' AND instr(lower(m.subject),lower(?5))>0) OR (?6<>'' AND instr(lower(m.body),lower(?6))>0))))";
fn rule_row(r:&rusqlite::Row<'_>)->rusqlite::Result<MailRule>{
    Ok(MailRule{id:r.get(0)?,name:r.get(1)?,account_id:r.get(2)?,enabled:r.get(3)?,sender:r.get(4)?,recipient:r.get(5)?,subject:r.get(6)?,body:r.get(7)?,label:r.get(8)?,match_any:r.get(9)?})
}
const RULE_COLUMNS:&str="id,name,account_id,enabled,sender,recipient,subject,body,label,match_any";
fn normalize_rule(mut r:MailRule)->Result<MailRule>{
    r.name=name(&r.name)?;r.label=name(&r.label)?;
    r.sender=r.sender.trim().to_owned();r.recipient=r.recipient.trim().to_owned();r.subject=r.subject.trim().to_owned();r.body=r.body.trim().to_owned();
    if [&r.sender,&r.recipient,&r.subject,&r.body].iter().all(|v|v.is_empty()){bail!("Specify at least one rule condition")}
    Ok(r)
}
impl Store{
    pub fn contacts(&self)->Result<Vec<Contact>>{
        let mut q=self.db.prepare("SELECT id,email,name,group_name,notes FROM contacts ORDER BY lower(group_name),lower(name),lower(email),id")?;
        let rows=q.query_map([],|r|Ok(Contact{id:r.get(0)?,email:r.get(1)?,name:r.get(2)?,group_name:r.get(3)?,notes:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn save_contact(&self,mut c:Contact)->Result<Contact>{
        c.id=id(&c.id)?;let address:lettre::Address=c.email.trim().parse().map_err(|_|anyhow!("Invalid contact email address"))?;
        c.email=address.to_string();c.name=c.name.trim().to_owned();c.group_name=c.group_name.trim().to_owned();
        self.db.execute("INSERT INTO contacts(id,email,name,group_name,notes,updated_at) VALUES(?,?,?,?,?,unixepoch()) ON CONFLICT(id) DO UPDATE SET email=excluded.email,name=excluded.name,group_name=excluded.group_name,notes=excluded.notes,updated_at=excluded.updated_at",params![c.id,c.email,c.name,c.group_name,c.notes]).map_err(|e|match e{rusqlite::Error::SqliteFailure(ref err,_) if err.code==rusqlite::ErrorCode::ConstraintViolation=>anyhow!("This email address is already in the address book"),_=>e.into()})?;
        Ok(c)
    }
    pub fn delete_contact(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM contacts WHERE id=?",[id])?;Ok(())}
    pub fn templates(&self)->Result<Vec<MailTemplate>>{
        let mut q=self.db.prepare("SELECT id,name,subject,body,html FROM templates ORDER BY lower(name),id")?;
        let rows=q.query_map([],|r|Ok(MailTemplate{id:r.get(0)?,name:r.get(1)?,subject:r.get(2)?,body:r.get(3)?,html:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn save_template(&self,mut t:MailTemplate)->Result<MailTemplate>{
        t.id=id(&t.id)?;t.name=name(&t.name)?;t.html=t.html.map(|h|mime::sanitize_html(&h));
        self.db.execute("INSERT INTO templates(id,name,subject,body,html,updated_at) VALUES(?,?,?,?,?,unixepoch()) ON CONFLICT(id) DO UPDATE SET name=excluded.name,subject=excluded.subject,body=excluded.body,html=excluded.html,updated_at=excluded.updated_at",params![t.id,t.name,t.subject,t.body,t.html])?;
        Ok(t)
    }
    pub fn delete_template(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM templates WHERE id=?",[id])?;Ok(())}
    pub fn saved_searches(&self)->Result<Vec<SavedSearch>>{
        let mut q=self.db.prepare("SELECT id,name,account_id,mode,folder,query,filters FROM saved_searches ORDER BY lower(name),id")?;
        let rows=q.query_map([],|r|Ok((SavedSearch{id:r.get(0)?,name:r.get(1)?,account_id:r.get(2)?,mode:r.get(3)?,folder:r.get(4)?,query:r.get(5)?,filters:SearchFilters::default()},r.get::<_,String>(6)?)))?;
        let mut result=Vec::new();for row in rows{let (mut search,filters)=row?;search.filters=serde_json::from_str(&filters)?;result.push(search)}
        Ok(result)
    }
    pub fn save_search(&self,mut s:SavedSearch)->Result<SavedSearch>{
        s.id=id(&s.id)?;s.name=name(&s.name)?;
        if !["inbox","unread","starred","all","folder"].contains(&s.mode.as_str()){bail!("Invalid saved search scope")}
        if s.mode=="folder"&&(s.account_id.is_none()||s.folder.is_empty()){bail!("Folder searches require an account and folder")}
        if let Some(a)=&s.account_id{self.account(a)?;}
        if matches!((s.filters.since,s.filters.until),(Some(from),Some(to)) if from>=to){bail!("Invalid search date range")}
        let filters=serde_json::to_string(&s.filters)?;
        self.db.execute("INSERT INTO saved_searches(id,name,account_id,mode,folder,query,filters,updated_at) VALUES(?,?,?,?,?,?,?,unixepoch()) ON CONFLICT(id) DO UPDATE SET name=excluded.name,account_id=excluded.account_id,mode=excluded.mode,folder=excluded.folder,query=excluded.query,filters=excluded.filters,updated_at=excluded.updated_at",params![s.id,s.name,s.account_id,s.mode,s.folder,s.query,filters])?;
        Ok(s)
    }
    pub fn delete_search(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM saved_searches WHERE id=?",[id])?;Ok(())}
    pub fn rules(&self)->Result<Vec<MailRule>>{
        let mut q=self.db.prepare(&format!("SELECT {RULE_COLUMNS} FROM mail_rules ORDER BY lower(name),id"))?;
        let rows=q.query_map([],rule_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn save_rule(&self,r:MailRule)->Result<MailRule>{
        let mut r=normalize_rule(r)?;r.id=id(&r.id)?;
        if let Some(a)=&r.account_id{self.account(a)?;}
        self.db.execute("INSERT INTO mail_rules(id,name,account_id,enabled,sender,recipient,subject,body,label,match_any,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,unixepoch()) ON CONFLICT(id) DO UPDATE SET name=excluded.name,account_id=excluded.account_id,enabled=excluded.enabled,sender=excluded.sender,recipient=excluded.recipient,subject=excluded.subject,body=excluded.body,label=excluded.label,match_any=excluded.match_any,updated_at=excluded.updated_at",params![r.id,r.name,r.account_id,r.enabled,r.sender,r.recipient,r.subject,r.body,r.label,r.match_any])?;
        Ok(r)
    }
    pub fn delete_rule(&self,id:&str)->Result<()>{self.db.execute("DELETE FROM mail_rules WHERE id=?",[id])?;Ok(())}
    pub fn preview_rule(&self,r:MailRule)->Result<usize>{
        let r=normalize_rule(r)?;
        Ok(self.db.query_row(&format!("SELECT count(*) FROM messages m WHERE {RULE_PREDICATE}"),params![r.account_id,r.match_any,r.sender,r.recipient,r.subject,r.body],|row|row.get(0))?)
    }
    pub fn apply_rule(&mut self,id:&str)->Result<RuleReport>{
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let r=tx.query_row(&format!("SELECT {RULE_COLUMNS} FROM mail_rules WHERE id=?"),[id],rule_row)?;
        let matches:usize=tx.query_row(&format!("SELECT count(*) FROM messages m WHERE {RULE_PREDICATE}"),params![r.account_id,r.match_any,r.sender,r.recipient,r.subject,r.body],|row|row.get(0))?;
        let newly_labeled=tx.execute(&format!("INSERT OR IGNORE INTO rule_labels(message_id,rule_id) SELECT m.id,?7 FROM messages m WHERE {RULE_PREDICATE}"),params![r.account_id,r.match_any,r.sender,r.recipient,r.subject,r.body,r.id])?;
        // Explicit apply is additive; existing manual/history classifications are retained.
        tx.commit()?;Ok(RuleReport{matches,newly_labeled})
    }
    pub fn labels(&self)->Result<Vec<LabelCount>>{
        let mut q=self.db.prepare("SELECT r.label,count(DISTINCT m.id),count(DISTINCT CASE WHEN m.seen=0 THEN m.id END) FROM mail_rules r LEFT JOIN rule_labels l ON l.rule_id=r.id LEFT JOIN messages m ON m.id=l.message_id GROUP BY r.label ORDER BY lower(r.label),r.label")?;
        let rows=q.query_map([],|r|Ok(LabelCount{name:r.get(0)?,total:r.get(1)?,unread:r.get(2)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn message_labels(&self,id:i64)->Result<Vec<String>>{
        let mut q=self.db.prepare("SELECT DISTINCT r.label FROM rule_labels l JOIN mail_rules r ON r.id=l.rule_id WHERE l.message_id=? ORDER BY r.label")?;
        let rows=q.query_map([id],|r|r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn outbox(&self)->Result<Vec<OutboxItem>>{
        let mut q=self.db.prepare(&format!("SELECT {OUTBOX_COLUMNS} FROM outbox ORDER BY CASE state WHEN 'sending' THEN 0 WHEN 'queued' THEN 1 WHEN 'uncertain' THEN 2 ELSE 3 END,due_at DESC,created_at DESC,id"))?;
        let rows=q.query_map([],outbox_row)?.collect::<rusqlite::Result<Vec<_>>>()?;Ok(rows)
    }
    fn outbox_item(&self,id:&str)->Result<OutboxItem>{
        Ok(self.db.query_row(&format!("SELECT {OUTBOX_COLUMNS} FROM outbox WHERE id=?"),[id],outbox_row)?)
    }
    pub fn enqueue_mail(&mut self,request_id:String,mail:Compose,due_at:i64,draft_id:Option<String>)->Result<OutboxItem>{
        if request_id.is_empty(){bail!("A stable delivery request ID is required")}
        let request_id=id(&request_id)?;
        if due_at<0{bail!("Invalid delivery time")}
        let account=self.account(&mail.account_id)?;
        // Preflight full MIME before queueing; no network I/O occurs here.
        super::prepare_message(&account,mail.clone())?;
        let payload=serde_json::to_string(&mail)?;
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing:Option<(String,i64)>=tx.query_row("SELECT payload,due_at FROM outbox WHERE id=?",[&request_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        if let Some((existing_payload,existing_time))=existing{
            if existing_payload!=payload||existing_time!=due_at{bail!("Delivery request already exists with different content; inspect the outbox")}
        }else{
            tx.execute("INSERT INTO outbox(id,account_id,recipients,subject,payload,due_at,state,created_at,updated_at) VALUES(?,?,?,?,?,?,'queued',unixepoch(),unixepoch())",params![request_id,mail.account_id,mail.to,mail.subject,payload,due_at])?;
            if let Some(draft_id)=draft_id{tx.execute("DELETE FROM drafts WHERE id=?",[draft_id])?;}
        }
        tx.commit()?;self.outbox_item(&request_id)
    }
    // Claim atomically before contacting the network. Never requeue a claimed item.
    fn claim_outbox(&mut self,requested:Option<&str>)->Result<Option<(String,Compose)>>{
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row:Option<(String,String)>=tx.query_row("SELECT id,payload FROM outbox WHERE state='queued' AND due_at<=?1 AND (?2 IS NULL OR id=?2) ORDER BY due_at,created_at,id LIMIT 1",params![now(),requested],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let Some((id,payload))=row else{return Ok(None)};
        let mail:Compose=serde_json::from_str(&payload).context("Invalid queued mail")?;
        let changed=tx.execute("UPDATE outbox SET state='sending',updated_at=unixepoch(),detail='' WHERE id=? AND state='queued'",[&id])?;
        if changed!=1{bail!("Outbox item is no longer queued")}
        tx.commit()?;Ok(Some((id,mail)))
    }
    pub fn dispatch_outbox(&mut self,requested:Option<String>)->Result<Option<OutboxItem>>{
        let Ok(_guard)=DISPATCH.try_lock() else{return Ok(None)};
        let Some((id,mail))=self.claim_outbox(requested.as_deref())? else{return Ok(None)};
        // A crash or connection error after claim leaves an ambiguous item; it is
        // intentionally not automatically sent again, even after restart.
        let (state,detail,sent_copy)=match self.send(mail){
            Ok(r)=>("accepted",if r.sent_copy=="failed_or_uncertain"{"SMTP accepted; server Sent copy failed or is uncertain"}else{"SMTP server accepted the message"},r.sent_copy),
            Err(_)=>("uncertain","Send failed or delivery is uncertain. Verify the mailbox/server before creating a new send request.",String::new()),
        };
        self.db.execute("UPDATE outbox SET state=?,detail=?,sent_copy=?,updated_at=unixepoch() WHERE id=? AND state IN ('sending','uncertain')",params![state,detail,sent_copy,id])?;
        Ok(Some(self.outbox_item(&id)?))
    }
    pub fn cancel_outbox(&self,id:&str)->Result<()>{
        if self.db.execute("UPDATE outbox SET state='canceled',detail='Canceled before delivery claim',updated_at=unixepoch() WHERE id=? AND state='queued'",[id])?!=1{bail!("Only a queued, unclaimed item can be canceled")}
        Ok(())
    }
    pub fn mark_interrupted_outbox(&self,id:&str)->Result<()>{
        let _guard=DISPATCH.try_lock().map_err(|_|anyhow!("A send is still active in this process"))?;
        // This only changes the status; it never requeues or sends. The user must
        // confirm that no other app instance is still processing the delivery.
        if self.db.execute("UPDATE outbox SET state='uncertain',detail='Interrupted or manually marked uncertain; verify the server before resending',updated_at=unixepoch() WHERE id=? AND state='sending'",[id])?!=1{bail!("Item is not marked as sending")}
        Ok(())
    }
    pub fn copy_outbox(&self,id:&str)->Result<Compose>{
        let payload:String=self.db.query_row("SELECT payload FROM outbox WHERE id=? AND state IN ('accepted','canceled','uncertain')",[id],|r|r.get(0)).context("Only completed or canceled items can be copied")?;
        Ok(serde_json::from_str(&payload)?)
    }
    pub fn delete_outbox(&self,id:&str)->Result<()>{
        if self.db.execute("DELETE FROM outbox WHERE id=? AND state IN ('accepted','canceled','uncertain')",[id])?!=1{bail!("Cancel queued mail or resolve interrupted sending first")}
        Ok(())
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn account(store:&Store)->super::super::Account{
        store.add_account(super::super::NewAccount{name:"Fixture".into(),email:"fixture@example.invalid".into(),username:"fixture@example.invalid".into(),password:"local-only".into(),imap_host:"imap.example.invalid".into(),imap_port:993,smtp_host:"smtp.example.invalid".into(),smtp_port:465,smtp_starttls:false,color:"#123456".into(),signature:String::new()}).unwrap()
    }
    fn insert(store:&Store,account:&str,folder:&str,uid:u32,sender:&str,subject:&str)->i64{
        store.db.execute("INSERT INTO messages(account_id,folder,uid_validity,uid,sender,recipients,subject,timestamp,seen,body,html_only,raw,starred) VALUES(?,?,1,?,?,'team@example.invalid',?,100,0,'release notes',0,X'00',0)",params![account,folder,uid,sender,subject]).unwrap();
        store.db.last_insert_rowid()
    }
    #[test]fn rule_labels_auto_manual_and_scoped_search(){
        let dir=tempfile::tempdir().unwrap();let mut store=Store::open(dir.path()).unwrap();let a=account(&store);
        let prior=insert(&store,&a.id,"INBOX",1,"alice","build success");
        let rule=store.save_rule(MailRule{name:"Build".into(),label:"Development".into(),enabled:true,sender:"alice".into(),subject:"build".into(),account_id:Some(a.id.clone()),..Default::default()}).unwrap();
        let fresh=insert(&store,&a.id,"INBOX",2,"Alice","Build failed");
        let rejected=insert(&store,&a.id,"INBOX",3,"Bob","Build success");
        let sent=insert(&store,&a.id,"Sent",4,"Alice","Build success");
        assert!(store.message_labels(prior).unwrap().is_empty());
        assert_eq!(store.message_labels(fresh).unwrap(),vec!["Development"]);
        assert!(store.message_labels(rejected).unwrap().is_empty());
        assert!(store.message_labels(sent).unwrap().is_empty());
        assert_eq!(store.preview_rule(rule.clone()).unwrap(),2);
        assert_eq!(store.apply_rule(&rule.id).unwrap().newly_labeled,1);
        assert_eq!(store.apply_rule(&rule.id).unwrap().newly_labeled,0);
        assert_eq!(store.labels().unwrap()[0].total,2);
        let f=SearchFilters{label:"Development".into(),..Default::default()};
        assert_eq!(store.search_messages(None,None,false,false,"",0,f.clone()).unwrap().len(),2);
        let search=store.save_search(SavedSearch{name:"Build mail".into(),mode:"all".into(),filters:f,..Default::default()}).unwrap();
        assert_eq!(store.saved_searches().unwrap()[0].filters.label,"Development");
        store.delete_search(&search.id).unwrap();
        store.delete_rule(&rule.id).unwrap();assert!(store.message_labels(fresh).unwrap().is_empty());
        assert!(store.save_rule(MailRule{name:"Empty".into(),label:"No".into(),..Default::default()}).is_err());
    }
    #[test]fn contacts_templates_and_draft_queue(){
        let dir=tempfile::tempdir().unwrap();let mut store=Store::open(dir.path()).unwrap();let a=account(&store);
        let c=store.save_contact(Contact{email:"contact@example.invalid".into(),name:"Contact".into(),group_name:"Team".into(),..Default::default()}).unwrap();
        assert_eq!(store.contacts().unwrap()[0].name,"Contact");
        assert!(store.save_contact(Contact{email:"CONTACT@example.invalid".into(),..Default::default()}).is_err());
        let t=store.save_template(MailTemplate{name:"Hello".into(),subject:"Hello".into(),body:"World".into(),html:Some("<b>World</b><script>alert(1)</script>".into()),..Default::default()}).unwrap();
        assert_eq!(t.html.as_deref(),Some("<b>World</b>"));
        let mail=Compose{account_id:a.id.clone(),to:c.email.clone(),cc:String::new(),bcc:String::new(),subject:t.subject,body:t.body,html:t.html,attachments:Vec::new(),in_reply_to:None,references:None,save_sent_copy:false};
        let draft=store.save_draft(None,mail.clone()).unwrap();let request=Uuid::new_v4().to_string();
        let item=store.enqueue_mail(request.clone(),mail.clone(),i64::MAX,Some(draft)).unwrap();
        assert_eq!(item.state,"queued");assert!(store.drafts().unwrap().is_empty());
        assert!(store.claim_outbox(None).unwrap().is_none());
        store.enqueue_mail(request.clone(),mail.clone(),i64::MAX,None).unwrap();
        assert_eq!(store.outbox().unwrap().len(),1);
        assert!(store.remove_account(&a.id).is_err());
        store.cancel_outbox(&request).unwrap();assert_eq!(store.copy_outbox(&request).unwrap().body,"World");
        store.delete_outbox(&request).unwrap();
        let id=Uuid::new_v4().to_string();store.enqueue_mail(id.clone(),mail,0,None).unwrap();
        assert!(store.claim_outbox(Some(&id)).unwrap().is_some());
        drop(store);let mut store=Store::open(dir.path()).unwrap();
        assert!(store.claim_outbox(Some(&id)).unwrap().is_none());
        assert!(store.cancel_outbox(&id).is_err());
        store.mark_interrupted_outbox(&id).unwrap();
        assert!(store.claim_outbox(Some(&id)).unwrap().is_none());
        assert_eq!(store.outbox().unwrap()[0].state,"uncertain");
        store.delete_outbox(&id).unwrap();
        store.delete_contact(&c.id).unwrap();store.delete_template(&t.id).unwrap();
        store.remove_account(&a.id).unwrap();
    }
}
