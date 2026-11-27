use mail_ex_core::{Compose, NewAccount, Store, OutgoingAttachment, NewCalendarSource, CalendarEvent};
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
    let batch=store.batch(vec![id,id,i64::MAX],"archive",None).unwrap();
    assert_eq!(batch.succeeded,vec![id]);
    assert_eq!(batch.failed.len(),1);
    assert_eq!(batch.failed[0].id,i64::MAX);
    assert!(store.messages(None,Some("INBOX".into()),false,false,"",0).unwrap().is_empty());
    store.sync_account(&account.id,Some("Archive".into()),false).unwrap();
    assert_eq!(store.messages(None,Some("Archive".into()),false,true,"",0).unwrap().len(),1);
    let payload=b"\x00\xffABC\n\r\n";
    let outgoing=Compose {
        account_id:account.id.clone(),to:"receiver@example.invalid".into(),cc:"".into(),
        bcc:"hidden@example.invalid".into(),subject:"Fixture delivery".into(),
        body:"Local test only.".into(),in_reply_to:None,references:None,
        html:Some("<p><strong>Local test only.</strong><img src='https://tracker.invalid/a'></p>".into()),
        attachments:vec![OutgoingAttachment{name:"fixture.bin".into(),media_type:"application/octet-stream".into(),data:STANDARD.encode(payload)}],
        save_sent_copy:true,
    };
    let request=uuid::Uuid::new_v4().to_string();
    let draft=store.save_draft(None,outgoing.clone()).unwrap();
    let queued=store.enqueue_mail(request.clone(),outgoing.clone(),0,Some(draft)).unwrap();
    assert_eq!(queued.state,"queued");
    assert!(store.drafts().unwrap().is_empty());
    store.enqueue_mail(request.clone(),outgoing.clone(),0,None).unwrap();
    let report=store.dispatch_outbox(Some(request.clone())).unwrap().unwrap();
    assert_eq!(report.state,"accepted");
    assert_eq!(report.sent_copy,"saved");
    assert!(store.dispatch_outbox(Some(request.clone())).unwrap().is_none());
    assert_eq!(store.enqueue_mail(request.clone(),outgoing.clone(),0,None).unwrap().state,"accepted");
    assert!(store.dispatch_outbox(Some(request)).unwrap().is_none());
    let reject_id=uuid::Uuid::new_v4().to_string();
    let mut rejected=outgoing;
    rejected.to="reject@example.invalid".into();
    rejected.bcc=String::new();
    rejected.save_sent_copy=false;
    store.enqueue_mail(reject_id.clone(),rejected,0,None).unwrap();
    assert_eq!(store.dispatch_outbox(Some(reject_id.clone())).unwrap().unwrap().state,"uncertain");
    assert!(store.dispatch_outbox(Some(reject_id)).unwrap().is_none());
    let sent=store.sync_account(&account.id,Some("Sent".into()),false).unwrap();
    assert_eq!(sent.added,1);
    let sent_list=store.messages(None,Some("Sent".into()),false,false,"",0).unwrap();
    assert_eq!(sent_list.len(),1);
    let sent_id=sent_list[0].id;
    assert_eq!(store.attachment_download(sent_id,0).unwrap().1,payload);
    assert_eq!(store.message(sent_id).unwrap().html.as_deref().map(str::trim),Some("<p><strong>Local test only.</strong></p>"));
    let archive=store.messages(None,Some("Archive".into()),false,true,"",0).unwrap();
    let archived_id=archive[0].id;
    let ids=vec![archived_id,sent_id];
    let report=store.batch(ids.clone(),"unread",None).unwrap();
    assert_eq!(report.succeeded,ids);
    assert!(report.failed.is_empty());
    assert!(!store.message(archived_id).unwrap().summary.seen);
    assert!(!store.message(sent_id).unwrap().summary.seen);
    let report=store.batch(ids.clone(),"star",None).unwrap();
    assert_eq!(report.succeeded,ids);
    assert!(store.message(sent_id).unwrap().summary.starred);
    let report=store.batch(vec![archived_id],"trash",None).unwrap();
    assert_eq!(report.succeeded,vec![archived_id]);
    store.sync_account(&account.id,Some("Trash".into()),false).unwrap();
    assert_eq!(store.messages(None,Some("Trash".into()),false,true,"Fixture message",0).unwrap().len(),1);
}

