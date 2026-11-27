//! Native desktop lifecycle. Hiding the main window is opt-in; never silently
//! discard an editor or interrupt a claimed outbound delivery.
use super::{AppHandle,Manager,Emitter,Store};
use std::sync::{Mutex,atomic::{AtomicBool,Ordering}};
use tauri::{menu::{Menu,MenuItem},tray::{TrayIconBuilder,TrayIconEvent,MouseButton,MouseButtonState},WebviewWindow};
#[derive(Default)]
pub struct DeliveryGate{lock:Mutex<()>,quitting:AtomicBool}
impl DeliveryGate{
    pub fn dispatch(&self,store:&mut Store,id:Option<String>)->anyhow::Result<Option<mail_ex_core::OutboxItem>>{
        let Ok(_guard)=self.lock.try_lock() else{return Ok(None)};
        if self.quitting.load(Ordering::Acquire){return Ok(None)}
        store.dispatch_outbox(id)
    }
}
fn show_main(app:&AppHandle){
    if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.unminimize();let _=w.set_focus();}
}
pub fn install(app:&AppHandle)->tauri::Result<()>{
    let show=MenuItem::with_id(app,"mail-ex-show","Open Mail EX",true,None::<&str>)?;
    let sync=MenuItem::with_id(app,"mail-ex-sync","Sync inboxes",true,None::<&str>)?;
    let quit=MenuItem::with_id(app,"mail-ex-quit","Quit Mail EX",true,None::<&str>)?;
    let menu=Menu::with_items(app,&[&show,&sync,&quit])?;
    let mut builder=TrayIconBuilder::with_id("mail-ex-tray").menu(&menu).tooltip("Mail EX").show_menu_on_left_click(false)
        .on_menu_event(|app,event|match event.id().as_ref(){
            "mail-ex-show"=>show_main(app),
            "mail-ex-sync"=>{show_main(app);let _=app.emit_to("main","tray-sync",());},
            "mail-ex-quit"=>{show_main(app);let _=app.emit_to("main","tray-quit",());},
            _=>{}
        })
        .on_tray_icon_event(|tray,event|{
            if let TrayIconEvent::Click{button:MouseButton::Left,button_state:MouseButtonState::Up,..}=event{show_main(tray.app_handle())}
        });
    if let Some(icon)=app.default_window_icon(){builder=builder.icon(icon.clone());}
    builder.build(app)?;Ok(())
}
#[tauri::command]
pub fn tray_available(app:AppHandle)->bool{app.tray_by_id("mail-ex-tray").is_some()}
#[tauri::command]
pub async fn hide_main_to_tray(window:WebviewWindow,app:AppHandle)->Result<(),String>{
    if window.label()!="main"{return Err("Only the main window may be hidden to tray".into())}
    if app.tray_by_id("mail-ex-tray").is_none(){return Err("System tray is unavailable".into())}
    // A Linux appindicator can be created even if the session has no status
    // notifier host. Do not hide the only way back into the application.
    #[cfg(target_os="linux")]
    {
        let result=std::process::Command::new("gdbus").args(["call","--session","--timeout","2","--dest","org.freedesktop.DBus","--object-path","/org/freedesktop/DBus","--method","org.freedesktop.DBus.NameHasOwner","org.kde.StatusNotifierWatcher"]).output();
        if !result.is_ok_and(|r|r.status.success()&&String::from_utf8_lossy(&r.stdout).contains("true")){
            return Err("No Linux status notifier host is available; the main window stays open".into())
        }
    }
    window.hide().map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn quit_desktop(window:WebviewWindow,app:AppHandle)->Result<(),String>{
    if window.label()!="main"{return Err("Quit from the main window".into())}
    if app.webview_windows().keys().any(|label|label.starts_with("compose-")){return Err("Close all compose windows before quitting Mail EX".into())}
    tauri::async_runtime::spawn_blocking(move||{
        let gate=app.state::<DeliveryGate>();
        let _guard=gate.lock.try_lock().map_err(|_|"Mail delivery is in progress; wait for its result before quitting".to_string())?;
        gate.quitting.store(true,Ordering::Release);
        // The gate blocks new scheduler/manual claims until process exit.
        // Stale interrupted records remain uncertain; no automatic retry.
        app.exit(0);Ok(())
    }).await.map_err(|_|"Cannot finish desktop quit".to_string())?
}
