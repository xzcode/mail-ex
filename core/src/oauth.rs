//! Public desktop OAuth: system browser, loopback callback, PKCE S256 and state.
//! Access/refresh tokens and the optional Google desktop-client registration value
//! are serialized only into the encrypted account secret, never returned to IPC.
use anyhow::{anyhow,bail,Context,Result};
use aes_gcm::aead::{OsRng,rand_core::RngCore};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD,Engine};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{cell::Cell,io::{BufRead,BufReader,Read,Write},net::{TcpListener,TcpStream},sync::atomic::{AtomicBool,Ordering},time::{Duration,Instant,SystemTime,UNIX_EPOCH}};
use url::Url;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Provider{Google,Microsoft,Outlook}
impl Provider{
    pub fn parse(s:&str)->Result<Self>{match s{"google"=>Ok(Self::Google),"microsoft"=>Ok(Self::Microsoft),"outlook"=>Ok(Self::Outlook),_=>bail!("Unsupported OAuth provider")}}
    pub fn id(self)->&'static str{match self{Self::Google=>"google",Self::Microsoft=>"microsoft",Self::Outlook=>"outlook"}}
    fn auth_url(self)->&'static str{match self{Self::Google=>"https://accounts.google.com/o/oauth2/v2/auth",Self::Microsoft|Self::Outlook=>"https://login.microsoftonline.com/common/oauth2/v2.0/authorize"}}
    fn token_url(self)->&'static str{match self{Self::Google=>"https://oauth2.googleapis.com/token",Self::Microsoft|Self::Outlook=>"https://login.microsoftonline.com/common/oauth2/v2.0/token"}}
    pub fn scope(self)->&'static str{match self{Self::Google=>"https://mail.google.com/",Self::Microsoft|Self::Outlook=>"https://outlook.office.com/IMAP.AccessAsUser.All https://outlook.office.com/SMTP.Send offline_access"}}
    pub fn servers(self)->(&'static str,&'static str,u16,bool){match self{Self::Google=>("imap.gmail.com","smtp.gmail.com",465,false),Self::Microsoft=>("outlook.office365.com","smtp.office365.com",587,true),Self::Outlook=>("outlook.office365.com","smtp-mail.outlook.com",587,true)}}
    fn redirect(self,port:u16)->String{match self{Self::Google=>format!("http://127.0.0.1:{port}/"),Self::Microsoft|Self::Outlook=>format!("http://localhost:{port}/")}}
}
#[derive(Deserialize)]
pub struct OAuthRequest{
    pub provider:String,pub client_id:String,
    #[serde(default)] pub desktop_client_secret:String,
    pub email:String,
    #[serde(default)] pub name:String,
    #[serde(default)] pub color:String,
    #[serde(default)] pub account_id:Option<String>,
}
#[derive(Serialize,Deserialize)]
pub(crate) struct Tokens{
    pub(crate) client_id:String,
    pub(crate) desktop_client_secret:String,
    pub(crate) access_token:String,
    pub(crate) refresh_token:String,
    pub(crate) expires_at:u64,
}
#[derive(Deserialize)]
struct TokenResponse{
    access_token:String,
    refresh_token:Option<String>,
    expires_in:u64,
    token_type:Option<String>,
}
impl Tokens{
    pub fn expiring(&self)->bool{self.expires_at<=now().saturating_add(120)}
    fn apply(&mut self,r:TokenResponse)->Result<()>{
        if r.access_token.is_empty()||r.access_token.chars().any(char::is_control)||r.expires_in==0||r.token_type.as_ref().is_some_and(|t|!t.eq_ignore_ascii_case("Bearer")){bail!("Invalid OAuth token response")}
        if let Some(refresh)=r.refresh_token.filter(|s|!s.is_empty()){self.refresh_token=refresh}
        if self.refresh_token.is_empty(){bail!("Provider did not issue a refresh token; grant offline access again")}
        self.access_token=r.access_token;self.expires_at=now().saturating_add(r.expires_in);
        Ok(())
    }
}
fn now()->u64{SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()}
fn random()->String{let mut bytes=[0u8;32];OsRng.fill_bytes(&mut bytes);URL_SAFE_NO_PAD.encode(bytes)}
fn challenge(verifier:&str)->String{URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))}
fn validate_registration(provider:Provider,id:&str,secret:&str)->Result<()>{
    if id.is_empty()||id.chars().any(|c|c.is_whitespace()||c.is_control()){bail!("Enter a registered desktop OAuth client ID")}
    if provider!=Provider::Google&&!secret.is_empty(){bail!("Microsoft desktop public clients must not use a client secret")}
    if secret.chars().any(char::is_control){bail!("Invalid desktop client registration")}
    Ok(())
}
fn authorization_url(provider:Provider,id:&str,email:&str,redirect:&str,state:&str,verifier:&str)->Result<Url>{
    let mut url=Url::parse(provider.auth_url())?;
    url.query_pairs_mut().append_pair("client_id",id).append_pair("response_type","code")
        .append_pair("redirect_uri",redirect).append_pair("scope",provider.scope())
        .append_pair("state",state).append_pair("code_challenge",&challenge(verifier))
        .append_pair("code_challenge_method","S256").append_pair("login_hint",email);
    if provider==Provider::Google{url.query_pairs_mut().append_pair("access_type","offline").append_pair("prompt","consent");}
    else{url.query_pairs_mut().append_pair("response_mode","query").append_pair("prompt","select_account");}
    Ok(url)
}
enum Callback{Ignore,Denied,Code(String)}
fn parse_callback(target:&str,state:&str)->Callback{
    if !target.starts_with("/?"){return Callback::Ignore}
    let Ok(url)=Url::parse(&format!("http://127.0.0.1{target}"))else{return Callback::Ignore};
    let mut state_value=None;let mut code=None;let mut error=None;
    for (k,v) in url.query_pairs(){
        let slot=match k.as_ref(){"state"=>&mut state_value,"code"=>&mut code,"error"=>&mut error,_=>continue};
        if slot.replace(v.into_owned()).is_some(){return Callback::Ignore}
    }
    if state_value.as_deref()!=Some(state){return Callback::Ignore}
    if error.is_some(){return Callback::Denied}
    match code{Some(c) if !c.is_empty()&&!c.chars().any(char::is_control)=>Callback::Code(c),_=>Callback::Ignore}
}
fn respond(stream:&mut TcpStream,ok:bool){
    let (status,body)=if ok{("200 OK","Authorization response received. Return to Mail EX to continue.")}else{("400 Bad Request","This authorization response was not accepted.")};
    let _=write!(stream,"HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",body.len());
}
fn callback(listener:TcpListener,state:&str,cancel:&AtomicBool)->Result<String>{
    listener.set_nonblocking(true)?;
    let deadline=Instant::now()+Duration::from_secs(300);
    while Instant::now()<deadline&&!cancel.load(Ordering::Relaxed){
        match listener.accept(){
            Ok((mut stream,peer))=>{
                if !peer.ip().is_loopback(){continue}
                stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                stream.set_write_timeout(Some(Duration::from_secs(2)))?;
                let mut line=String::new();
                let read=BufReader::new(&mut stream).take(16*1024).read_line(&mut line);
                let mut words=line.split_ascii_whitespace();
                let result=if read.is_ok()&&words.next()==Some("GET"){words.next().map(|s|parse_callback(s,state)).unwrap_or(Callback::Ignore)}else{Callback::Ignore};
                respond(&mut stream,matches!(&result,Callback::Code(_)));
                match result{Callback::Code(c)=>return Ok(c),Callback::Denied=>bail!("OAuth authorization was declined"),Callback::Ignore=>{}}
            }
            Err(e) if e.kind()==std::io::ErrorKind::WouldBlock=>std::thread::sleep(Duration::from_millis(80)),
            Err(_)=>bail!("OAuth callback listener failed"),
        }
    }
    if cancel.load(Ordering::Relaxed){bail!("OAuth authorization canceled")}
    bail!("OAuth authorization timed out; try again")
}
fn post_token(url:&str,form:&[(&str,&str)])->Result<TokenResponse>{
    let client=reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30)).redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_|anyhow!("Cannot initialize OAuth HTTPS client"))?;
    let response=client.post(url).form(form).send().map_err(|_|anyhow!("OAuth token endpoint unreachable"))?;
    if !response.status().is_success(){bail!("OAuth token request rejected; check app registration or authorize again")}
    // Do not include provider error bodies, token strings or HTTP request details in errors.
    response.json().map_err(|_|anyhow!("Invalid OAuth token response"))
}
pub(crate) fn authorize(request:&OAuthRequest,cancel:&AtomicBool,open_browser:impl FnOnce(&str)->Result<()>)->Result<Tokens>{
    let provider=Provider::parse(&request.provider)?;
    let id=request.client_id.trim();
    validate_registration(provider,id,&request.desktop_client_secret)?;
    let listener=TcpListener::bind(("127.0.0.1",0)).context("Cannot open local OAuth callback")?;
    let redirect=provider.redirect(listener.local_addr()?.port());
    let state=random();let verifier=random();
    let url=authorization_url(provider,id,&request.email,&redirect,&state,&verifier)?;
    open_browser(url.as_str()).map_err(|_|anyhow!("Cannot open system browser for OAuth"))?;
    let code=callback(listener,&state,cancel)?;
    if cancel.load(Ordering::Relaxed){bail!("OAuth authorization canceled")}
    let mut form=vec![("client_id",id),("grant_type","authorization_code"),("code",&code),("redirect_uri",&redirect),("code_verifier",&verifier)];
    if provider!=Provider::Google{form.push(("scope",provider.scope()));}
    if provider==Provider::Google&&!request.desktop_client_secret.is_empty(){form.push(("client_secret",&request.desktop_client_secret));}
    let result=post_token(provider.token_url(),&form)?;
    if cancel.load(Ordering::Relaxed){bail!("OAuth authorization canceled")}
    let mut tokens=Tokens{client_id:id.into(),desktop_client_secret:request.desktop_client_secret.clone(),access_token:String::new(),refresh_token:String::new(),expires_at:0};
    tokens.apply(result)?;Ok(tokens)
}
pub(crate) fn refresh(provider:Provider,tokens:&mut Tokens)->Result<()>{
    validate_registration(provider,&tokens.client_id,&tokens.desktop_client_secret)?;
    let mut form=vec![("client_id",tokens.client_id.as_str()),("grant_type","refresh_token"),("refresh_token",tokens.refresh_token.as_str())];
    if provider!=Provider::Google{form.push(("scope",provider.scope()));}
    if provider==Provider::Google&&!tokens.desktop_client_secret.is_empty(){form.push(("client_secret",&tokens.desktop_client_secret));}
    let result=post_token(provider.token_url(),&form)?;
    tokens.apply(result)
}
pub(crate) struct Xoauth2<'a>{user:&'a str,token:&'a str,sent:Cell<bool>}
impl<'a> Xoauth2<'a>{
    pub fn new(user:&'a str,token:&'a str)->Self{Self{user,token,sent:Cell::new(false)}}
}
impl imap::Authenticator for Xoauth2<'_>{
    type Response=Vec<u8>;
    fn process(&self,_:&[u8])->Vec<u8>{
        if self.sent.replace(true){return Vec::new()}
        format!("user={}\x01auth=Bearer {}\x01\x01",self.user,self.token).into_bytes()
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use imap::Authenticator;
    #[test]fn pkce_and_state(){
        assert_eq!(challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let url=authorization_url(Provider::Google,"client","a@example.invalid","http://127.0.0.1:12345/","state","verifier").unwrap();
        assert_eq!(url.host_str(),Some("accounts.google.com"));
        let fields=url.query_pairs().collect::<std::collections::HashMap<_,_>>();
        assert_eq!(fields["code_challenge_method"],"S256");assert_eq!(fields["access_type"],"offline");
        assert!(matches!(parse_callback("/?state=wrong&code=x","state"),Callback::Ignore));
        assert!(matches!(parse_callback("/?state=state&state=state&code=x","state"),Callback::Ignore));
        assert!(matches!(parse_callback("/?state=state&error=access_denied","state"),Callback::Denied));
        assert!(matches!(parse_callback("/?state=state&code=a%2Bb","state"),Callback::Code(c) if c=="a+b"));
    }
    #[test]fn local_callback_and_cancel(){
        let listener=TcpListener::bind(("127.0.0.1",0)).unwrap();let addr=listener.local_addr().unwrap();
        let task=std::thread::spawn(move||{
            for request in ["/?state=wrong&code=bad","/?state=state&code=good"]{
                let mut stream=TcpStream::connect(addr).unwrap();
                write!(stream,"GET {request} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
                let mut response=String::new();stream.read_to_string(&mut response).unwrap();
            }
        });
        assert_eq!(callback(listener,"state",&AtomicBool::new(false)).unwrap(),"good");
        task.join().unwrap();
        assert!(callback(TcpListener::bind(("127.0.0.1",0)).unwrap(),"state",&AtomicBool::new(true)).is_err());
    }
    #[test]fn token_rotation_and_sasl(){
        let mut t=Tokens{client_id:"id".into(),desktop_client_secret:String::new(),access_token:"old".into(),refresh_token:"refresh1".into(),expires_at:0};
        assert!(t.expiring());
        t.apply(TokenResponse{access_token:"new".into(),refresh_token:None,expires_in:3600,token_type:Some("Bearer".into())}).unwrap();
        assert!(!t.expiring());assert_eq!(t.refresh_token,"refresh1");
        t.apply(TokenResponse{access_token:"new2".into(),refresh_token:Some("refresh2".into()),expires_in:3600,token_type:None}).unwrap();
        assert_eq!(t.refresh_token,"refresh2");
        let auth=Xoauth2::new("a@example.invalid","token");
        assert_eq!(auth.process(b""),b"user=a@example.invalid\x01auth=Bearer token\x01\x01");
        assert!(auth.process(b"error").is_empty());
    }
}