#[test]
#[ignore = "requires the loopback TLS protocol fixture"]
fn tls_pop3_and_caldav_roundtrip(){
    let pop_port=std::env::var("MAIL_EX_FIXTURE_POP_PORT").unwrap().parse().unwrap();
    let smtp_port=std::env::var("MAIL_EX_FIXTURE_SMTP_PORT").unwrap().parse().unwrap();
    let cal_port=std::env::var("MAIL_EX_FIXTURE_CAL_PORT").unwrap();
    let dir=tempfile::tempdir().unwrap();let mut store=Store::open(dir.path()).unwrap();
    let account=store.add_pop_account(NewAccount{name:"POP fixture".into(),email:"fixture@example.invalid".into(),username:"fixture@example.invalid".into(),password:"fixture-only-secret".into(),imap_host:"localhost".into(),imap_port:pop_port,smtp_host:"localhost".into(),smtp_port,smtp_starttls:false,color:"#123456".into(),signature:String::new()}).unwrap();
    assert_eq!(account.auth_kind,"pop3");
    store.test_account(&account.id).unwrap();
    assert_eq!(store.sync_account(&account.id,None,false).unwrap().added,1);
    assert_eq!(store.sync_account(&account.id,None,false).unwrap().added,0);
    assert_eq!(store.sync_account(&account.id,None,false).unwrap().added,0);
    let list=store.messages(Some(account.id.clone()),Some("INBOX".into()),false,false,"",0).unwrap();
    assert_eq!(list.len(),1);
    let id=list[0].id;
    assert!(store.message(id).unwrap().body.contains(".dot fixture"));
    store.set_seen(id,true).unwrap();store.set_starred(id,true).unwrap();
    assert!(store.message(id).unwrap().summary.seen&&store.message(id).unwrap().summary.starred);
    assert!(store.move_message(id,"Sent").is_err());
    assert_eq!(store.cache_preview(Some(account.id.clone()),None).unwrap().messages,0);
    let invitations=store.message_invitations(id).unwrap();assert_eq!(invitations.len(),1);
    assert_eq!(invitations[0].event.summary,"Fixture meeting");
    assert_eq!(store.add_invitation(id,0).unwrap().added,1);
    assert_eq!(store.add_invitation(id,0).unwrap().skipped,1);
    for state in ["ACCEPTED","TENTATIVE","DECLINED"]{
        let reply=store.invitation_reply(id,0,&account.id,state).unwrap();
        assert_eq!(reply.to,"host@example.invalid");
        assert!(reply.attachments[0].media_type.contains("method=REPLY"));
        let raw=String::from_utf8(STANDARD.decode(&reply.attachments[0].data).unwrap()).unwrap();
        assert!(raw.contains(&format!("PARTSTAT={state}:mailto:fixture@example.invalid")));
        assert!(raw.contains("UID:fixture-meeting@example.invalid"));
    }
    let event=store.save_calendar_event(CalendarEvent{summary:"Local invite".into(),start:"2026-12-02T10:00:00Z".into(),end:"2026-12-02T11:00:00Z".into(),..Default::default()}).unwrap();
    let invite=store.meeting_invitation(&event.id,&account.id,"guest@example.invalid").unwrap();
    let raw=String::from_utf8(STANDARD.decode(&invite.attachments[0].data).unwrap()).unwrap();
    assert!(raw.contains("METHOD:REQUEST")&&raw.contains("mailto:guest@example.invalid"));
    let source=store.add_calendar_source(NewCalendarSource{name:"Calendar fixture".into(),url:format!("https://localhost:{cal_port}/calendar/"),username:"fixture@example.invalid".into(),password:"fixture-only-secret".into()}).unwrap();
    let report=store.sync_calendar(&source.id).unwrap();assert_eq!(report.events,1);assert_eq!(report.resources,1);
    let remote=store.calendar_events().unwrap().into_iter().find(|e|e.source_id.as_deref()==Some(&source.id)).unwrap();
    assert_eq!(remote.summary,"Fixture meeting");
    assert!(store.save_calendar_event(remote.clone()).is_err());
    assert!(store.sync_calendar(&source.id).is_err());
    assert!(store.calendar_events().unwrap().iter().any(|e|e.id==remote.id));
    store.remove_calendar_source(&source.id).unwrap();
    assert!(store.calendar_events().unwrap().iter().all(|e|e.source_id.is_none()));
    let result=store.send(Compose{account_id:account.id.clone(),to:"pop-receiver@example.invalid".into(),cc:String::new(),bcc:String::new(),subject:"POP fixture sent".into(),body:"Loopback POP3 Sent copy".into(),in_reply_to:None,references:None,html:None,attachments:Vec::new(),save_sent_copy:true}).unwrap();
    assert_eq!(result.sent_copy,"saved");
    assert_eq!(store.messages(Some(account.id.clone()),Some("Sent".into()),false,false,"",0).unwrap().len(),1);
}
