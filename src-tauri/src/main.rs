#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use mail_ex_core::{Account,NewAccount,Store,Summary,Mail,Compose,Folder,Draft,SyncReport,AttachmentPreview,SendReport,OAuthRequest,SearchFilters,BatchReport,Contact,MailTemplate,SavedSearch,MailRule,LabelCount,RuleReport,OutboxItem,ImportReport,StorageStats,CacheReport,SnapshotReport,CalendarEvent,CalendarSource,NewCalendarSource,Invitation,CalendarSyncReport};
use tauri::{AppHandle,Manager,Emitter};
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
static OAUTH:Mutex<Option<Arc<AtomicBool>>>=Mutex::new(None);
struct OAuthGuard;
impl Drop for OAuthGuard {fn drop(&mut self){if let Ok(mut active)=OAUTH.lock(){*active=None;}}}
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use std::path::{Path,PathBuf};
use std::fs;
use std::io::Write;
struct DataLocation {active:PathBuf,settings:PathBuf}
fn configured_location(settings:&Path,default:&Path)->anyhow::Result<PathBuf>{
    if !settings.exists(){return Ok(default.to_path_buf())}
    let path:PathBuf=serde_json::from_reader(fs::File::open(settings)?)?;
    if !path.is_absolute()||!path.join("mail.sqlite3").is_file()||!path.join("vault.key").is_file(){anyhow::bail!("Configured mail storage is unavailable; restore the directory before starting Mail EX")}
    Ok(path)
}
fn set_next_location(settings:&Path,path:&Path)->anyhow::Result<()>{
    if !path.is_absolute(){anyhow::bail!("Storage directory must be absolute")}
    let tmp=settings.with_extension(format!("tmp-{}",uuid::Uuid::new_v4()));
    let result=(||->anyhow::Result<()>{let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;serde_json::to_writer(&file,path)?;file.flush()?;file.sync_all()?;drop(file);fs::rename(&tmp,settings)?;Ok(())})();
    if result.is_err(){let _=fs::remove_file(&tmp);}result
}
fn picked(path:tauri_plugin_dialog::FilePath)->anyhow::Result<PathBuf>{path.into_path().map_err(|_|anyhow::anyhow!("Unsupported file destination"))}

