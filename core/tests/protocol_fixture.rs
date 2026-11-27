use mail_ex_core::{Compose, NewAccount, Store};

/// Run via scripts/protocol-fixture.py, which creates a private CA and two
/// loopback-only TLS servers. Never use actual mail credentials here.
#[test]
#[ignore = "requires the loopback TLS protocol fixture"]
fn tls_imap_and_smtp_roundtrip() {
    let imap_port = std::env::var("MAIL_EX_FIXTURE_IMAP_PORT").unwrap().parse().unwrap();
    let smtp_port = std::env::var("MAIL_EX_FIXTURE_SMTP_PORT").unwrap().parse().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let account = store.add_account(NewAccount {
        name:"Fixture".into(), email:"fixture@example.invalid".into(),
        username:"fixture@example.invalid".into(), password:"fixture-only-secret".into(),
        imap_host:"localhost".into(), imap_port, smtp_host:"localhost".into(), smtp_port,
        smtp_starttls:false, color:"#3469c7".into(), signature:"".into(),
    }).unwrap();
    store.test_account(&account.id).unwrap();
    let first=store.sync_account(&account.id,None,false).unwrap();
    assert_eq!(first.added,1);
    let folders=store.folders().unwrap();
    assert!(folders.iter().any(|f|f.kind=="archive"&&f.name=="Archive"));
    assert!(folders.iter().any(|f|f.kind=="trash"&&f.name=="Trash"));
    let mail=store.messages(None,Some("INBOX".into()),true,false,"",0).unwrap();
    assert_eq!(mail.len(),1);
    let id=mail[0].id;
    assert_eq!(store.message(id).unwrap().body.trim(),"Fixture body.");
    store.set_seen(id,true).unwrap();
    store.set_starred(id,true).unwrap();
    assert!(store.message(id).unwrap().summary.seen);
    assert!(store.message(id).unwrap().summary.starred);
    let again=store.sync_account(&account.id,None,false).unwrap();
    assert_eq!(again.added,0);
    let mail=store.messages(None,None,false,true,"",0).unwrap();
    assert_eq!(mail.len(),1);
    let reply=store.compose_from(id,"reply_all").unwrap();
    assert_eq!(reply.to,"sender@example.invalid");
    assert_eq!(reply.cc,"other@example.invalid");
    assert_eq!(reply.in_reply_to.as_deref(),Some("<fixture-1@example.invalid>"));
    let draft_id=store.save_draft(None,reply).unwrap();
    assert_eq!(store.drafts().unwrap().len(),1);
    store.delete_draft(&draft_id).unwrap();
    store.move_message(id,"Archive").unwrap();
    assert!(store.messages(None,Some("INBOX".into()),false,false,"",0).unwrap().is_empty());
    store.sync_account(&account.id,Some("Archive".into()),false).unwrap();
    assert_eq!(store.messages(None,Some("Archive".into()),false,true,"",0).unwrap().len(),1);
    store.send(Compose {
        account_id:account.id,to:"receiver@example.invalid".into(),cc:"".into(),
        bcc:"hidden@example.invalid".into(),subject:"Fixture delivery".into(),
        body:"Local test only.".into(),in_reply_to:None,references:None,html:None,attachments:Vec::new(),save_sent_copy:false,
    }).unwrap();
}
