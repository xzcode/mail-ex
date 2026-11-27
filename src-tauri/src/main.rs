#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use mail_ex_core::{Account,NewAccount,Store,Summary,Mail,Compose,Folder,Draft,SyncReport,AttachmentPreview,SendReport,OAuthRequest,SearchFilters,BatchReport};
use tauri::{AppHandle,Manager};
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}};
static OAUTH:Mutex<Option<Arc<AtomicBool>>>=Mutex::new(None);
struct OAuthGuard;
impl Drop for OAuthGuard {fn drop(&mut self){if let Ok(mut active)=OAUTH.lock(){*active=None;}}}
use tauri_plugin_dialog::DialogExt;

async fn work<T:Send+'static>(app:AppHandle,f:impl FnOnce(&mut Store)->anyhow::Result<T>+Send+'static)->Result<T,String>{
    let dir=app.path().app_data_dir().map_err(|_|"Cannot resolve app data directory".to_string())?;
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
async fn sync_account(app:AppHandle,id:String,folder:Option<String>,all_folders:bool)->Result<SyncReport,String>{work(app,move|s|s.sync_account(&id,folder,all_folders)).await}
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
fn main(){
    tauri::Builder::default().plugin(tauri_plugin_dialog::init()).setup(|app| {
        let dir=app.path().app_data_dir()?;
        Store::open(&dir)?;
        Ok(())
    }).invoke_handler(tauri::generate_handler![list_accounts,add_account,authorize_account,cancel_oauth,update_account,remove_account,list_folders,test_account,sync_account,list_messages,get_message,set_seen,set_starred,move_message,batch_messages,list_drafts,get_draft,save_draft,delete_draft,compose_from,attachment_preview,attachment_save,send_mail]).run(tauri::generate_context!()).expect("Mail EX desktop startup failed");
}