async fn work<T:Send+'static>(app:AppHandle,f:impl FnOnce(&mut Store)->anyhow::Result<T>+Send+'static)->Result<T,String>{
    let dir=app.state::<DataLocation>().active.clone();
    tauri::async_runtime::spawn_blocking(move||{let mut store=Store::open(&dir).map_err(|e|e.to_string())?;f(&mut store).map_err(|e|e.to_string())}).await.map_err(|_|"Background task failed".to_string())?
}
#[tauri::command]
async fn list_accounts(app:AppHandle)->Result<Vec<Account>,String>{work(app,|s|s.accounts()).await}
#[tauri::command]
async fn add_account(app:AppHandle,account:NewAccount)->Result<Account,String>{work(app,move|s|s.add_account(account)).await}
#[tauri::command]
async fn authorize_account(app:AppHandle,request:OAuthRequest)->Result<Account,String>{
    let cancel=Arc::new(AtomicBool::new(false));
    {
        let mut active=OAUTH.lock().map_err(|_|"OAuth state unavailable".to_string())?;
        if active.is_some(){return Err("An authorization is already in progress".into())}
        *active=Some(cancel.clone());
    }
    let guard=OAuthGuard;
    work(app,move|store|{
        let _guard=guard;
        store.authorize_account(request,&cancel,|url|open::that(url).map_err(anyhow::Error::from))
    }).await
}
#[tauri::command]
fn cancel_oauth()->Result<(),String>{
    let active=OAUTH.lock().map_err(|_|"OAuth state unavailable".to_string())?;
    if let Some(cancel)=active.as_ref(){cancel.store(true,Ordering::Relaxed)}
    Ok(())
}
#[tauri::command]
async fn update_account(app:AppHandle,id:String,account:NewAccount)->Result<Account,String>{work(app,move|s|s.update_account(&id,account)).await}
#[tauri::command]
async fn remove_account(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.remove_account(&id)).await}
#[tauri::command]
async fn list_folders(app:AppHandle)->Result<Vec<Folder>,String>{work(app,|s|s.folders()).await}
#[tauri::command]
async fn test_account(app:AppHandle,id:String)->Result<String,String>{work(app,move|s|s.test_account(&id)).await}
#[tauri::command]
async fn sync_account(app:AppHandle,id:String,folder:Option<String>,all_folders:bool)->Result<SyncReport,String>{
    let handle=app.clone();
    work(app,move|s|{
        let result=s.sync_account(&id,folder,all_folders)?;
        if result.added>0&&s.notification_enabled().unwrap_or(false){
            let _=handle.notification().builder().title("Mail EX").body(format!("{} newly synchronized messages",result.added)).show();
        }
        Ok(result)
    }).await
}
#[tauri::command]
async fn list_messages(app:AppHandle,account_id:Option<String>,folder:Option<String>,unread:bool,starred:bool,query:String,offset:u32,filters:SearchFilters)->Result<Vec<Summary>,String>{work(app,move|s|s.search_messages(account_id,folder,unread,starred,&query,offset,filters)).await}
#[tauri::command]
async fn get_message(app:AppHandle,id:i64)->Result<Mail,String>{work(app,move|s|s.message(id)).await}
#[tauri::command]
async fn set_seen(app:AppHandle,id:i64,seen:bool)->Result<(),String>{work(app,move|s|s.set_seen(id,seen)).await}
#[tauri::command]
async fn set_starred(app:AppHandle,id:i64,starred:bool)->Result<(),String>{work(app,move|s|s.set_starred(id,starred)).await}
#[tauri::command]
async fn move_message(app:AppHandle,id:i64,destination:String)->Result<(),String>{work(app,move|s|s.move_message(id,&destination)).await}
#[tauri::command]
async fn batch_messages(app:AppHandle,ids:Vec<i64>,operation:String,destination:Option<String>)->Result<BatchReport,String>{work(app,move|s|s.batch(ids,&operation,destination)).await}
#[tauri::command]
async fn list_drafts(app:AppHandle)->Result<Vec<Draft>,String>{work(app,|s|s.drafts()).await}
#[tauri::command]
async fn get_draft(app:AppHandle,id:String)->Result<Draft,String>{work(app,move|s|s.draft(&id)).await}
#[tauri::command]
async fn attachment_preview(app:AppHandle,id:i64,index:usize)->Result<AttachmentPreview,String>{work(app,move|s|s.attachment_preview(id,index)).await}
#[tauri::command]
async fn attachment_save(app:AppHandle,id:i64,index:usize)->Result<bool,String>{
    let (name,bytes)=work(app.clone(),move|s|s.attachment_download(id,index)).await?;
    tauri::async_runtime::spawn_blocking(move||{
        let Some(destination)=app.dialog().file().set_file_name(&name).blocking_save_file() else{return Ok(false)};
        let path=destination.into_path().map_err(|_|"Unsupported save destination".to_string())?;
        std::fs::write(path,bytes).map_err(|_|"Cannot write the selected attachment file".to_string())?;
        Ok(true)
    }).await.map_err(|_|"Attachment save task failed".to_string())?
}
#[tauri::command]
async fn save_draft(app:AppHandle,id:Option<String>,mail:Compose)->Result<String,String>{work(app,move|s|s.save_draft(id,mail)).await}
#[tauri::command]
async fn delete_draft(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_draft(&id)).await}
#[tauri::command]
async fn compose_from(app:AppHandle,id:i64,mode:String)->Result<Compose,String>{work(app,move|s|s.compose_from(id,&mode)).await}
#[tauri::command]
async fn send_mail(app:AppHandle,mail:Compose)->Result<SendReport,String>{work(app,move|s|s.send(mail)).await}
#[tauri::command]
async fn list_contacts(app:AppHandle)->Result<Vec<Contact>,String>{work(app,move|s|s.contacts()).await}
#[tauri::command]
async fn save_contact(app:AppHandle,contact:Contact)->Result<Contact,String>{work(app,move|s|s.save_contact(contact)).await}
#[tauri::command]
async fn delete_contact(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_contact(&id)).await}
#[tauri::command]
async fn list_templates(app:AppHandle)->Result<Vec<MailTemplate>,String>{work(app,move|s|s.templates()).await}
#[tauri::command]
async fn save_template(app:AppHandle,template:MailTemplate)->Result<MailTemplate,String>{work(app,move|s|s.save_template(template)).await}
#[tauri::command]
async fn delete_template(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_template(&id)).await}
#[tauri::command]
async fn list_saved_searches(app:AppHandle)->Result<Vec<SavedSearch>,String>{work(app,move|s|s.saved_searches()).await}
#[tauri::command]
async fn save_search(app:AppHandle,search:SavedSearch)->Result<SavedSearch,String>{work(app,move|s|s.save_search(search)).await}
#[tauri::command]
async fn delete_search(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_search(&id)).await}
#[tauri::command]
async fn list_rules(app:AppHandle)->Result<Vec<MailRule>,String>{work(app,move|s|s.rules()).await}
#[tauri::command]
async fn save_rule(app:AppHandle,rule:MailRule)->Result<MailRule,String>{work(app,move|s|s.save_rule(rule)).await}
#[tauri::command]
async fn delete_rule(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_rule(&id)).await}
#[tauri::command]
async fn preview_rule(app:AppHandle,rule:MailRule)->Result<usize,String>{work(app,move|s|s.preview_rule(rule)).await}
#[tauri::command]
async fn apply_rule(app:AppHandle,id:String)->Result<RuleReport,String>{work(app,move|s|s.apply_rule(&id)).await}
#[tauri::command]
async fn list_labels(app:AppHandle)->Result<Vec<LabelCount>,String>{work(app,move|s|s.labels()).await}
#[tauri::command]
async fn message_labels(app:AppHandle,id:i64)->Result<Vec<String>,String>{work(app,move|s|s.message_labels(id)).await}
#[tauri::command]
async fn list_outbox(app:AppHandle)->Result<Vec<OutboxItem>,String>{work(app,move|s|s.outbox()).await}
#[tauri::command]
async fn enqueue_mail(app:AppHandle,request_id:String,mail:Compose,due_at:i64,draft_id:Option<String>)->Result<OutboxItem,String>{work(app,move|s|s.enqueue_mail(request_id,mail,due_at,draft_id)).await}
#[tauri::command]
async fn dispatch_outbox(app:AppHandle,id:Option<String>)->Result<Option<OutboxItem>,String>{work(app,move|s|s.dispatch_outbox(id)).await}
#[tauri::command]
async fn cancel_outbox(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.cancel_outbox(&id)).await}
#[tauri::command]
async fn mark_interrupted_outbox(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.mark_interrupted_outbox(&id)).await}
#[tauri::command]
async fn copy_outbox(app:AppHandle,id:String)->Result<Compose,String>{work(app,move|s|s.copy_outbox(&id)).await}
#[tauri::command]
async fn delete_outbox(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_outbox(&id)).await}

