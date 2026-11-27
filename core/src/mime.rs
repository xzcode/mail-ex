//! Mail MIME handling. Incoming HTML and outgoing rich text share a strict allowlist.
//! No HTML attributes are retained: remote resources, links, styles and active content
//! cannot be loaded by a preview.
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart, MessageBuilder};
use lettre::Message;
use mailparse::{DispositionType, MailHeaderMap, ParsedMail};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutgoingAttachment {
    pub name: String,
    pub media_type: String,
    pub data: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct AttachmentInfo {
    pub index: usize,
    pub name: String,
    pub media_type: String,
    pub size: usize,
    pub previewable: bool,
}
#[derive(Serialize)]
pub struct AttachmentPreview {
    pub name: String,
    pub media_type: String,
    pub data: String,
}
pub fn sanitize_html(html: &str) -> String {
    let tags:HashSet<&str> = ["p","div","span","br","hr","h1","h2","h3","h4","h5","h6",
        "b","strong","i","em","u","s","strike","del","sub","sup","blockquote","pre","code",
        "ul","ol","li","dl","dt","dd","table","caption","thead","tbody","tfoot","tr","td","th"]
        .into_iter().collect();
    let mut builder = ammonia::Builder::default();
    builder.tags(tags)
        .tag_attributes(HashMap::new())
        .generic_attributes(HashSet::new())
        .link_rel(None)
        .clean_content_tags(["script","style","iframe","object","embed","svg","math","template","noscript","head","title","form"].into_iter().collect());
    builder.clean(html).to_string()
}
pub fn plain_from_html(html:&str)->String{
    let safe=sanitize_html(html);
    let mut s=safe.replace("<br>","\n").replace("<hr>","\n");
    for tag in ["p","div","li","blockquote","pre","h1","h2","h3","h4","h5","h6","tr","dl","dt","dd"] {
        s=s.replace(&format!("</{tag}>"),"\n");
    }
    let mut builder=ammonia::Builder::default();
    builder.tags(HashSet::new()).tag_attributes(HashMap::new()).generic_attributes(HashSet::new()).link_rel(None);
    html_escape::decode_html_entities(&builder.clean(&s).to_string()).trim().to_owned()
}
fn filename(mail:&ParsedMail<'_>)->Option<String>{
    let disposition=mail.get_content_disposition();
    disposition.params.get("filename").or_else(||mail.ctype.params.get("name")).map(|s|s.to_string())
}
fn explicit_attachment(mail:&ParsedMail<'_>)->bool{
    mail.get_content_disposition().disposition==DispositionType::Attachment||filename(mail).is_some()
}
pub fn body_part(mail:&ParsedMail<'_>,mime:&str)->Option<String>{
    if explicit_attachment(mail)||mail.ctype.mimetype.eq_ignore_ascii_case("message/rfc822"){return None}
    if mail.ctype.mimetype.eq_ignore_ascii_case(mime){return mail.get_body().ok()}
    mail.subparts.iter().find_map(|p|body_part(p,mime))
}
fn collect_parts<'a,'b>(mail:&'a ParsedMail<'b>,out:&mut Vec<&'a ParsedMail<'b>>){
    let mime=mail.ctype.mimetype.to_ascii_lowercase();
    if explicit_attachment(mail)||mime=="message/rfc822"||
        (mail.subparts.is_empty()&&!mime.starts_with("multipart/")&&mime!="text/plain"&&mime!="text/html"){
        out.push(mail);
    } else {
        for sub in &mail.subparts{collect_parts(sub,out)}
    }
}
pub fn attachment_parts<'a,'b>(mail:&'a ParsedMail<'b>)->Vec<&'a ParsedMail<'b>>{
    let mut parts=Vec::new();collect_parts(mail,&mut parts);parts
}
pub fn safe_filename(name:&str)->String{
    // Never interpret a MIME-provided file name as a path.
    let clean:String=name.rsplit(['/', '\\']).next().unwrap_or("").chars()
        .filter(|c|!c.is_control())
        .map(|c|if "<>:\"|?*".contains(c){'_'}else{c}).collect();
    let clean=clean.trim().trim_matches('.');
    if clean.is_empty(){"attachment.bin".into()}else{clean.to_owned()}
}
pub fn attachment_name(part:&ParsedMail<'_>,index:usize)->String{
    safe_filename(&filename(part).unwrap_or_else(||if part.ctype.mimetype.eq_ignore_ascii_case("message/rfc822"){format!("message-{}.eml",index+1)}else{format!("attachment-{}.bin",index+1)}))
}
pub fn attachment_bytes(part:&ParsedMail<'_>)->Result<Vec<u8>>{
    part.get_body_raw().map_err(|_|anyhow!("Cannot decode attachment"))
}
fn raster_type(bytes:&[u8])->Option<&'static str>{
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n"){Some("image/png")}
    else if bytes.starts_with(&[0xff,0xd8,0xff]){Some("image/jpeg")}
    else if bytes.starts_with(b"GIF87a")||bytes.starts_with(b"GIF89a"){Some("image/gif")}
    else if bytes.len()>=12&&&bytes[..4]==b"RIFF"&&&bytes[8..12]==b"WEBP"{Some("image/webp")}
    else{None}
}
pub fn metadata(part:&ParsedMail<'_>,index:usize)->Result<AttachmentInfo>{
    let bytes=attachment_bytes(part)?;
    let media_type=part.ctype.mimetype.clone();
    let previewable=raster_type(&bytes).is_some()||media_type.eq_ignore_ascii_case("text/plain");
    Ok(AttachmentInfo{index,name:attachment_name(part,index),media_type,size:bytes.len(),previewable})
}
pub fn preview(part:&ParsedMail<'_>,index:usize)->Result<AttachmentPreview>{
    let bytes=attachment_bytes(part)?;
    let name=attachment_name(part,index);
    if let Some(media)=raster_type(&bytes){return Ok(AttachmentPreview{name,media_type:media.into(),data:STANDARD.encode(bytes)})}
    if part.ctype.mimetype.eq_ignore_ascii_case("text/plain"){
        let text=part.get_body().map_err(|_|anyhow!("Cannot decode text attachment"))?;
        return Ok(AttachmentPreview{name,media_type:"text/plain".into(),data:text})
    }
    bail!("Preview is not supported for this file type; save it to disk instead")
}
pub fn outgoing(part:&ParsedMail<'_>,index:usize)->Result<OutgoingAttachment>{
    Ok(OutgoingAttachment{name:attachment_name(part,index),media_type:part.ctype.mimetype.clone(),data:STANDARD.encode(attachment_bytes(part)?)})
}
pub fn build_message(builder:MessageBuilder,body:String,html:Option<String>,attachments:Vec<OutgoingAttachment>)->Result<Message>{
    let html=html.filter(|s|!s.trim().is_empty()).map(|s|sanitize_html(&s));
    if attachments.is_empty(){
        return Ok(match html{
            Some(html)=>builder.multipart(MultiPart::alternative_plain_html(body,html))?,
            None=>builder.header(ContentType::TEXT_PLAIN).body(body)?,
        })
    }
    let mut mixed=match html{
        Some(html)=>MultiPart::mixed().multipart(MultiPart::alternative_plain_html(body,html)),
        None=>MultiPart::mixed().singlepart(SinglePart::plain(body)),
    };
    for a in attachments{
        let bytes=STANDARD.decode(a.data).map_err(|_|anyhow!("Invalid attachment data"))?;
        let ct=ContentType::parse(&a.media_type).unwrap_or_else(|_|ContentType::parse("application/octet-stream").expect("valid MIME type"));
        mixed=mixed.singlepart(Attachment::new(safe_filename(&a.name)).body(bytes,ct));
    }
    Ok(builder.multipart(mixed)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sanitizer_blocks_active_content_and_tracking(){
        let html=sanitize_html(r#"<p style="background:url(https://evil.invalid/x)" onclick="evil()"><b>Hello</b><img src="https://evil.invalid/p"><a href="javascript:alert(1)">link</a><svg><script>alert(1)</script></svg><iframe src="https://evil.invalid"></iframe><form><input name="x"></form></p>"#);
        assert!(html.contains("<b>Hello</b>"));assert!(html.contains("link"));
        for forbidden in ["evil.invalid","onclick","style=","<img","javascript:","<svg","<iframe","<form","alert("]{assert!(!html.contains(forbidden),"{html}")}
        assert_eq!(plain_from_html("<p>A &amp; B</p><p>C</p>"),"A & B\nC");
    }
    #[test]
    fn mime_roundtrip_and_attachment_safety(){
        let message=build_message(Message::builder().from("a@example.invalid".parse().unwrap()).to("b@example.invalid".parse().unwrap()).subject("MIME"),
            "plain".into(),Some("<b>rich</b><img src='https://evil.invalid'>".into()),
            vec![OutgoingAttachment{name:"../test.txt".into(),media_type:"text/plain".into(),data:STANDARD.encode(b"abc")}]).unwrap();
        let raw=message.formatted();
        let parsed=mailparse::parse_mail(&raw).unwrap();
        assert_eq!(body_part(&parsed,"text/plain").unwrap().trim(),"plain");
        assert_eq!(body_part(&parsed,"text/html").unwrap().trim(),"<b>rich</b>");
        let attachments=attachment_parts(&parsed);
        assert_eq!(attachments.len(),1);
        assert_eq!(metadata(attachments[0],0).unwrap().name,"test.txt");
        assert_eq!(attachment_bytes(attachments[0]).unwrap(),b"abc");
        assert_eq!(preview(attachments[0],0).unwrap().data,"abc");
        assert_eq!(safe_filename(r"..\..\danger?.txt"),"danger_.txt");
    }
}
