//! Local calendar, RFC 5545 subset, read-only CalDAV REPORT and explicit iTIP drafts.
use super::{Store,Compose,OutgoingAttachment,ImportReport,encrypt,decrypt,Uuid};
use anyhow::{anyhow,bail,Context,Result};
use base64::{engine::general_purpose::STANDARD,Engine};
use chrono::{NaiveDate,NaiveDateTime,Utc,Local,TimeZone};
use rusqlite::{params,Connection,TransactionBehavior};
use serde::{Deserialize,Serialize};
use sha2::{Digest,Sha256};
use std::{collections::HashSet,fs::{self,OpenOptions},io::Write,path::Path,time::Duration};
use url::Url;

pub(crate) fn migrate(db:&Connection)->Result<()>{
    db.execute_batch("CREATE TABLE calendar_sources(id TEXT PRIMARY KEY,name TEXT NOT NULL,url TEXT NOT NULL,username TEXT NOT NULL,secret BLOB NOT NULL,last_sync INTEGER);
      CREATE TABLE calendar_items(id TEXT PRIMARY KEY,source_id TEXT REFERENCES calendar_sources(id) ON DELETE CASCADE,origin TEXT NOT NULL,uid TEXT NOT NULL,recurrence_id TEXT NOT NULL DEFAULT '',payload TEXT NOT NULL,raw_ics TEXT NOT NULL);
      CREATE INDEX calendar_item_source ON calendar_items(source_id);
      CREATE UNIQUE INDEX calendar_local_import ON calendar_items(uid,recurrence_id) WHERE origin='import';
      CREATE TABLE calendar_reminded(event_id TEXT NOT NULL REFERENCES calendar_items(id) ON DELETE CASCADE,sequence INTEGER NOT NULL,PRIMARY KEY(event_id,sequence));
      CREATE TABLE calendar_settings(id INTEGER PRIMARY KEY CHECK(id=1),notifications INTEGER NOT NULL DEFAULT 0);
      INSERT INTO calendar_settings(id,notifications) VALUES(1,0);
      PRAGMA user_version=8;")?;Ok(())
}
#[derive(Default,Clone,Serialize,Deserialize)]
#[serde(default)]
pub struct CalendarEvent{
    pub id:String,pub source_id:Option<String>,pub origin:String,pub uid:String,pub summary:String,
    pub start:String,pub end:String,pub all_day:bool,pub timezone:String,pub location:String,pub description:String,
    pub organizer:String,pub attendees:Vec<String>,pub sequence:u32,pub recurrence_id:String,pub recurring:bool,
    pub method:String,pub reminder_minutes:Option<u32>,
}
#[derive(Clone,Serialize,Deserialize)]
pub struct CalendarSource{pub id:String,pub name:String,pub url:String,pub username:String,pub last_sync:Option<i64>}
#[derive(Deserialize)]
pub struct NewCalendarSource{pub name:String,pub url:String,pub username:String,pub password:String}
#[derive(Serialize)]
pub struct Invitation{pub index:usize,pub event:CalendarEvent}
#[derive(Serialize)]
pub struct CalendarSyncReport{pub events:usize,pub resources:usize}
#[derive(Clone)]
struct ParsedEvent{event:CalendarEvent,raw:String,start_line:String,end_line:Option<String>,recurrence_line:Option<String>}
#[derive(Clone)]
struct Property{name:String,head:String,value:String}
fn property(line:&str)->Option<Property>{
    let mut quoted=false;let mut colon=None;
    for (i,c) in line.char_indices(){if c=='"'{quoted=!quoted}else if c==':'&&!quoted{colon=Some(i);break}}
    let i=colon?;let head=line[..i].to_owned();let name=head.split(';').next()?.to_ascii_uppercase();
    Some(Property{name,head,value:line[i+1..].to_owned()})
}
fn unfold(text:&str)->Vec<String>{
    let mut lines:Vec<String>=Vec::new();
    for line in text.lines(){
        let line=line.trim_end_matches('\r');
        if line.starts_with([' ','\t'])&&!lines.is_empty(){lines.last_mut().unwrap().push_str(&line[1..])}
        else{lines.push(line.to_owned())}
    }
    lines
}
fn unescape(s:&str)->String{
    let mut out=String::new();let mut it=s.chars();
    while let Some(c)=it.next(){if c=='\\'{match it.next(){Some('n'|'N')=>out.push('\n'),Some(v)=>out.push(v),None=>out.push('\\')}}else{out.push(c)}}out
}
fn escape(s:&str)->String{s.replace('\\',"\\\\").replace("\r\n","\n").replace('\r',"\n").replace('\n',"\\n").replace(';',"\\;").replace(',',"\\,")}
fn param(head:&str,name:&str)->Option<String>{
    head.split(';').skip(1).find_map(|s|{let (k,v)=s.split_once('=')?;if k.eq_ignore_ascii_case(name){Some(v.trim_matches('"').to_owned())}else{None}})
}
fn date_value(p:&Property)->Result<(String,bool,String)>{
    let v=p.value.trim();
    if v.len()==8&&!v.contains('T'){
        let d=NaiveDate::parse_from_str(v,"%Y%m%d").context("Invalid iCalendar date")?;
        return Ok((d.format("%Y-%m-%d").to_string(),true,String::new()))
    }
    let utc=v.ends_with('Z');let d=NaiveDateTime::parse_from_str(v.trim_end_matches('Z'),"%Y%m%dT%H%M%S").context("Unsupported iCalendar date-time")?;
    let zone=if utc{"UTC".into()}else{param(&p.head,"TZID").unwrap_or_default()};
    let value=d.format("%Y-%m-%dT%H:%M:%S").to_string()+if utc{"Z"}else{""};
    Ok((value,false,zone))
}
fn address(s:&str)->Option<String>{
    let (scheme,value)=s.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("mailto")||value.chars().any(char::is_control){return None}
    value.parse::<lettre::Address>().ok().map(|a|a.to_string())
}
fn duration_seconds(value:&str)->Result<i64>{
    let value=value.strip_prefix('+').unwrap_or(value).strip_prefix('P').ok_or_else(||anyhow!("Unsupported iCalendar duration"))?;
    let mut number=String::new();let mut total=0i64;let mut time=false;
    for c in value.chars(){
        if c=='T'&&!time&&number.is_empty(){time=true;continue}
        if c.is_ascii_digit(){number.push(c);continue}
        let n:i64=number.parse().context("Invalid calendar duration")?;number.clear();
        let multiplier=match(c,time){('W',false)=>604800,('D',false)=>86400,('H',true)=>3600,('M',true)=>60,('S',true)=>1,_=>bail!("Unsupported calendar duration")};
        total=total.checked_add(n.checked_mul(multiplier).ok_or_else(||anyhow!("Duration overflow"))?).ok_or_else(||anyhow!("Duration overflow"))?;
    }
    if !number.is_empty()||total<=0||total>i64::MAX/1000{bail!("Invalid calendar duration")}
    Ok(total)
}
fn event_from_lines(lines:&[String],header:&[String],zones:&[String],method:&str)->Result<ParsedEvent>{
    let mut props=Vec::new();let mut level=0;
    for line in lines.iter().skip(1).take(lines.len().saturating_sub(2)){
        if line.to_ascii_uppercase().starts_with("BEGIN:"){level+=1;continue}
        if line.to_ascii_uppercase().starts_with("END:"){level-=1;continue}
        if level==0{if let Some(p)=property(line){props.push(p)}}
    }
    let find=|key:&str|props.iter().find(|p|p.name==key);
    let get=|key:&str|find(key).map(|p|p.value.clone()).unwrap_or_default();
    let uid=get("UID");if uid.is_empty()||uid.chars().any(char::is_control){bail!("VEVENT missing valid UID")}
    let start_prop=find("DTSTART").ok_or_else(||anyhow!("VEVENT missing DTSTART"))?;
    let (start,all_day,timezone)=date_value(start_prop)?;
    let end_prop=find("DTEND");
    let end=if let Some(p)=end_prop{let (v,day,_)=date_value(p)?;if day!=all_day{bail!("Mismatched event date types")}v}
        else if let Some(duration)=find("DURATION"){
            let seconds=duration_seconds(&duration.value)?;
            if all_day{
                if seconds%86400!=0{bail!("All-day event duration must be in days or weeks")}
                NaiveDate::parse_from_str(&start,"%Y-%m-%d")?.checked_add_signed(chrono::Duration::seconds(seconds)).ok_or_else(||anyhow!("Date overflow"))?.format("%Y-%m-%d").to_string()
            }else{
                let dt=NaiveDateTime::parse_from_str(start.trim_end_matches('Z'),"%Y-%m-%dT%H:%M:%S")?;
                dt.checked_add_signed(chrono::Duration::seconds(seconds)).ok_or_else(||anyhow!("Date-time overflow"))?.format("%Y-%m-%dT%H:%M:%S").to_string()+if start.ends_with('Z'){"Z"}else{""}
            }
        }
        else if all_day{NaiveDate::parse_from_str(&start,"%Y-%m-%d")?.succ_opt().ok_or_else(||anyhow!("Date overflow"))?.format("%Y-%m-%d").to_string()}
        else{start.clone()};
    let sequence=get("SEQUENCE").parse().unwrap_or(0);
    let recurrence_id=get("RECURRENCE-ID");
    let organizer=address(&get("ORGANIZER")).unwrap_or_default();
    let attendees=props.iter().filter(|p|p.name=="ATTENDEE").filter_map(|p|address(&p.value)).collect();
    let recurring=!recurrence_id.is_empty()||["RRULE","RDATE","EXDATE"].iter().any(|k|find(k).is_some());
    let mut raw=header.join("\r\n")+"\r\n";
    if !zones.is_empty(){raw+=&(zones.join("\r\n")+"\r\n");}
    raw+=&(lines.join("\r\n")+"\r\nEND:VCALENDAR\r\n");
    let event=CalendarEvent{id:String::new(),source_id:None,origin:"import".into(),uid,summary:unescape(&get("SUMMARY")),start,end,all_day,timezone,
        location:unescape(&get("LOCATION")),description:unescape(&get("DESCRIPTION")),organizer,attendees,sequence,recurrence_id,recurring,method:method.into(),reminder_minutes:None};
    Ok(ParsedEvent{event,raw,start_line:format!("{}:{}",start_prop.head,start_prop.value),end_line:end_prop.map(|p|format!("{}:{}",p.head,p.value)),recurrence_line:find("RECURRENCE-ID").map(|p|format!("{}:{}",p.head,p.value))})
}
fn parse_ics(text:&str)->Result<Vec<ParsedEvent>>{
    let lines=unfold(text.trim_start_matches('\u{feff}'));
    if lines.iter().any(|l|l.chars().any(|c|c.is_control()&&c!='\t')){bail!("Invalid calendar content line")}
    let mut result=Vec::new();let mut header:Vec<String>=Vec::new();let mut zones:Vec<String>=Vec::new();let mut events:Vec<Vec<String>>=Vec::new();
    let mut inside=false;let mut depth=0usize;let mut component=Vec::new();let mut method=String::new();
    for line in lines{
        let upper=line.to_ascii_uppercase();
        if !inside{
            if upper=="BEGIN:VCALENDAR"{inside=true;header=vec!["BEGIN:VCALENDAR".into()];zones.clear();events.clear();method.clear()}
            else if !line.trim().is_empty(){bail!("Expected VCALENDAR")}
            continue
        }
        if depth==0&&upper=="END:VCALENDAR"{
            if !header.iter().any(|l|l.eq_ignore_ascii_case("VERSION:2.0")){bail!("Only iCalendar 2.0 is supported")}
            for event in &events{result.push(event_from_lines(event,&header,&zones,&method)?)}
            inside=false;continue
        }
        if upper.starts_with("BEGIN:"){
            if depth==0{component.clear()}depth+=1;component.push(line);continue
        }
        if depth>0{
            component.push(line);
            if upper.starts_with("END:"){depth-=1;if depth==0{
                let first=component.first().map(|s|s.to_ascii_uppercase()).unwrap_or_default();
                if first=="BEGIN:VEVENT"{if upper!="END:VEVENT"{bail!("Invalid VEVENT boundary")}events.push(component.clone())}
                else if first=="BEGIN:VTIMEZONE"{if upper!="END:VTIMEZONE"{bail!("Invalid VTIMEZONE boundary")}zones.extend(component.clone())}
            }}
            continue
        }
        if let Some(p)=property(&line){if p.name=="METHOD"{method=p.value.to_ascii_uppercase()}}
        header.push(line)
    }
    if inside||depth!=0{bail!("Incomplete iCalendar document")}
    Ok(result)
}
fn format_date(value:&str,day:bool)->Result<String>{
    if day{return Ok(NaiveDate::parse_from_str(value,"%Y-%m-%d")?.format("%Y%m%d").to_string())}
    let utc=value.ends_with('Z');let dt=NaiveDateTime::parse_from_str(value.trim_end_matches('Z'),"%Y-%m-%dT%H:%M:%S").or_else(|_|NaiveDateTime::parse_from_str(value.trim_end_matches('Z'),"%Y-%m-%dT%H:%M"))?;
    Ok(dt.format("%Y%m%dT%H%M%S").to_string()+if utc{"Z"}else{""})
}
fn fold(line:&str)->String{
    let mut result=String::new();let mut len=0;
    for ch in line.chars(){if len+ch.len_utf8()>75{result.push_str("\r\n ");len=1}result.push(ch);len+=ch.len_utf8()}
    result.push_str("\r\n");result
}
fn calendar_lines(lines:Vec<String>)->String{lines.iter().map(|s|fold(s)).collect()}
fn date_line(key:&str,value:&str,day:bool)->Result<String>{
    Ok(format!("{key}{}:{}",if day{";VALUE=DATE"}else{""},format_date(value,day)?))
}
fn dtstamp()->String{Utc::now().format("%Y%m%dT%H%M%SZ").to_string()}
fn validate_local(e:&CalendarEvent)->Result<()>{
    if e.summary.trim().is_empty(){bail!("Event title is required")}
    if !e.timezone.is_empty()&&e.timezone!="UTC"{bail!("Local editing supports floating local time or UTC")}
    let start=format_date(&e.start,e.all_day)?;let end=format_date(&e.end,e.all_day)?;
    if end<=start||start.ends_with('Z')!=end.ends_with('Z'){bail!("Event end must be after start in the same time basis")}
    if e.uid.is_empty()||e.uid.chars().any(char::is_control){bail!("Invalid event UID")}
    Ok(())
}
fn make_ics(e:&CalendarEvent,method:Option<&str>)->Result<String>{
    validate_local(e)?;
    let mut lines=vec!["BEGIN:VCALENDAR".into(),"VERSION:2.0".into(),"PRODID:-//Mail EX//Calendar//EN".into()];
    if let Some(m)=method{lines.push(format!("METHOD:{m}"))}
    lines.extend(["BEGIN:VEVENT".into(),format!("UID:{}",e.uid),format!("DTSTAMP:{}",dtstamp()),format!("SEQUENCE:{}",e.sequence),date_line("DTSTART",&e.start,e.all_day)?,date_line("DTEND",&e.end,e.all_day)?,format!("SUMMARY:{}",escape(&e.summary)),format!("DESCRIPTION:{}",escape(&e.description)),format!("LOCATION:{}",escape(&e.location))]);
    if !e.organizer.is_empty(){let a:lettre::Address=e.organizer.parse().map_err(|_|anyhow!("Invalid organizer"))?;lines.push(format!("ORGANIZER:mailto:{a}"));}
    for attendee in &e.attendees{
        let a:lettre::Address=attendee.parse().map_err(|_|anyhow!("Invalid attendee"))?;
        lines.push(format!("ATTENDEE;RSVP=TRUE;PARTSTAT=NEEDS-ACTION:mailto:{a}"));
    }
    if let Some(minutes)=e.reminder_minutes{lines.extend(["BEGIN:VALARM".into(),format!("TRIGGER:-PT{minutes}M"),"ACTION:DISPLAY".into(),format!("DESCRIPTION:{}",escape(&e.summary)),"END:VALARM".into()]);}
    lines.extend(["END:VEVENT".into(),"END:VCALENDAR".into()]);Ok(calendar_lines(lines))
}
fn validate_url(value:&str)->Result<Url>{
    let url=Url::parse(value).context("Invalid CalDAV collection URL")?;
    if url.scheme()!="https"||url.host_str().is_none()||!url.username().is_empty()||url.password().is_some()||url.fragment().is_some()||url.query().is_some(){bail!("CalDAV requires an explicit HTTPS collection URL without embedded credentials")}
    Ok(url)
}
fn remote_events(xml:&str)->Result<(usize,Vec<(String,ParsedEvent)>)>{
    let doc=roxmltree::Document::parse(xml).context("Invalid CalDAV XML response")?;
    let root=doc.root_element();
    if root.tag_name().namespace()!=Some("DAV:")||root.tag_name().name()!="multistatus"{bail!("Expected DAV multistatus")}
    let dav=|n:roxmltree::Node<'_, '_>,name:&str|n.is_element()&&n.tag_name().namespace()==Some("DAV:")&&n.tag_name().name()==name;
    let mut out=Vec::new();let mut count=0;let mut keys=HashSet::new();
    for response in root.children().filter(|n|dav(*n,"response")){
        let href=response.children().find(|n|dav(*n,"href")).and_then(|n|n.text()).filter(|s|!s.is_empty()).ok_or_else(||anyhow!("CalDAV response missing href"))?;
        let mut data=None;
        for stat in response.children().filter(|n|dav(*n,"propstat")){
            let status=stat.children().find(|n|dav(*n,"status")).and_then(|n|n.text()).unwrap_or("");
            if status.split_whitespace().nth(1)!=Some("200"){continue}
            if let Some(prop)=stat.children().find(|n|dav(*n,"prop")){
                if let Some(value)=prop.children().find(|n|n.is_element()&&n.tag_name().namespace()==Some("urn:ietf:params:xml:ns:caldav")&&n.tag_name().name()=="calendar-data"){
                    data=value.text();
                }
            }
        }
        let data=data.ok_or_else(||anyhow!("Incomplete CalDAV response; previous calendar retained"))?;
        let events=parse_ics(data)?;if events.is_empty(){bail!("CalDAV response has no supported VEVENT")}
        count+=1;
        for event in events{
            if !keys.insert((href.to_owned(),event.event.uid.clone(),event.event.recurrence_id.clone())){bail!("Duplicate CalDAV event identity")}
            out.push((href.to_owned(),event));
        }
    }
    Ok((count,out))
}
fn load_invitations(raw:&[u8])->Result<Vec<ParsedEvent>>{
    let mail=mailparse::parse_mail(raw)?;
    fn scan(mail:&mailparse::ParsedMail<'_>,out:&mut Vec<ParsedEvent>)->Result<()>{
        let kind=mail.ctype.mimetype.to_ascii_lowercase();
        let filename=mail.get_content_disposition().params.get("filename").cloned().unwrap_or_default();
        if kind=="text/calendar"||filename.to_ascii_lowercase().ends_with(".ics"){
            if let Ok(body)=mail.get_body(){if let Ok(items)=parse_ics(&body){out.extend(items.into_iter().filter(|e|e.event.method=="REQUEST"))}}
        }else{for part in &mail.subparts{scan(part,out)?}}
        Ok(())
    }
    let mut out=Vec::new();scan(&mail,&mut out)?;Ok(out)
}
fn calendar_attachment(ics:String,method:&str)->OutgoingAttachment{
    OutgoingAttachment{name:"meeting.ics".into(),media_type:format!("text/calendar; charset=utf-8; method={method}"),data:STANDARD.encode(ics)}
}
fn compose(account_id:String,to:String,subject:String,body:String,ics:String,method:&str)->Compose{
    Compose{account_id,to,cc:String::new(),bcc:String::new(),subject,body,in_reply_to:None,references:None,html:None,attachments:vec![calendar_attachment(ics,method)],save_sent_copy:false}
}
fn reminder_time(value:&str,all_day:bool)->Option<i64>{
    if value.ends_with('Z'){return NaiveDateTime::parse_from_str(value.trim_end_matches('Z'),"%Y-%m-%dT%H:%M:%S").ok().or_else(||NaiveDateTime::parse_from_str(value.trim_end_matches('Z'),"%Y-%m-%dT%H:%M").ok()).map(|dt|dt.and_utc().timestamp())}
    let dt=if all_day{NaiveDate::parse_from_str(value,"%Y-%m-%d").ok()?.and_hms_opt(0,0,0)?}
        else{NaiveDateTime::parse_from_str(value,"%Y-%m-%dT%H:%M:%S").ok().or_else(||NaiveDateTime::parse_from_str(value,"%Y-%m-%dT%H:%M").ok())?};
    Local.from_local_datetime(&dt).single().map(|d|d.timestamp())
}
impl Store{
    pub fn notification_enabled(&self)->Result<bool>{Ok(self.db.query_row("SELECT notifications FROM calendar_settings WHERE id=1",[],|r|r.get(0))?)}
    pub fn set_notifications(&self,enabled:bool)->Result<()>{
        self.db.execute("UPDATE calendar_settings SET notifications=? WHERE id=1",[enabled])?;Ok(())
    }
    pub fn claim_reminders(&mut self,now:i64)->Result<usize>{
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let enabled:bool=tx.query_row("SELECT notifications FROM calendar_settings WHERE id=1",[],|r|r.get(0))?;
        if !enabled{tx.commit()?;return Ok(0)}
        let pending={
            let mut q=tx.prepare("SELECT payload FROM calendar_items WHERE origin='local'")?;
            let rows=q.query_map([],|r|r.get::<_,String>(0))?;
            let mut items=Vec::new();for row in rows{let e:CalendarEvent=serde_json::from_str(&row?)?;
                if let (Some(minutes),Some(start),Some(end))=(e.reminder_minutes,reminder_time(&e.start,e.all_day),reminder_time(&e.end,e.all_day)){
                    if now>=start.saturating_sub(i64::from(minutes)*60)&&now<end{items.push((e.id,e.sequence))}
                }
            }items
        };
        let mut count=0;for (id,sequence) in pending{count+=tx.execute("INSERT OR IGNORE INTO calendar_reminded(event_id,sequence) VALUES(?,?)",params![id,sequence])?;}
        tx.commit()?;Ok(count)
    }
    pub fn calendar_sources(&self)->Result<Vec<CalendarSource>>{
        let mut q=self.db.prepare("SELECT id,name,url,username,last_sync FROM calendar_sources ORDER BY name,id")?;
        let rows=q.query_map([],|r|Ok(CalendarSource{id:r.get(0)?,name:r.get(1)?,url:r.get(2)?,username:r.get(3)?,last_sync:r.get(4)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;Ok(rows)
    }
    pub fn add_calendar_source(&self,input:NewCalendarSource)->Result<CalendarSource>{
        let url=validate_url(input.url.trim())?;
        if input.name.trim().is_empty()||input.username.trim().is_empty()||input.password.is_empty(){bail!("Calendar name, username and app password are required")}
        let source=CalendarSource{id:Uuid::new_v4().to_string(),name:input.name.trim().into(),url:url.to_string(),username:input.username.trim().into(),last_sync:None};
        let secret=encrypt(&self.key,&source.id,&input.password)?;
        self.db.execute("INSERT INTO calendar_sources(id,name,url,username,secret) VALUES(?,?,?,?,?)",params![source.id,source.name,source.url,source.username,secret])?;Ok(source)
    }
    pub fn remove_calendar_source(&self,id:&str)->Result<()>{
        self.db.execute("DELETE FROM calendar_sources WHERE id=?",[id])?;Ok(())
    }
    pub fn calendar_events(&self)->Result<Vec<CalendarEvent>>{
        let mut q=self.db.prepare("SELECT payload FROM calendar_items")?;
        let rows=q.query_map([],|r|r.get::<_,String>(0))?;let mut items=Vec::new();
        for row in rows{items.push(serde_json::from_str::<CalendarEvent>(&row?)?)}
        items.sort_by(|a,b|a.start.cmp(&b.start).then(a.id.cmp(&b.id)));Ok(items)
    }
    pub fn save_calendar_event(&self,mut e:CalendarEvent)->Result<CalendarEvent>{
        if e.id.is_empty(){
            e.id=Uuid::new_v4().to_string();e.uid=format!("{}@mail-ex.local",e.id);e.sequence=0;
        }else{
            let raw:String=self.db.query_row("SELECT payload FROM calendar_items WHERE id=? AND origin='local'",[&e.id],|r|r.get(0)).context("Only local events can be edited")?;
            let old:CalendarEvent=serde_json::from_str(&raw)?;e.uid=old.uid;e.sequence=old.sequence.saturating_add(1);
        }
        e.origin="local".into();e.source_id=None;e.recurring=false;e.recurrence_id.clear();e.organizer.clear();e.attendees.clear();e.method.clear();
        if e.start.ends_with('Z'){e.timezone="UTC".into()}else{e.timezone.clear()}
        let raw=make_ics(&e,None)?;
        self.db.execute("INSERT INTO calendar_items(id,source_id,origin,uid,recurrence_id,payload,raw_ics) VALUES(?1,NULL,'local',?2,'',?3,?4) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload,raw_ics=excluded.raw_ics WHERE calendar_items.origin='local'",params![e.id,e.uid,serde_json::to_string(&e)?,raw])?;Ok(e)
    }
    pub fn delete_calendar_event(&self,id:&str)->Result<()>{
        if self.db.execute("DELETE FROM calendar_items WHERE id=? AND origin<>'remote'",[id])?!=1{bail!("Remote events are read-only; remove the calendar source to remove its cache")}Ok(())
    }
    fn import_events(&mut self,items:Vec<ParsedEvent>)->Result<ImportReport>{
        let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;let mut report=ImportReport::default();
        for mut item in items{
            item.event.id=Uuid::new_v4().to_string();item.event.origin="import".into();
            let e=&item.event;
            let changed=tx.execute("INSERT OR IGNORE INTO calendar_items(id,source_id,origin,uid,recurrence_id,payload,raw_ics) VALUES(?1,NULL,'import',?2,?3,?4,?5)",params![e.id,e.uid,e.recurrence_id,serde_json::to_string(e)?,item.raw])?;
            if changed==1{report.added+=1}else{report.skipped+=1}
        }
        tx.commit()?;Ok(report)
    }
    pub fn import_calendar_file(&mut self,path:&Path)->Result<ImportReport>{self.import_events(parse_ics(&fs::read_to_string(path)?)?)}
    pub fn export_calendar_event(&self,id:&str,path:&Path)->Result<()>{
        let raw:String=self.db.query_row("SELECT raw_ics FROM calendar_items WHERE id=?",[id],|r|r.get(0))?;
        let mut file=OpenOptions::new().write(true).create_new(true).open(path).context("Choose a new file; existing files are not overwritten")?;
        file.write_all(raw.as_bytes())?;file.sync_all()?;Ok(())
    }
    pub fn sync_calendar(&mut self,id:&str)->Result<CalendarSyncReport>{
        let (url,user,secret):(String,String,Vec<u8>)=self.db.query_row("SELECT url,username,secret FROM calendar_sources WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let url=validate_url(&url)?;let password=decrypt(&self.key,id,&secret)?;
        let client=reqwest::blocking::Client::builder().timeout(Duration::from_secs(60)).redirect(reqwest::redirect::Policy::none()).build()?;
        let query=r#"<?xml version="1.0" encoding="utf-8"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/><c:calendar-data/></d:prop><c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT"/></c:comp-filter></c:filter></c:calendar-query>"#;
        let response=client.request(reqwest::Method::from_bytes(b"REPORT")?,url).basic_auth(user,Some(password)).header("Depth","1").header("Content-Type","application/xml; charset=utf-8").body(query).send().map_err(|_|anyhow!("CalDAV request failed"))?;
        if response.status().as_u16()!=207{bail!("CalDAV REPORT rejected (HTTP {})",response.status().as_u16())}
        let (resources,items)=remote_events(&response.text().map_err(|_|anyhow!("Cannot read CalDAV response"))?)?;
        let count=items.len();let tx=self.db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Replace only after every object has been parsed successfully.
        tx.execute("DELETE FROM calendar_items WHERE source_id=?",[id])?;
        for (href,mut item) in items{
            let e=&mut item.event;e.source_id=Some(id.into());e.origin="remote".into();
            e.id=format!("{:x}",Sha256::digest(serde_json::to_vec(&(id,href,&e.uid,&e.recurrence_id))?));
            tx.execute("INSERT INTO calendar_items(id,source_id,origin,uid,recurrence_id,payload,raw_ics) VALUES(?,?,'remote',?,?,?,?)",params![e.id,id,e.uid,e.recurrence_id,serde_json::to_string(e)?,item.raw])?;
        }
        tx.execute("UPDATE calendar_sources SET last_sync=unixepoch() WHERE id=?",[id])?;tx.commit()?;
        Ok(CalendarSyncReport{events:count,resources})
    }
    fn invitations(&self,message_id:i64)->Result<Vec<ParsedEvent>>{
        let raw:Vec<u8>=self.db.query_row("SELECT raw FROM messages WHERE id=?",[message_id],|r|r.get(0))?;
        load_invitations(&raw)
    }
    pub fn message_invitations(&self,message_id:i64)->Result<Vec<Invitation>>{
        Ok(self.invitations(message_id)?.into_iter().enumerate().map(|(index,item)|Invitation{index,event:item.event}).collect())
    }
    pub fn add_invitation(&mut self,message_id:i64,index:usize)->Result<ImportReport>{
        let item=self.invitations(message_id)?.into_iter().nth(index).ok_or_else(||anyhow!("Invitation not found"))?;
        self.import_events(vec![item])
    }
    pub fn invitation_reply(&self,message_id:i64,index:usize,account_id:&str,partstat:&str)->Result<Compose>{
        if !["ACCEPTED","DECLINED","TENTATIVE"].contains(&partstat){bail!("Unsupported meeting response")}
        let a=self.account(account_id)?;
        if a.auth_kind=="local"{bail!("Choose a sending mailbox")}
        let item=self.invitations(message_id)?.into_iter().nth(index).ok_or_else(||anyhow!("Invitation not found"))?;
        let e=item.event;
        if e.organizer.is_empty()||!e.attendees.iter().any(|x|x.eq_ignore_ascii_case(&a.email)){bail!("Selected mailbox is not an invited attendee or organizer is missing")}
        let mut lines=vec!["BEGIN:VCALENDAR".into(),"VERSION:2.0".into(),"PRODID:-//Mail EX//Calendar//EN".into(),"METHOD:REPLY".into(),"BEGIN:VEVENT".into(),format!("UID:{}",e.uid),format!("DTSTAMP:{}",dtstamp()),format!("SEQUENCE:{}",e.sequence),format!("ORGANIZER:mailto:{}",e.organizer),format!("ATTENDEE;PARTSTAT={partstat}:mailto:{}",a.email)];
        if let Some(r)=item.recurrence_line{lines.push(r)}
        // Retain DTSTART and DTEND for UTC / date / floating values; for TZID
        // responses omit them rather than introduce an undefined timezone.
        if e.timezone.is_empty()||e.timezone=="UTC"{lines.push(item.start_line);if let Some(end)=item.end_line{lines.push(end)}}
        lines.extend([format!("SUMMARY:{}",escape(&e.summary)),"END:VEVENT".into(),"END:VCALENDAR".into()]);
        Ok(compose(a.id,e.organizer,format!("{partstat}: {}",e.summary),format!("Meeting response: {partstat}\n{}",e.summary),calendar_lines(lines),"REPLY"))
    }
    pub fn meeting_invitation(&self,event_id:&str,account_id:&str,recipients:&str)->Result<Compose>{
        let a=self.account(account_id)?;if a.auth_kind=="local"{bail!("Choose a sending mailbox")}
        let payload:String=self.db.query_row("SELECT payload FROM calendar_items WHERE id=? AND origin='local'",[event_id],|r|r.get(0)).context("Only local events can be used to create invitations")?;
        let mut e:CalendarEvent=serde_json::from_str(&payload)?;
        e.attendees=super::mail_addresses(recipients)?;if e.attendees.is_empty(){bail!("At least one attendee is required")}
        e.organizer=a.email;let ics=make_ics(&e,Some("REQUEST"))?;
        Ok(compose(a.id,e.attendees.join(", "),format!("Invitation: {}",e.summary),format!("{}\n{}\n{}\n{}",e.summary,e.start,e.end,e.location),ics,"REQUEST"))
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    const ICS:&str="BEGIN:VCALENDAR\r\nVERSION:2.0\r\nMETHOD:REQUEST\r\nBEGIN:VEVENT\r\nUID:meeting@example.invalid\r\nDTSTART:20261201T080000Z\r\nDTEND:20261201T090000Z\r\nSUMMARY:Review\\, build\r\nORGANIZER:mailto:host@example.invalid\r\nATTENDEE:mailto:guest@example.invalid\r\nSEQUENCE:2\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    #[test]fn ics_parsing_and_local_roundtrip(){
        let e=parse_ics(ICS).unwrap().remove(0).event;
        assert_eq!(e.summary,"Review, build");assert_eq!(e.start,"2026-12-01T08:00:00Z");assert_eq!(e.method,"REQUEST");assert_eq!(e.sequence,2);
        let d=tempfile::tempdir().unwrap();let mut store=Store::open(d.path()).unwrap();
        let event=store.save_calendar_event(CalendarEvent{summary:"测试，日程".into(),start:"2026-12-01T10:00".into(),end:"2026-12-01T11:00".into(),reminder_minutes:Some(10),..Default::default()}).unwrap();
        let file=d.path().join("meeting.ics");store.export_calendar_event(&event.id,&file).unwrap();
        assert_eq!(parse_ics(&fs::read_to_string(&file).unwrap()).unwrap()[0].event.summary,"测试，日程");
        assert_eq!(store.import_calendar_file(&file).unwrap().added,1);assert_eq!(store.import_calendar_file(&file).unwrap().skipped,1);
        assert_eq!(store.calendar_events().unwrap().len(),2);
        store.delete_calendar_event(&event.id).unwrap();assert_eq!(store.calendar_events().unwrap().len(),1);
    }
    #[test]fn reminders_claim_once_and_opt_in(){
        let d=tempfile::tempdir().unwrap();let mut store=Store::open(d.path()).unwrap();
        let e=store.save_calendar_event(CalendarEvent{summary:"Reminder".into(),start:"2026-12-01T10:00:00Z".into(),end:"2026-12-01T11:00:00Z".into(),reminder_minutes:Some(10),..Default::default()}).unwrap();
        let now=reminder_time(&e.start,false).unwrap()-300;
        assert_eq!(store.claim_reminders(now).unwrap(),0);
        store.set_notifications(true).unwrap();
        assert_eq!(store.claim_reminders(now-600).unwrap(),0);
        assert_eq!(store.claim_reminders(now).unwrap(),1);
        assert_eq!(store.claim_reminders(now).unwrap(),0);
        drop(store);let mut store=Store::open(d.path()).unwrap();
        assert_eq!(store.claim_reminders(now).unwrap(),0);
    }
    #[test]fn caldav_report_and_invitation_mime(){
        let xml=format!(r#"<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:response><d:href>/calendar/1.ics</d:href><d:propstat><d:prop><c:calendar-data><![CDATA[{ICS}]]></c:calendar-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#);
        assert_eq!(remote_events(&xml).unwrap().1.len(),1);
        assert!(remote_events(&xml.replace("200 OK","404 Not Found")).is_err());
        assert!(validate_url("http://example.invalid/calendar").is_err());assert!(validate_url("https://u:p@example.invalid/").is_err());
        let raw=format!("Content-Type: text/calendar; method=REQUEST\r\n\r\n{ICS}");
        assert_eq!(load_invitations(raw.as_bytes()).unwrap().len(),1);
    }
}