#[tauri::command]
async fn storage_stats(app:AppHandle)->Result<StorageStats,String>{work(app,|s|s.storage_stats()).await}
#[tauri::command]
fn pending_storage(app:AppHandle)->Result<Option<String>,String>{
    let state=app.state::<DataLocation>();
    if !state.settings.exists(){return Ok(None)}
    let path:PathBuf=serde_json::from_reader(fs::File::open(&state.settings).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(if path==state.active{None}else{Some(path.to_string_lossy().into_owned())})
}
#[tauri::command]
async fn import_mail_files(app:AppHandle,format:String)->Result<Option<ImportReport>,String>{
    let handle=app.clone();
    work(app,move|s|{
        if !["eml","mbox"].contains(&format.as_str()){anyhow::bail!("Unsupported mail format")}
        let Some(paths)=handle.dialog().file().add_filter("Mail files",&[format.as_str()]).blocking_pick_files() else{return Ok(None)};
        let paths=paths.into_iter().map(picked).collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Some(s.import_mail(&paths,&format)?))
    }).await
}
#[tauri::command]
async fn export_mail_eml(app:AppHandle,id:i64)->Result<bool,String>{
    let handle=app.clone();
    work(app,move|s|{
        let Some(path)=handle.dialog().file().set_file_name("message.eml").add_filter("EML",&["eml"]).blocking_save_file() else{return Ok(false)};
        s.export_eml(id,&picked(path)?)?;Ok(true)
    }).await
}
#[tauri::command]
async fn export_mail_mbox(app:AppHandle,account_id:Option<String>,folder:Option<String>)->Result<Option<usize>,String>{
    let handle=app.clone();
    work(app,move|s|{
        let Some(path)=handle.dialog().file().set_file_name("mail.mbox").add_filter("MBOX",&["mbox"]).blocking_save_file() else{return Ok(None)};
        Ok(Some(s.export_mbox(account_id,folder,&picked(path)?)?))
    }).await
}
#[tauri::command]
async fn delete_local_mail(app:AppHandle,id:i64)->Result<(),String>{work(app,move|s|s.delete_import(id)).await}
#[tauri::command]
async fn import_contacts_csv(app:AppHandle)->Result<Option<ImportReport>,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().add_filter("CSV",&["csv"]).blocking_pick_file()else{return Ok(None)};Ok(Some(s.import_contacts(&picked(path)?)?))}).await
}
#[tauri::command]
async fn export_contacts_csv(app:AppHandle)->Result<Option<usize>,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().set_file_name("contacts.csv").add_filter("CSV",&["csv"]).blocking_save_file()else{return Ok(None)};Ok(Some(s.export_contacts(&picked(path)?)?))}).await
}
#[tauri::command]
async fn import_accounts_json(app:AppHandle)->Result<Option<ImportReport>,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().add_filter("JSON",&["json"]).blocking_pick_file()else{return Ok(None)};Ok(Some(s.import_account_config(&picked(path)?)?))}).await
}
#[tauri::command]
async fn export_accounts_json(app:AppHandle)->Result<Option<usize>,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().set_file_name("mail-ex-accounts.json").add_filter("JSON",&["json"]).blocking_save_file()else{return Ok(None)};Ok(Some(s.export_account_config(&picked(path)?)?))}).await
}
#[tauri::command]
async fn cache_preview(app:AppHandle,account_id:Option<String>,before:Option<i64>)->Result<CacheReport,String>{work(app,move|s|s.cache_preview(account_id,before)).await}
#[tauri::command]
async fn clear_cache(app:AppHandle,account_id:Option<String>,before:Option<i64>)->Result<CacheReport,String>{work(app,move|s|s.clear_cache(account_id,before)).await}
#[tauri::command]
async fn optimize_storage(app:AppHandle)->Result<(),String>{work(app,|s|s.optimize_storage()).await}
#[tauri::command]
async fn create_backup(app:AppHandle)->Result<Option<SnapshotReport>,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().set_title("Choose backup parent directory").blocking_pick_folder()else{return Ok(None)};Ok(Some(s.create_backup(&picked(path)?)?))}).await
}
#[tauri::command]
async fn restore_backup(app:AppHandle)->Result<Option<SnapshotReport>,String>{
    let handle=app.clone();let settings=app.state::<DataLocation>().settings.clone();
    work(app,move|s|{
        let Some(source)=handle.dialog().file().set_title("Choose trusted Mail EX backup folder").blocking_pick_folder()else{return Ok(None)};
        let Some(parent)=handle.dialog().file().set_title("Choose parent directory for restored data").blocking_pick_folder()else{return Ok(None)};
        let result=s.restore_backup(&picked(source)?,&picked(parent)?)?;
        set_next_location(&settings,Path::new(&result.directory))?;Ok(Some(result))
    }).await
}
#[tauri::command]
async fn copy_storage(app:AppHandle)->Result<Option<SnapshotReport>,String>{
    let handle=app.clone();let settings=app.state::<DataLocation>().settings.clone();
    work(app,move|s|{
        let Some(parent)=handle.dialog().file().set_title("Choose parent directory for copied data").blocking_pick_folder()else{return Ok(None)};
        let result=s.copy_storage(&picked(parent)?)?;
        set_next_location(&settings,Path::new(&result.directory))?;Ok(Some(result))
    }).await
}


