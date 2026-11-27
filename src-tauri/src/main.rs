#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use mail_ex_core::{Account,NewAccount,Store,Summary,Mail,Compose};
use tauri::{AppHandle,Manager};

async fn work<T:Send+'static>(app:AppHandle,f:impl FnOnce(&mut Store)->anyhow::Result<T>+Send+'static)->Result<T,String>{
    let dir=app.path().app_data_dir().map_err(|_|"Cannot resolve app data directory".to_string())?;
    tauri::async_runtime::spawn_blocking(move||{let mut store=Store::open(&dir).map_err(|e|e.to_string())?;f(&mut store).map_err(|e|e.to_string())}).await.map_err(|_|"Background task failed".to_string())?
}
#[tauri::command]
async fn list_accounts(app:AppHandle)->Result<Vec<Account>,String>{work(app,|s|s.accounts()).await}
#[tauri::command]
async fn add_account(app:AppHandle,account:NewAccount)->Result<Account,String>{work(app,move|s|s.add_account(account)).await}
#[tauri::command]
async fn remove_account(app:AppHandle,id:String)->Result<(),String>{work(app,move|s|s.remove_account(&id)).await}
#[tauri::command]
async fn test_account(app:AppHandle,id:String)->Result<String,String>{work(app,move|s|s.test_account(&id)).await}
#[tauri::command]
async fn sync_account(app:AppHandle,id:String)->Result<usize,String>{work(app,move|s|s.sync_account(&id)).await}
#[tauri::command]
async fn list_messages(app:AppHandle,account_id:Option<String>,unread:bool,query:String,offset:u32)->Result<Vec<Summary>,String>{work(app,move|s|s.messages(account_id,unread,&query,offset)).await}
#[tauri::command]
async fn get_message(app:AppHandle,id:i64)->Result<Mail,String>{work(app,move|s|s.message(id)).await}
#[tauri::command]
async fn set_seen(app:AppHandle,id:i64,seen:bool)->Result<(),String>{work(app,move|s|s.set_seen(id,seen)).await}
#[tauri::command]
async fn send_mail(app:AppHandle,mail:Compose)->Result<(),String>{work(app,move|s|s.send(mail)).await}
fn main(){
    tauri::Builder::default().invoke_handler(tauri::generate_handler![list_accounts,add_account,remove_account,test_account,sync_account,list_messages,get_message,set_seen,send_mail]).run(tauri::generate_context!()).expect("Mail EX desktop startup failed");
}
