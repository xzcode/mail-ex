use mail_ex_core::{Compose, NewAccount, Store, OutgoingAttachment};
use base64::{engine::general_purpose::STANDARD,Engine};

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
    let incoming=store.message(id).unwrap();
    assert_eq!(incoming.body.trim(),"Fixture body.");
    assert_eq!(incoming.html.as_deref().map(str::trim),Some("<p><b>Fixture HTML.</b></p>"));
    assert_eq!(incoming.attachments.len(),1);
    assert_eq!(incoming.attachments[0].name,"incoming.txt");
    assert_eq!(incoming.attachments[0].size,3);
    assert_eq!(store.attachment_preview(id,0).unwrap().data,"abc");
    assert_eq!(store.attachment_download(id,0).unwrap().1,b"abc");
    let forwarded=store.compose_from(id,"forward").unwrap();
    assert_eq!(forwarded.attachments.len(),1);
    assert_eq!(STANDARD.decode(&forwarded.attachments[0].data).unwrap(),b"abc");
    let forward_id=store.save_draft(None,forwarded).unwrap();
    assert_eq!(store.draft(&forward_id).unwrap().mail.attachments.len(),1);
    assert!(store.drafts().unwrap()[0].mail.attachments.is_empty());
    store.delete_draft(&forward_id).unwrap();
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
    let payload=b"\x00\xffABC\n\r\n";
    let report=store.send(Compose {
        account_id:account.id.clone(),to:"receiver@example.invalid".into(),cc:"".into(),
        bcc:"hidden@example.invalid".into(),subject:"Fixture delivery".into(),
        body:"Local test only.".into(),in_reply_to:None,references:None,
        html:Some("<p><strong>Local test only.</strong><img src='https://tracker.invalid/a'></p>".into()),
        attachments:vec![OutgoingAttachment{name:"fixture.bin".into(),media_type:"application/octet-stream".into(),data:STANDARD.encode(payload)}],
        save_sent_copy:true,
    }).unwrap();
    assert_eq!(report.sent_copy,"saved");
    let sent=store.sync_account(&account.id,Some("Sent".into()),false).unwrap();
    assert_eq!(sent.added,1);
    let sent_list=store.messages(None,Some("Sent".into()),false,false,"",0).unwrap();
    assert_eq!(sent_list.len(),1);
    let sent_id=sent_list[0].id;
    assert_eq!(store.attachment_download(sent_id,0).unwrap().1,payload);
    assert_eq!(store.message(sent_id).unwrap().html.as_deref().map(str::trim),Some("<p><strong>Local test only.</strong></p>"));
}