#[tauri::command]
async fn add_pop_account(app:AppHandle,account:NewAccount)->Result<Account,String>{work(app,move|s|s.add_pop_account(account)).await}
#[tauri::command]
async fn list_calendar_events(app:AppHandle)->Result<Vec<CalendarEvent>,String>{work(app,|s|s.calendar_events()).await}
#[tauri::command]
async fn save_calendar_event(app:AppHandle,event:CalendarEvent)->Result<CalendarEvent,String>{work(app,move|s|s.save_calendar_event(event)).await}
#[tauri::command]
async fn delete_calendar_event(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.delete_calendar_event(&id)).await}
#[tauri::command]
async fn list_calendar_sources(app:AppHandle)->Result<Vec<CalendarSource>,String>{work(app,|s|s.calendar_sources()).await}
#[tauri::command]
async fn add_calendar_source(app:AppHandle,source:NewCalendarSource)->Result<CalendarSource,String>{work(app,move|s|s.add_calendar_source(source)).await}
#[tauri::command]
async fn remove_calendar_source(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.remove_calendar_source(&id)).await}
#[tauri::command]
async fn sync_calendar(app:AppHandle,id:String)->Result<CalendarSyncReport,String>{work(app,move|s|s.sync_calendar(&id)).await}
#[tauri::command]
async fn import_calendar_ics(app:AppHandle)->Result<Option<ImportReport>,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().add_filter("iCalendar",&["ics"]).blocking_pick_file()else{return Ok(None)};Ok(Some(s.import_calendar_file(&picked(path)?)?))}).await
}
#[tauri::command]
async fn export_calendar_ics(app:AppHandle,id:String)->Result<bool,String>{
    let handle=app.clone();
    work(app,move|s|{let Some(path)=handle.dialog().file().set_file_name("event.ics").add_filter("iCalendar",&["ics"]).blocking_save_file()else{return Ok(false)};s.export_calendar_event(&id,&picked(path)?)?;Ok(true)}).await
}
#[tauri::command]
async fn message_invitations(app:AppHandle,id:i64)->Result<Vec<Invitation>,String>{work(app,move|s|s.message_invitations(id)).await}
#[tauri::command]
async fn add_invitation(app:AppHandle,id:i64,index:usize)->Result<ImportReport,String>{work(app,move|s|s.add_invitation(id,index)).await}
#[tauri::command]
async fn invitation_reply(app:AppHandle,id:i64,index:usize,account_id:String,partstat:String)->Result<Compose,String>{work(app,move|s|s.invitation_reply(id,index,&account_id,&partstat)).await}
#[tauri::command]
async fn meeting_invitation(app:AppHandle,id:String,account_id:String,recipients:String)->Result<Compose,String>{work(app,move|s|s.meeting_invitation(&id,&account_id,&recipients)).await}
#[tauri::command]
async fn notification_enabled(app:AppHandle)->Result<bool,String>{work(app,|s|s.notification_enabled()).await}
#[tauri::command]
async fn set_notifications(app:AppHandle,enabled:bool)->Result<(),String>{
    let handle=app.clone();
    work(app,move|s|{
        use tauri::plugin::PermissionState;
        if enabled{
            let state=handle.notification().permission_state()?;
            if state!=PermissionState::Granted && handle.notification().request_permission()?!=PermissionState::Granted{anyhow::bail!("System notification permission was not granted")}
        }
        s.set_notifications(enabled)
    }).await
}

