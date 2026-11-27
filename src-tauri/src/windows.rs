//! Dedicated local webviews. The compose window is keyed by the saved draft UUID;
//! readers get immutable cached snapshots so a reused SQLite row ID cannot retarget them.
use super::{work,AppHandle,Manager,WebviewWindow,WebviewWindowBuilder,WebviewUrl,Mail,Invitation};
use serde::Serialize;
use std::{collections::HashMap,sync::Mutex};
use tauri::WindowEvent;

#[derive(Serialize)]
pub struct ReaderSnapshot{pub mail:Mail,pub invitations:Vec<Invitation>,pub labels:Vec<String>}
#[derive(Default)]
pub struct ReaderWindows(pub Mutex<HashMap<String,ReaderSnapshot>>);

fn bring_to_front(window:&WebviewWindow)->Result<(),String>{
    window.show().map_err(|e|e.to_string())?;
    let _=window.unminimize();
    let _=window.set_focus();Ok(())
}
#[tauri::command]
pub async fn open_compose_window(app:AppHandle,id:String)->Result<(),String>{
    let uuid=uuid::Uuid::parse_str(&id).map_err(|_|"Invalid draft identifier".to_string())?;
    if uuid.to_string()!=id{return Err("Invalid draft identifier".into())}
    let label=format!("compose-{id}");
    if let Some(window)=app.get_webview_window(&label){return bring_to_front(&window)}
    work(app.clone(),move|s|{s.draft(&id)?;Ok(())}).await?;
    match WebviewWindowBuilder::new(&app,&label,WebviewUrl::App("index.html".into()))
        .title("Mail EX — Compose").center().inner_size(880.0,760.0).min_inner_size(600.0,500.0).build(){
        Ok(window)=>bring_to_front(&window),
        Err(e)=>{if let Some(window)=app.get_webview_window(&label){bring_to_front(&window)}else{Err(e.to_string())}}
    }
}
// The caller is supplied by Tauri, not by a user-controlled window label.
#[tauri::command]
pub async fn close_compose_window(window:WebviewWindow)->Result<(),String>{
    let id=window.label().strip_prefix("compose-").ok_or_else(||"Only a compose window may close itself".to_string())?;
    let uuid=uuid::Uuid::parse_str(id).map_err(|_|"Invalid compose window label".to_string())?;
    if uuid.to_string()!=id{return Err("Invalid compose window label".into())}
    window.destroy().map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn open_reader_window(app:AppHandle,id:i64)->Result<(),String>{
    let snapshot=work(app.clone(),move|s|Ok(ReaderSnapshot{mail:s.message(id)?,invitations:s.message_invitations(id).unwrap_or_default(),labels:s.message_labels(id)?})).await?;
    let label=format!("reader-{}",uuid::Uuid::new_v4());
    {
        app.state::<ReaderWindows>().0.lock().map_err(|_|"Reader snapshot unavailable".to_string())?.insert(label.clone(),snapshot);
    }
    let created=WebviewWindowBuilder::new(&app,&label,WebviewUrl::App("index.html".into()))
        .title("Mail EX — Message").inner_size(900.0,760.0).min_inner_size(500.0,400.0).build();
    match created{
        Ok(window)=>{
            let handle=app.clone();let key=label.clone();
            window.on_window_event(move|event|if matches!(event,WindowEvent::Destroyed){
                if let Ok(mut state)=handle.state::<ReaderWindows>().0.lock(){state.remove(&key);}
            });
            bring_to_front(&window)
        },
        Err(e)=>{if let Ok(mut state)=app.state::<ReaderWindows>().0.lock(){state.remove(&label);}Err(e.to_string())}
    }
}
// Take the snapshot once, after the reader webview has mounted. The reader
// intentionally exposes no server-side mutation or live attachment command.
#[tauri::command]
pub fn reader_snapshot(window:WebviewWindow,app:AppHandle)->Result<ReaderSnapshot,String>{
    if !window.label().starts_with("reader-"){return Err("Not a reader window".into())}
    app.state::<ReaderWindows>().0.lock().map_err(|_|"Reader snapshot unavailable".to_string())?.remove(window.label()).ok_or_else(||"Reader snapshot expired; reopen from the main mail list".into())
}
// The debug-only fixture is seeded in an entirely separate temporary data
// directory. No real account, storage path or queued delivery is involved.
#[cfg(debug_assertions)]
pub struct SmokeTargets(pub Mutex<Option<(String,i64)>>);
#[tauri::command]
pub fn frontend_ready(window:WebviewWindow,app:AppHandle){
    // Harmless startup marker for an isolated release-profile smoke test.
    // No fixture injection or privileged behavior is enabled in release.
    if window.label()=="main"&&std::env::var_os("MAIL_EX_RELEASE_SMOKE").is_some(){println!("MAIL_EX_RELEASE_FRONTEND_READY main");}
    #[cfg(debug_assertions)]
    if std::env::var_os("MAIL_EX_SMOKE").is_some(){
        println!("MAIL_EX_FRONTEND_READY {}",window.label());
        if window.label()=="main"{
            let targets=app.state::<SmokeTargets>().0.lock().ok().and_then(|mut state|state.take());
            if let Some((draft,message))=targets{
                tauri::async_runtime::spawn(async move{
                    if let Err(e)=open_compose_window(app.clone(),draft).await{eprintln!("MAIL_EX_SMOKE_COMPOSE_ERROR {e}")}
                    if let Err(e)=open_reader_window(app,message).await{eprintln!("MAIL_EX_SMOKE_READER_ERROR {e}")}
                });
            }
        }
    }
    #[cfg(not(debug_assertions))]
    let _=(window,app);
}