fn main(){
    tauri::Builder::default().plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_notification::init()).setup(|app| {
        let root=app.path().app_data_dir()?;
        fs::create_dir_all(&root)?;
        let settings=root.join("storage-location.json");
        let dir=configured_location(&settings,&root)?;
        Store::open(&dir)?;
        app.manage(DataLocation{active:dir.clone(),settings});
        let reminder_dir=dir.clone();
        let reminder_handle=app.handle().clone();
        std::thread::spawn(move||loop {
            let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
            if let Ok(count)=Store::open(&reminder_dir).and_then(|mut store|store.claim_reminders(now)){
                if count>0{
                    let _=reminder_handle.notification().builder().title("Mail EX").body(format!("{count} calendar reminder(s)")).show();
                    let _=reminder_handle.emit("calendar-reminder",count);
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(30));
        });
        let handle=app.handle().clone();
        std::thread::spawn(move||loop {
            let result=Store::open(&dir).and_then(|mut store|store.dispatch_outbox(None));
            if let Ok(Some(item))=result { let _=handle.emit("outbox-changed",item.id); }
            std::thread::sleep(std::time::Duration::from_secs(5));
        });
        Ok(())
    }).invoke_handler(tauri::generate_handler![list_accounts,add_account,authorize_account,cancel_oauth,update_account,remove_account,list_folders,test_account,sync_account,list_messages,get_message,set_seen,set_starred,move_message,batch_messages,list_drafts,get_draft,save_draft,delete_draft,compose_from,attachment_preview,attachment_save,send_mail,list_contacts,save_contact,delete_contact,list_templates,save_template,delete_template,list_saved_searches,save_search,delete_search,list_rules,save_rule,delete_rule,preview_rule,apply_rule,list_labels,message_labels,list_outbox,enqueue_mail,dispatch_outbox,cancel_outbox,mark_interrupted_outbox,copy_outbox,delete_outbox,storage_stats,pending_storage,import_mail_files,export_mail_eml,export_mail_mbox,delete_local_mail,import_contacts_csv,export_contacts_csv,import_accounts_json,export_accounts_json,cache_preview,clear_cache,optimize_storage,create_backup,restore_backup,copy_storage,add_pop_account,list_calendar_events,save_calendar_event,delete_calendar_event,list_calendar_sources,add_calendar_source,remove_calendar_source,sync_calendar,import_calendar_ics,export_calendar_ics,message_invitations,add_invitation,invitation_reply,meeting_invitation,notification_enabled,set_notifications]).run(tauri::generate_context!()).expect("Mail EX desktop startup failed");
}
