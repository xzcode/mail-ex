<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { api, type Account, type NewAccount, type Summary, type Mail, type Compose, type Folder, type Draft, type AttachmentPreview, type OutgoingAttachment, type OAuthRequest, type BatchOperation, type SearchFilters, type Contact, type MailTemplate, type SavedSearch, type LabelCount, type OutboxItem, type Invitation } from './api'
import {listen,type UnlistenFn} from '@tauri-apps/api/event'
import {getCurrentWindow} from '@tauri-apps/api/window'
import OrganizerPanel from './components/OrganizerPanel.vue'
import RecipientField from './components/RecipientField.vue'
import RichEditor from './components/RichEditor.vue'
import DataPanel from './components/DataPanel.vue'
import CalendarPanel from './components/CalendarPanel.vue'
import InvitationPanel from './components/InvitationPanel.vue'

type View = 'inbox'|'unread'|'starred'|'all'|'folder'|'drafts'|'outbox'
type OrganizerTab='contacts'|'templates'|'rules'|'searches'
let windowLabel='main'
try{windowLabel=getCurrentWindow().label}catch{/* browser preview */}
const detachedCompose=windowLabel.startsWith('compose-')
const detachedDraftId=detachedCompose?windowLabel.slice('compose-'.length):null
const language=ref(localStorage.getItem('language')||'system')
const zh=computed(()=>language.value==='zh'||(language.value==='system'&&navigator.language.toLowerCase().startsWith('zh')))
const separateCompose=ref(localStorage.getItem('separateCompose')!=='off')
const density=ref(localStorage.getItem('density')||'comfortable')
const readerFontSize=ref(Number(localStorage.getItem('readerFontSize')||14))
const settingsPanel=ref(false)
const startupError=ref('')
let allowClose=false
let unlistenDrafts:UnlistenFn|undefined,unlistenClose:UnlistenFn|undefined
const searchInput=ref<HTMLInputElement|null>(null)
const t=(en:string,cn:string)=>zh.value?cn:en
const theme=ref(localStorage.getItem('theme')||'system')
const autoMinutes=ref(Math.max(0,Number(localStorage.getItem('autoMinutes')||0)||0))
const navWidth=ref(Number(localStorage.getItem('navWidth'))||234), listWidth=ref(Number(localStorage.getItem('listWidth'))||355)
watch(language,v=>localStorage.setItem('language',v))
watch(separateCompose,v=>localStorage.setItem('separateCompose',v?'on':'off'))
watch(density,v=>localStorage.setItem('density',v))
watch(readerFontSize,v=>localStorage.setItem('readerFontSize',String(v)))
watch(theme,v=>localStorage.setItem('theme',v))
watch(autoMinutes,v=>{localStorage.setItem('autoMinutes',String(v));lastAuto=Date.now()})
watch(navWidth,v=>localStorage.setItem('navWidth',String(v)))
watch(listWidth,v=>localStorage.setItem('listWidth',String(v)))
const accounts=ref<Account[]>([]), folders=ref<Folder[]>([]), messages=ref<Summary[]>([]), drafts=ref<Draft[]>([])
const selected=ref<Mail|null>(null), expanded=ref<string[]>([])
const contacts=ref<Contact[]>([]),templates=ref<MailTemplate[]>([]),savedSearches=ref<SavedSearch[]>([]),labels=ref<LabelCount[]>([]),messageLabels=ref<string[]>([]),outbox=ref<OutboxItem[]>([])
const dataPanel=ref(false),calendarPanel=ref(false)
const invitations=ref<Invitation[]>([]),incomingProtocol=ref<'imap'|'pop3'>('imap')
let unlistenReminder:UnlistenFn|undefined
const sendingAccounts=computed(()=>accounts.value.filter(a=>a.auth_kind!=='local'))
const organizerTab=ref<OrganizerTab|null>(null),activeSavedId=ref(''),templateChoice=ref(''),templateName=ref('')
const scheduleEnabled=ref(false),scheduleAt=ref(''),deliveryRequestId=ref('')
let unlistenOutbox:UnlistenFn|undefined
const readMode=ref<'html'|'plain'>('html'), preview=ref<AttachmentPreview|null>(null)
const accountId=ref<string|null>(null), mode=ref<View>('inbox'), folderName=ref('INBOX')
const query=ref(''), offset=ref(0), more=ref(false), destination=ref('')
const status=ref(''), error=ref(''), busy=ref(false), loading=ref(false)
const dialog=ref<'account'|'compose'|'oauth'|null>(null), editId=ref<string|null>(null), draftId=ref<string|null>(null)
const preset=ref('custom')
const oauthForm=ref<OAuthRequest>({provider:'google',client_id:'',desktop_client_secret:'',email:'',name:'',color:'#507ed8',account_id:null})
const oauthWaiting=ref(false),oauthCanceling=ref(false)
const editingOAuth=computed(()=>!!editId.value&&!['password','pop3'].includes(accounts.value.find(a=>a.id===editId.value)?.auth_kind||''))
const advanced=ref(false)
const blankFilters=()=>({sender:'',recipient:'',subject:'',label:'',fromDate:'',toDate:''})
const filters=ref(blankFilters()),appliedFilters=ref<SearchFilters>({sender:'',recipient:'',subject:'',since:null,until:null})
const batchIds=ref<number[]>([]),batchDestination=ref('')
const selectedRows=computed(()=>messages.value.filter(m=>batchIds.value.includes(m.id)))
const batchFolders=computed(()=>{
  const rows=selectedRows.value
  if(!rows.length||rows.some(m=>m.account_id!==rows[0].account_id)||['local','pop3'].includes(accounts.value.find(a=>a.id===rows[0].account_id)?.auth_kind||''))return []
  return folders.value.filter(f=>f.account_id===rows[0].account_id&&f.available&&!rows.some(m=>m.folder===f.name))
})
function localDate(s:string,next=false):number|null{
  if(!s)return null
  const [y,m,d]=s.split('-').map(Number)
  const value=new Date(y,m-1,d+(next?1:0)).getTime()/1000
  return Number.isFinite(value)?value:null
}
function applySearch(){
  const f=filters.value,since=localDate(f.fromDate),until=localDate(f.toDate,true)
  if(since!==null&&until!==null&&since>=until){error.value=t('Start date must not be after end date.','开始日期不能晚于结束日期。');return}
  activeSavedId.value='';appliedFilters.value={sender:f.sender,recipient:f.recipient,subject:f.subject,label:f.label,since,until}
  void load()
}
function clearSearch(){query.value='';filters.value=blankFilters();applySearch()}
function toggleBatch(id:number){batchIds.value=batchIds.value.includes(id)?batchIds.value.filter(v=>v!==id):[...batchIds.value,id];batchDestination.value=''}
function toggleAllBatch(){batchIds.value=messages.value.length&&messages.value.every(m=>batchIds.value.includes(m.id))?[]:messages.value.map(m=>m.id);batchDestination.value=''}
async function applyBatch(operation:BatchOperation){
  const ids=[...batchIds.value];if(!ids.length)return
  const dest=operation==='move'?batchDestination.value:null
  if(operation==='move'&&!dest)return
  if(['move','archive','trash'].includes(operation)&&!confirm(t('Apply this server-side operation to the selected messages?','对所选邮件执行此服务器操作？')))return
  await run(async()=>{
    const report=await api.batch(ids,operation,dest)
    await refreshAccounts();await load(true,true)
    batchIds.value=report.failed.map(f=>f.id).filter(id=>messages.value.some(m=>m.id===id))
    status.value=t('Succeeded: ','成功：')+report.succeeded.length+' · '+t('Failed: ','失败：')+report.failed.length
    if(report.failed.length)error.value=report.failed.map(f=>'#'+f.id+': '+f.reason).join('\n')
  })
}
function startOAuth(account?:Account){
  if(busy.value)return
  if(account?.auth_kind==='password'||account?.auth_kind==='local'||account?.auth_kind==='pop3')return
  oauthForm.value={provider:account?.auth_kind||'google',client_id:'',desktop_client_secret:'',email:account?.email||form.value.email,name:account?.name||form.value.name,color:account?.color||form.value.color,account_id:account?.id||null}
  error.value='';dialog.value='oauth'
}
async function authorizeOAuth(){
  if(oauthWaiting.value)return
  await run(async()=>{
    oauthWaiting.value=true;oauthCanceling.value=false
    status.value=t('Complete authorization in the system browser.','请在系统浏览器完成授权。')
    try{
      const result=await api.authorize({...oauthForm.value,client_id:oauthForm.value.client_id.trim(),email:oauthForm.value.email.trim(),desktop_client_secret:oauthForm.value.provider==='google'?oauthForm.value.desktop_client_secret:''})
      oauthForm.value.desktop_client_secret='';dialog.value=null
      await refreshAccounts();view(result.id,'folder')
      status.value=t('OAuth authorization and inbox access verified.','OAuth 授权与收件箱访问已验证。')
    }finally{oauthWaiting.value=false;oauthCanceling.value=false}
  })
}
async function cancelOAuth(){
  if(!oauthWaiting.value){closeDialog();return}
  if(oauthCanceling.value)return
  oauthCanceling.value=true
  try{await api.cancelOAuth()}catch(e){error.value=errorText(e);oauthCanceling.value=false}
}
const blankAccount=():NewAccount=>({name:'',email:'',username:'',password:'',imap_host:'',imap_port:993,smtp_host:'',smtp_port:465,smtp_starttls:false,color:'#507ed8',signature:''})
const form=ref<NewAccount>(blankAccount())
const blankCompose=():Compose=>({account_id:sendingAccounts.value.some(a=>a.id===accountId.value)?accountId.value!:sendingAccounts.value[0]?.id||'',to:'',cc:'',bcc:'',subject:'',body:'',html:null,attachments:[],save_sent_copy:false})
const draft=ref<Compose>(blankCompose())
const revision=ref(0),savedRevision=ref(0),autosave=ref(localStorage.getItem('draftAutosave')!=='off')
watch(draft,()=>{revision.value++},{deep:true,flush:'sync'})
watch(autosave,v=>localStorage.setItem('draftAutosave',v?'on':'off'))
const dirty=computed(()=>revision.value!==savedRevision.value)
const attachmentInput=ref<HTMLInputElement|null>(null)
const composeFiles=computed(()=>draft.value.attachments||[])
const rich=computed(()=>draft.value.html!==null&&draft.value.html!==undefined)
const fileSize=(size:number)=>size<1024?size+' B':size<1024*1024?(size/1024).toFixed(1)+' KiB':(size/(1024*1024)).toFixed(1)+' MiB'
const attachedSize=(a:OutgoingAttachment)=>Math.max(0,Math.floor(a.data.length*3/4)-(a.data.endsWith('==')?2:a.data.endsWith('=')?1:0))
const currentAccount=computed(()=>accounts.value.find(a=>a.id===accountId.value))
const currentFolder=computed(()=>folders.value.find(f=>f.account_id===accountId.value&&f.name===folderName.value))
const selectedFolders=computed(()=>['local','pop3'].includes(accounts.value.find(a=>a.id===selected.value?.account_id)?.auth_kind||'')?[]:folders.value.filter(f=>f.account_id===selected.value?.account_id&&f.name!==selected.value?.folder&&f.available))
const archiveFolder=computed(()=>selectedFolders.value.find(f=>f.kind==='archive'))
const trashFolder=computed(()=>selectedFolders.value.find(f=>f.kind==='trash'))
const folderLabel=(f:Folder)=>f.kind==='local'?t('Imported mail','导入邮件'):f.kind==='inbox'?t('Inbox','收件箱'):f.kind==='sent'?t('Sent','已发送'):f.kind==='drafts'?t('Server drafts','服务器草稿'):f.kind==='trash'?t('Trash','废纸篓'):f.kind==='junk'?t('Junk','垃圾邮件'):f.kind==='archive'?t('Archive','归档'):f.label
const title=computed(()=>savedSearches.value.find(s=>s.id===activeSavedId.value)?.name||appliedFilters.value.label||(mode.value==='folder'?(currentFolder.value?folderLabel(currentFolder.value):folderName.value):({inbox:t('All inboxes','全部收件箱'),unread:t('Unread inboxes','未读邮件'),starred:t('Starred','星标邮件'),all:t('All cached mail','全部本地邮件'),drafts:t('Local drafts','本地草稿'),outbox:t('Outbox','发件箱')}[mode.value])))
const searchSeed=computed<SavedSearch>(()=>({id:'',name:'',account_id:['drafts','outbox'].includes(mode.value)?null:accountId.value,mode:['drafts','outbox'].includes(mode.value)?'all':mode.value,folder:folderName.value,query:['drafts','outbox'].includes(mode.value)?'':query.value,filters:['drafts','outbox'].includes(mode.value)?{sender:'',recipient:'',subject:'',label:'',since:null,until:null}:{...appliedFilters.value}}))
const currentOutbox=computed(()=>outbox.value.filter(o=>!query.value||[o.subject,o.recipients,o.state].some(s=>s.toLowerCase().includes(query.value.toLowerCase()))))
const pendingOutbox=computed(()=>outbox.value.filter(o=>o.state==='queued'||o.state==='sending').length)
const outboxState=(o:OutboxItem)=>({queued:t('Queued','待发送'),sending:t('Sending / interrupted','发送中 / 可能中断'),accepted:t('SMTP accepted','SMTP 已接受'),uncertain:t('Uncertain','结果不确定'),canceled:t('Canceled','已取消')}[o.state])
function dateInput(n:number|null,exclusive=false){if(n===null)return '';const d=new Date(n*1000-(exclusive?1:0));return [d.getFullYear(),String(d.getMonth()+1).padStart(2,'0'),String(d.getDate()).padStart(2,'0')].join('-')}
function useSavedSearch(search:SavedSearch){
  organizerTab.value=null;activeSavedId.value=search.id;mode.value=search.mode as View;accountId.value=search.account_id;folderName.value=search.folder
  query.value=search.query;appliedFilters.value={...search.filters};filters.value={sender:search.filters.sender,recipient:search.filters.recipient,subject:search.filters.subject,label:search.filters.label||'',fromDate:dateInput(search.filters.since),toDate:dateInput(search.filters.until,true)};advanced.value=false;void load()
}
function viewLabel(name:string){
  activeSavedId.value='';mode.value='all';accountId.value=null;folderName.value='INBOX';query.value='';filters.value={...blankFilters(),label:name};appliedFilters.value={sender:'',recipient:'',subject:'',label:name,since:null,until:null};advanced.value=false;void load()
}
async function refreshOutbox(){outbox.value=await api.outbox()}
async function refreshOrganizer(){
  const [c,m,s,l]=await Promise.all([api.contacts(),api.templates(),api.searches(),api.labels()])
  contacts.value=c;templates.value=m;savedSearches.value=s;labels.value=l
}
async function organizerChanged(){try{await refreshOrganizer();await load(true,true)}catch(e){error.value=errorText(e)}}
function openOrganizer(tab:OrganizerTab){if(!busy.value)organizerTab.value=tab}
async function dataChanged(){try{await refreshAccounts();await refreshOrganizer();await refreshDrafts();await refreshOutbox();await load(true,true)}catch(e){error.value=errorText(e)}}
async function exportSelected(){const m=selected.value;if(m)await run(async()=>{if(await api.exportEml(m.id))status.value=t('EML exported.','EML 已导出。')})}
async function exportFolder(){await run(async()=>{const count=await api.exportMbox(accountId.value,mode.value==='folder'?folderName.value:mode.value==='inbox'||mode.value==='unread'?'INBOX':null);if(count!==null)status.value=t('Exported messages: ','已导出邮件：')+count})}
async function deleteLocal(){const m=selected.value;if(!m||owner(m.account_id)?.auth_kind!=='local')return;if(!confirm(t('Permanently delete this imported local message? This cannot be undone.','永久删除这封本地导入邮件？此操作不可撤销。')))return;await run(async()=>{await api.deleteLocalMail(m.id);await refreshAccounts();await load()})}
function applyTemplate(){
  const item=templates.value.find(x=>x.id===templateChoice.value);if(!item)return
  if((draft.value.subject||draft.value.body)&&!confirm(t('Replace subject and body with this template? Recipients and attachments are kept.','使用模板替换主题和正文？收件人和附件将保留。')))return
  draft.value.subject=item.subject;draft.value.body=item.body;draft.value.html=item.html
}
async function saveCurrentTemplate(){await run(async()=>{
  await api.saveTemplate({id:'',name:templateName.value,subject:draft.value.subject,body:draft.value.body,html:draft.value.html??null})
  templates.value=await api.templates();templateName.value='';status.value=t('Template saved.','模板已保存。')
})}
function newRequestId(){
  if(typeof crypto.randomUUID==='function')return crypto.randomUUID()
  const bytes=crypto.getRandomValues(new Uint8Array(16));bytes[6]=(bytes[6]&15)|64;bytes[8]=(bytes[8]&63)|128
  const hex=Array.from(bytes,b=>b.toString(16).padStart(2,'0')).join('');return hex.slice(0,8)+'-'+hex.slice(8,12)+'-'+hex.slice(12,16)+'-'+hex.slice(16,20)+'-'+hex.slice(20)
}
async function cancelQueued(o:OutboxItem){if(confirm(t('Cancel this queued message?','取消这封待发送邮件？')))await run(async()=>{await api.cancelOutbox(o.id);await refreshOutbox()})}
async function resolveInterrupted(o:OutboxItem){if(confirm(t('Only continue if no other Mail EX instance is sending this message. Marking it uncertain does not resend it.','请确认没有其他 Mail EX 实例正在发送此邮件。标为不确定不会再次发送。')))await run(async()=>{await api.interruptOutbox(o.id);await refreshOutbox()})}
async function copyQueued(o:OutboxItem){
  if(o.state==='uncertain'&&!confirm(t('Verify whether the original message was delivered before creating another send request. Copy to a draft?','请先核对原邮件是否已经发送成功，再创建新的发送请求。复制为草稿？')))return
  await run(async()=>{await beginCompose(await api.copyOutbox(o.id))})
}
async function removeQueued(o:OutboxItem){if(confirm(t('Delete this local outbox record? Server mail is unchanged.','删除此本地发件箱记录？服务器邮件不受影响。')))await run(async()=>{await api.deleteOutbox(o.id);await refreshOutbox()})}

const currentDrafts=computed(()=>drafts.value.filter(d=>!query.value||[d.mail.subject,d.mail.to,d.mail.body].some(s=>s.toLowerCase().includes(query.value.toLowerCase()))))
const totalUnread=computed(()=>folders.value.filter(f=>f.kind==='inbox').reduce((n,f)=>n+f.unread,0))
const errorText=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:String(e)
const owner=(id:string)=>accounts.value.find(a=>a.id===id)
const folderOf=(m:Summary)=>folders.value.find(f=>f.account_id===m.account_id&&f.name===m.folder)
const date=(n:number)=>n?new Date(n*1000).toLocaleString(zh.value?'zh-CN':undefined,{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}):''
let generation=0,detailGeneration=0,lastAuto=Date.now(),timer:number|undefined,draftTimer:number|undefined
async function refreshAccounts(){
  const old=new Set(accounts.value.map(a=>a.id))
  accounts.value=await api.accounts()
  expanded.value=expanded.value.filter(id=>accounts.value.some(a=>a.id===id))
  for(const a of accounts.value)if(!old.has(a.id))expanded.value.push(a.id)
  folders.value=await api.folders();labels.value=await api.labels()
}
async function refreshDrafts(){drafts.value=await api.drafts()}
async function load(reset=true,preserve=false){
  const token=++generation
  const previous=preserve?selected.value:null
  if(reset){offset.value=0;selected.value=null;invitations.value=[];messageLabels.value=[];preview.value=null;batchIds.value=[];batchDestination.value='';detailGeneration++}
  if(mode.value==='outbox'){messages.value=[];more.value=false;loading.value=false;try{await refreshOutbox()}catch(e){error.value=errorText(e)}return}
  if(mode.value==='drafts'){messages.value=[];more.value=false;loading.value=false;try{await refreshDrafts()}catch(e){error.value=errorText(e)}return}
  loading.value=true
  try{
    const folder=mode.value==='folder'?folderName.value:mode.value==='inbox'||mode.value==='unread'?'INBOX':null
    const result=await api.messages(accountId.value,folder,mode.value==='unread',mode.value==='starred',query.value,reset?0:offset.value,appliedFilters.value)
    if(token!==generation)return
    messages.value=reset?result:[...messages.value,...result]
    offset.value=messages.value.length;more.value=result.length===100
    if(previous&&messages.value.some(m=>m.id===previous.id))await open(previous)
  }catch(e){if(token===generation)error.value=errorText(e)}
  finally{if(token===generation)loading.value=false}
}
async function run(fn:()=>Promise<void>){
  if(busy.value)return
  error.value='';status.value='';busy.value=true
  try{await fn()}catch(e){error.value=errorText(e)}finally{busy.value=false}
}
function view(id:string|null,kind:View='inbox',folder='INBOX'){
  activeSavedId.value='';accountId.value=id;mode.value=kind;folderName.value=folder;query.value='';filters.value=blankFilters();appliedFilters.value={sender:'',recipient:'',subject:'',since:null,until:null};advanced.value=false;void load()
}
function toggleExpand(id:string){
  expanded.value=expanded.value.includes(id)?expanded.value.filter(x=>x!==id):[...expanded.value,id]
}
function newAccount(){incomingProtocol.value='imap';form.value=blankAccount();preset.value='custom';editId.value=null;error.value='';dialog.value='account'}
function editAccount(){
  const a=currentAccount.value;if(!a)return
  incomingProtocol.value=a.auth_kind==='pop3'?'pop3':'imap';const {id,auth_kind:_,...data}=a;editId.value=id;form.value={...data,password:''};preset.value='custom';error.value='';dialog.value='account'
}
function changeIncoming(){preset.value='custom';form.value.imap_host='';form.value.imap_port=incomingProtocol.value==='pop3'?995:993}
function applyPreset(){
  const p:Record<string,[string,string,number,boolean]>={qq:['imap.qq.com','smtp.qq.com',465,false],net163:['imap.163.com','smtp.163.com',465,false],net126:['imap.126.com','smtp.126.com',465,false],icloud:['imap.mail.me.com','smtp.mail.me.com',587,true],gmail:['imap.gmail.com','smtp.gmail.com',465,false]}
  const v=p[preset.value]
  if(v){[form.value.imap_host,form.value.smtp_host,form.value.smtp_port,form.value.smtp_starttls]=v;form.value.imap_port=993}
}
function closeDialog(){
  if(busy.value)return
  if(detachedCompose){void getCurrentWindow().close();return}
  if(dialog.value==='compose'&&dirty.value&&!confirm(t('Close without saving the latest changes?','关闭且不保存最近的修改？')))return
  form.value.password='';oauthForm.value.desktop_client_secret='';dialog.value=null
}
async function saveAccount(){await run(async()=>{
  const input={...form.value,username:form.value.username.trim()||form.value.email.trim(),name:form.value.name.trim()||form.value.email.trim()}
  const a=editId.value?await api.update(editId.value,input):incomingProtocol.value==='pop3'?await api.addPop(input):await api.add(input)
  form.value=blankAccount();dialog.value=null;await refreshAccounts();view(a.id,'folder');status.value=t('Account saved. Test or sync to verify access.','账号已保存，请测试连接或同步。')
})}
async function testAccount(){const id=accountId.value;if(id)await run(async()=>{status.value=await api.test(id)})}
async function sync(all=false,automatic=false){await run(async()=>{
  const active=currentAccount.value,targets=(automatic?sendingAccounts.value:active?[active]:sendingAccounts.value).filter(a=>a.auth_kind!=='local')
  if(!targets.length){if(!automatic&&active?.auth_kind!=='local')newAccount();return}
  const targetFolder=!automatic&&mode.value==='folder'?folderName.value:null
  let total=0,count=0;const failures:string[]=[]
  for(const a of targets){
    status.value=t('Syncing ','正在同步 ')+a.name
    try{const result=await api.sync(a.id,targetFolder,all);total+=result.added;count+=result.folders}
    catch(e){failures.push(a.name+': '+errorText(e))}
  }
  lastAuto=Date.now()
  await refreshAccounts();await load(true,true)
  status.value=t('Synced folders: ','已同步文件夹：')+count+' · '+t('New cached messages: ','新增缓存邮件：')+total
  if(failures.length)error.value=failures.join('\n')
})}
async function remove(){const a=currentAccount.value;if(!a||!confirm(t('Remove this account and its local cache? Server mail is not deleted.','移除此账号及本地缓存？不会删除服务器邮件。')))return;await run(async()=>{await api.remove(a.id);await refreshAccounts();await refreshDrafts();await refreshOrganizer();await refreshOutbox();view(null)})}
async function open(m:Summary){const token=++detailGeneration;try{const [result,categories,meetings]=await Promise.all([api.message(m.id),api.messageLabels(m.id),api.invitations(m.id).catch(()=>[] as Invitation[])]);if(token===detailGeneration){selected.value=result;invitations.value=meetings;messageLabels.value=categories;destination.value='';readMode.value=result.html!==null?'html':'plain';preview.value=null}}catch(e){if(token===detailGeneration)error.value=errorText(e)}}
async function toggleSeen(){const m=selected.value;if(!m)return;await run(async()=>{await api.seen(m.id,!m.seen);await refreshAccounts();await load(true,true)})}
async function toggleStar(){const m=selected.value;if(!m)return;await run(async()=>{await api.star(m.id,!m.starred);await load(true,true)})}
async function moveTo(name:string){
  const m=selected.value;const dest=selectedFolders.value.find(f=>f.name===name);if(!m||!dest)return
  if(!confirm(t('Move the message to ','将邮件移动至 ')+folderLabel(dest)+'?'))return
  await run(async()=>{
    await api.move(m.id,name);selected.value=null;await load()
    status.value=t('Moved on server.','已在服务器移动。')
    try{await api.sync(m.account_id,name,false);await refreshAccounts();await load()}
    catch(e){error.value=t('Move succeeded, but destination refresh failed: ','移动成功，但目标文件夹刷新失败：')+errorText(e)}
  })
}
function escapeHtml(value:string){return value.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/\r?\n/g,'<br>')}
function startCompose(mail:Compose,id:string|null=null){
  draft.value={...mail,attachments:[...(mail.attachments||[])],html:mail.html??null,save_sent_copy:mail.save_sent_copy??false}
  draftId.value=id;savedRevision.value=revision.value;dialog.value='compose';error.value='';templateChoice.value='';templateName.value='';scheduleEnabled.value=false;scheduleAt.value='';deliveryRequestId.value=newRequestId()
}
async function beginCompose(mail:Compose,id:string|null=null){
  if(detachedCompose||(!separateCompose.value&&!id)){startCompose(mail,id);return}
  const saved=id||await api.saveDraft(null,mail)
  try{await api.openComposer(saved)}catch(e){startCompose(mail,saved);error.value=errorText(e);return}
  try{await refreshDrafts()}catch(e){error.value=errorText(e)}
}
function compose(){
  if(busy.value)return
  if(!sendingAccounts.value.length){newAccount();return}
  const mail=blankCompose(),sig=owner(mail.account_id)?.signature
  if(sig)mail.body='\n\n'+sig
  void run(()=>beginCompose(mail))
}
async function editDraft(d:Draft){await run(async()=>{await api.openComposer(d.id)})}
async function reply(kind:'reply'|'reply_all'|'forward'){
  const id=selected.value?.id;if(id!==undefined)await run(async()=>{
    const mail=await api.composeFrom(id,kind),sig=owner(mail.account_id)?.signature
    if(sig)mail.body='\n\n'+sig+mail.body
    await beginCompose(mail)
  })
}
function handleCompose(mail:Compose){void run(()=>beginCompose(mail))}
async function detachCurrent(){if(detachedCompose||dialog.value!=='compose')return;await run(async()=>{
  await persistDraft(false);if(!draftId.value)return
  await api.openComposer(draftId.value);dialog.value=null;draftId.value=null
})}
async function detachReader(m:Summary){if(busy.value)return;await run(()=>api.openReader(m.id))}
function changeFormat(value:'plain'|'rich'){
  if(value==='rich'&&!rich.value)draft.value.html=escapeHtml(draft.value.body)
  if(value==='plain'&&rich.value){
    if(!confirm(t('Switch to plain text and remove formatting?','切换为纯文本并移除格式？')))return
    draft.value.html=null
  }
}
function richUpdate(value:{html:string;body:string}){draft.value.html=value.html;draft.value.body=value.body}
function insertSignature(){
  const sig=owner(draft.value.account_id)?.signature
  if(!sig)return
  if(draft.value.body.includes(sig)&&!confirm(t('This signature already appears in the message. Insert again?','正文已包含签名，仍要再次插入？')))return
  draft.value.body+='\n\n'+sig
  if(rich.value)draft.value.html=(draft.value.html||'')+'<br><br>'+escapeHtml(sig)
}
function readFile(file:File):Promise<OutgoingAttachment>{
  return new Promise((resolve,reject)=>{
    const reader=new FileReader()
    reader.onerror=()=>reject(new Error(t('Cannot read selected file','无法读取选定文件')))
    reader.onload=()=>{
      const url=String(reader.result||''),comma=url.indexOf(',')
      if(comma<0){reject(new Error(t('Invalid file data','无效文件数据')));return}
      resolve({name:file.name,media_type:file.type||'application/octet-stream',data:url.slice(comma+1)})
    }
    reader.readAsDataURL(file)
  })
}
async function addFiles(event:Event){
  const input=event.target as HTMLInputElement
  const files=Array.from(input.files||[]);input.value=''
  if(!files.length)return
  await run(async()=>{
    for(const file of files){
      const a=await readFile(file)
      draft.value.attachments=[...(draft.value.attachments||[]),a]
    }
  })
}
function removeAttachment(index:number){draft.value.attachments=composeFiles.value.filter((_,i)=>i!==index)}
async function viewAttachment(index:number){
  const id=selected.value?.id;if(id===undefined)return
  await run(async()=>{const p=await api.preview(id,index);if(selected.value?.id===id)preview.value=p})
}
async function downloadAttachment(index:number){
  const id=selected.value?.id;if(id===undefined)return
  await run(async()=>{if(await api.download(id,index))status.value=t('Attachment saved.','附件已保存。')})
}
async function persistDraft(close:boolean){
  const current=revision.value
  const snapshot:Compose={...draft.value,attachments:[...(draft.value.attachments||[])]}
  draftId.value=await api.saveDraft(draftId.value,snapshot)
  savedRevision.value=current
  await refreshDrafts()
  if(close&&revision.value===current){if(detachedCompose){allowClose=true;await getCurrentWindow().close()}else dialog.value=null}
  status.value=t('Draft saved locally.','草稿已保存到本地。')
}
async function saveDraft(){await run(async()=>{await persistDraft(true)})}
async function deleteDraft(d:Draft){
  if(!confirm(t('Delete this local draft?','删除这份本地草稿？')))return
  await run(async()=>{await api.deleteDraft(d.id);await refreshDrafts()})
}
async function send(){
  let due=0
  if(scheduleEnabled.value){
    due=Math.floor(new Date(scheduleAt.value).getTime()/1000)
    if(!Number.isFinite(due)||due<=Math.floor(Date.now()/1000)){error.value=t('Choose a future send time.','请选择未来的发送时间。');return}
    if(!confirm(t('Queue this message for automatic delivery at the selected time while Mail EX is running?','将邮件加入队列，并在 Mail EX 运行期间于指定时间自动发送？')))return
  }
  await run(async()=>{
    const id=deliveryRequestId.value
    const item=await api.enqueue(id,{...draft.value,attachments:[...(draft.value.attachments||[])]},due,draftId.value)
    dialog.value=null;draftId.value=null;draft.value=blankCompose();savedRevision.value=revision.value
    if(detachedCompose){allowClose=true;await getCurrentWindow().close();return}
    await refreshDrafts();await refreshOutbox();view(null,'outbox')
    status.value=due?t('Scheduled in the local outbox.','已加入本地定时发送队列。'):t('Queued for delivery.','已加入发件队列。')
    if(!due&&item.state==='queued'){
      const report=await api.dispatch(item.id)
      if(report?.state==='accepted')status.value=t('Accepted by SMTP server.','SMTP 服务器已接受邮件。')
      if(report?.state==='uncertain')error.value=t('Delivery is uncertain; verify before resending.','发送结果不确定，请核对后再发送。')
      if(report?.sent_copy==='failed_or_uncertain')error.value=t('Message accepted; Sent copy failed or is uncertain. Do not send again just to save a copy.','邮件已被接受，但已发送副本保存失败或结果不确定。请勿为保存副本重新发送。')
      await refreshOutbox()
    }
  })
}
function resize(event:PointerEvent,which:'nav'|'list'){
  event.preventDefault()
  const start=event.clientX,initial=which==='nav'?navWidth.value:listWidth.value
  const move=(e:PointerEvent)=>{const value=Math.max(which==='nav'?180:260,Math.min(which==='nav'?440:650,initial+e.clientX-start));if(which==='nav')navWidth.value=value;else listWidth.value=value}
  const done=()=>{document.removeEventListener('pointermove',move);document.removeEventListener('pointerup',done);document.removeEventListener('pointercancel',done)}
  document.addEventListener('pointermove',move);document.addEventListener('pointerup',done);document.addEventListener('pointercancel',done)
}
function preferenceChanged(e:StorageEvent){
  if(e.key==='theme')theme.value=e.newValue||'system'
  if(e.key==='language')language.value=e.newValue||'system'
  if(e.key==='density')density.value=e.newValue||'comfortable'
  if(e.key==='readerFontSize')readerFontSize.value=Number(e.newValue||14)
  if(e.key==='separateCompose')separateCompose.value=e.newValue!=='off'
  if(e.key==='autoMinutes')autoMinutes.value=Math.max(0,Number(e.newValue||0)||0)
  if(e.key==='draftAutosave')autosave.value=e.newValue!=='off'
}
function keyboard(e:KeyboardEvent){
  const mod=e.ctrlKey||e.metaKey
  if(e.repeat)return
  if(mod&&e.key.toLowerCase()==='s'&&dialog.value==='compose'){e.preventDefault();if(!busy.value)void saveDraft();return}
  if(detachedCompose)return
  if(mod&&e.key.toLowerCase()==='n'&&!dialog.value&&!calendarPanel.value&&!dataPanel.value&&!organizerTab.value&&!settingsPanel.value){e.preventDefault();compose()}
  if(mod&&e.key.toLowerCase()==='f'&&!dialog.value&&!settingsPanel.value){e.preventDefault();searchInput.value?.focus();searchInput.value?.select()}
  if(mod&&e.key===','&&!dialog.value&&!busy.value){e.preventDefault();settingsPanel.value=true}
  if(e.key==='F5'&&!busy.value&&!dialog.value&&!settingsPanel.value&&!calendarPanel.value&&!organizerTab.value&&!dataPanel.value){e.preventDefault();void sync()}
}
function resetLayout(){navWidth.value=234;listWidth.value=355;readerFontSize.value=14;density.value='comfortable'}
onMounted(async()=>{
  window.addEventListener('storage',preferenceChanged)
  window.addEventListener('keydown',keyboard)
  try{
    await refreshAccounts();await refreshOrganizer()
    if(detachedCompose&&detachedDraftId){
      const item=await api.draft(detachedDraftId)
      startCompose(item.mail,item.id)
      unlistenClose=await getCurrentWindow().onCloseRequested(e=>{
        if(allowClose)return
        if(busy.value){e.preventDefault();return}
        if(dirty.value){
          e.preventDefault()
          if(confirm(t('Close without saving the latest changes? The last saved draft is retained.','关闭且不保存最近修改？已保存的草稿仍会保留。'))){allowClose=true;void getCurrentWindow().close()}
        }
      })
    }else{
      await refreshDrafts();await refreshOutbox();await load()
      unlistenOutbox=await listen<string>('outbox-changed',()=>{void refreshOutbox().catch(e=>{error.value=errorText(e)})})
      unlistenDrafts=await listen('drafts-changed',()=>{void refreshDrafts().catch(e=>{error.value=errorText(e)})})
      unlistenReminder=await listen<number>('calendar-reminder',e=>{status.value=t('Calendar reminders: ','日程提醒：')+e.payload})
      timer=window.setInterval(()=>{if(autoMinutes.value>0&&!busy.value&&!loading.value&&!dialog.value&&!organizerTab.value&&!dataPanel.value&&!calendarPanel.value&&!settingsPanel.value&&Date.now()-lastAuto>=autoMinutes.value*60000)void sync(false,true)},15000)
    }
    await api.frontendReady()
  }catch(e){error.value=errorText(e);if(detachedCompose)startupError.value=error.value}
  draftTimer=window.setInterval(()=>{if(autosave.value&&dialog.value==='compose'&&dirty.value&&!busy.value)void run(()=>persistDraft(false))},5000)
})
onUnmounted(()=>{if(timer!==undefined)clearInterval(timer);if(draftTimer!==undefined)clearInterval(draftTimer);unlistenOutbox?.();unlistenReminder?.();unlistenDrafts?.();unlistenClose?.();window.removeEventListener('storage',preferenceChanged);window.removeEventListener('keydown',keyboard)})
</script>

<template>
<div class="shell" :data-theme="theme" :data-density="density" :class="{'standalone-compose':detachedCompose}" :style="{'--reader-font':readerFontSize+'px'}">
  <header v-if="!detachedCompose" class="top">
    <div class="brand"><span class="logo">✉</span> Mail EX</div>
    <form class="search" @submit.prevent="applySearch"><input ref="searchInput" v-model="query" :placeholder="t('Search subject, sender, recipient, body','搜索主题、发件人、收件人、正文')" :aria-label="t('Search','搜索')"><button>{{t('Search','搜索')}}</button><button v-if="mode!=='drafts'&&mode!=='outbox'" type="button" :class="{active:advanced}" @click="advanced=!advanced">{{t('Filters','筛选')}}</button></form>
    <div class="top-actions">
      <button :disabled="busy" @click="sync()">↻ {{t('Sync','同步')}}</button>
      <button :disabled="busy" @click="settingsPanel=true">⚙ {{t('Settings','设置')}}</button>
    </div>
  </header>
  <main v-if="!detachedCompose" class="workspace" :style="{gridTemplateColumns:navWidth+'px 5px '+listWidth+'px 5px minmax(0,1fr)'}">
    <aside class="navigation">
      <button class="primary compose" @click="compose">＋ {{t('New message','写邮件')}}</button>
      <div class="section">{{t('UNIFIED MAIL','统一邮箱')}}</div>
      <button class="nav-row" :class="{active:mode==='inbox'}" @click="view(null,'inbox')">▤ <span class="nav-label">{{t('All inboxes','全部收件箱')}}</span><small v-if="totalUnread" class="count">{{totalUnread}}</small></button>
      <button class="nav-row" :class="{active:mode==='unread'}" @click="view(null,'unread')">● <span class="nav-label">{{t('Unread','未读邮件')}}</span></button>
      <button class="nav-row" :class="{active:mode==='starred'}" @click="view(null,'starred')">★ <span class="nav-label">{{t('Starred','星标邮件')}}</span></button>
      <button class="nav-row" :class="{active:mode==='all'}" @click="view(null,'all')">▧ <span class="nav-label">{{t('All cached mail','全部本地邮件')}}</span></button>
      <button class="nav-row" :class="{active:mode==='drafts'}" @click="view(null,'drafts')">✎ <span class="nav-label">{{t('Local drafts','本地草稿')}}</span><small v-if="drafts.length" class="count">{{drafts.length}}</small></button>
      <button class="nav-row" :class="{active:mode==='outbox'}" @click="view(null,'outbox')">↗ <span class="nav-label">{{t('Outbox','发件箱')}}</span><small v-if="pendingOutbox" class="count">{{pendingOutbox}}</small></button>
      <div class="section accounts-header">{{t('ORGANIZE','邮件管理')}}</div>
      <button class="nav-row" @click="openOrganizer('contacts')" :disabled="busy">♙ <span class="nav-label">{{t('Contacts','联系人')}}</span></button>
      <button class="nav-row" @click="openOrganizer('templates')" :disabled="busy">▧ <span class="nav-label">{{t('Templates','邮件模板')}}</span></button>
      <button class="nav-row" @click="openOrganizer('rules')" :disabled="busy">⚙ <span class="nav-label">{{t('Mail rules','邮件规则')}}</span></button>
      <button class="nav-row" @click="openOrganizer('searches')" :disabled="busy">⌕ <span class="nav-label">{{t('Saved searches','保存的搜索')}}</span></button>
      <button v-for="s in savedSearches" :key="'saved-'+s.id" class="nav-row folder-row" :class="{active:activeSavedId===s.id}" @click="useSavedSearch(s)"><span class="nav-label">{{s.name}}</span></button>
      <button v-for="l in labels" :key="'label-'+l.name" class="nav-row folder-row" :class="{active:!activeSavedId&&appliedFilters.label===l.name}" @click="viewLabel(l.name)"><span class="folder-symbol">◈</span><span class="nav-label">{{l.name}}</span><small class="count">{{l.unread||l.total}}</small></button>
      <div class="section accounts-header">{{t('ACCOUNTS','邮箱账号')}}<button @click="newAccount" :title="t('Add account','添加账号')">＋</button></div>
      <div v-for="a in accounts" :key="a.id" class="account-group">
        <div class="account-heading"><button class="expand" @click="toggleExpand(a.id)" :aria-expanded="expanded.includes(a.id)" :aria-label="t('Toggle folders','展开或收起文件夹')">{{expanded.includes(a.id)?'⌄':'›'}}</button>
          <button class="nav-row account" :class="{active:accountId===a.id&&mode==='folder'&&folderName===(a.auth_kind==='local'?'Imported':'INBOX')}" @click="view(a.id,'folder',a.auth_kind==='local'?'Imported':'INBOX')"><span class="dot" :style="{background:a.color}"></span><span class="account-text">{{a.auth_kind==='local'?t('Local archive','本地归档'):a.name}}<small>{{a.auth_kind==='local'?t('Offline mail','离线邮件'):a.email}}</small></span></button>
        </div>
        <template v-if="expanded.includes(a.id)">
          <button v-for="f in folders.filter(f=>f.account_id===a.id)" :key="f.name" class="nav-row folder-row" :class="{active:accountId===a.id&&mode==='folder'&&folderName===f.name,unavailable:!f.available}" @click="view(a.id,'folder',f.name)" :title="f.available?f.label:t('Unavailable on server; cached mail retained','服务器文件夹不可用，已保留缓存')"><span class="folder-symbol">{{f.kind==='inbox'?'▤':f.kind==='sent'?'↗':f.kind==='drafts'?'✎':f.kind==='trash'?'♧':f.kind==='archive'?'▣':'▱'}}</span><span class="nav-label">{{folderLabel(f)}}</span><small v-if="f.unread" class="count">{{f.unread}}</small></button>
        </template>
      </div>
      <div v-if="!accounts.length" class="hint">{{t('Add your first mailbox to get started.','添加第一个邮箱以开始使用。')}}</div>
      <button class="add" @click="newAccount">＋ {{t('Add account','添加账号')}}</button>
      <button class="nav-row" @click="calendarPanel=true" :disabled="busy">▦ <span class="nav-label">{{t('Calendar','日历')}}</span></button>
      <button class="nav-row" @click="dataPanel=true" :disabled="busy"><span class="folder-symbol">⚙</span><span class="nav-label">{{t('Data management','数据管理')}}</span></button>
      <div class="nav-footer">{{t('Local-first · No Mail EX cloud','本地优先 · 无强制云服务')}}</div>
    </aside>
    <div class="split" role="separator" aria-orientation="vertical" @pointerdown="resize($event,'nav')"></div>
    <section class="message-list">
      <div class="list-head"><h1>{{title}}</h1><div class="toolbar">
        <template v-if="mode!=='drafts'&&mode!=='outbox'"><button v-if="currentAccount?.auth_kind!=='local'" @click="sync()" :disabled="busy||currentFolder?.available===false">↻ {{t('Sync','同步')}}</button><button v-if="currentAccount&&currentAccount.auth_kind!=='local'" @click="sync(true)" :disabled="busy">{{t('Sync all folders','同步全部文件夹')}}</button><button v-if="currentAccount&&currentAccount.auth_kind!=='local'" @click="testAccount" :disabled="busy">{{t('Test','测试')}}</button><button v-if="currentAccount&&currentAccount.auth_kind!=='local'" @click="editAccount" :disabled="busy">{{t('Edit account','编辑账号')}}</button><button v-if="currentAccount&&currentAccount.auth_kind!=='password'&&currentAccount.auth_kind!=='local'&&currentAccount.auth_kind!=='pop3'" @click="startOAuth(currentAccount)" :disabled="busy">{{t('Reauthorize','重新授权')}}</button><button v-if="currentAccount&&currentAccount.auth_kind!=='local'" class="danger-text" @click="remove" :disabled="busy">{{t('Remove','移除')}}</button><button :disabled="busy" @click="exportFolder">{{t('Export folder / scope (MBOX)','导出文件夹／范围 (MBOX)')}}</button></template>
        <template v-else-if="mode==='outbox'"><button :disabled="busy" @click="run(refreshOutbox)">↻ {{t('Refresh','刷新')}}</button><button @click="compose">{{t('New message','写邮件')}}</button></template>
        <button v-else @click="compose">{{t('New draft','新建草稿')}}</button>
      </div></div>
      <form v-if="advanced&&mode!=='drafts'&&mode!=='outbox'" class="advanced-search" @submit.prevent="applySearch">
        <label>{{t('Sender','发件人')}}<input v-model="filters.sender"></label><label>{{t('Recipient','收件人')}}<input v-model="filters.recipient"></label>
        <label>{{t('Subject','主题')}}<input v-model="filters.subject"></label><label>{{t('Category','分类')}}<select v-model="filters.label"><option value="">{{t('All categories','全部分类')}}</option><option v-for="l in labels" :key="l.name" :value="l.name">{{l.name}}</option></select></label>
        <div class="search-dates"><label>{{t('From','开始日期')}}<input v-model="filters.fromDate" type="date"></label><label>{{t('Through','结束日期')}}<input v-model="filters.toDate" type="date"></label></div>
        <div class="search-filter-actions"><button type="button" @click="clearSearch">{{t('Clear','清除')}}</button><button class="primary">{{t('Apply filters','应用筛选')}}</button></div>
      </form>
      <div class="list-meta">{{mode==='outbox'?currentOutbox.length:mode==='drafts'?currentDrafts.length:messages.length}} {{t('loaded','封已加载')}} <span v-if="loading">{{t('Loading…','加载中…')}}</span></div>
      <div v-if="mode!=='drafts'&&mode!=='outbox'&&messages.length" class="batch-toolbar">
        <label class="batch-select"><input type="checkbox" :checked="messages.every(m=>batchIds.includes(m.id))" :disabled="busy" @change="toggleAllBatch">{{batchIds.length?t('Selected ','已选 ')+batchIds.length:t('Select loaded','选择已加载')}}</label>
        <template v-if="batchIds.length">
          <button :disabled="busy" @click="applyBatch('read')">{{t('Read','已读')}}</button><button :disabled="busy" @click="applyBatch('unread')">{{t('Unread','未读')}}</button>
          <button :disabled="busy" @click="applyBatch('star')">{{t('Star','星标')}}</button><button :disabled="busy" @click="applyBatch('unstar')">{{t('Unstar','取消星标')}}</button>
          <button :disabled="busy" @click="applyBatch('archive')">{{t('Archive','归档')}}</button><button class="danger-text" :disabled="busy" @click="applyBatch('trash')">{{t('Trash','废纸篓')}}</button>
          <div v-if="batchFolders.length" class="batch-move"><select v-model="batchDestination" :disabled="busy" :aria-label="t('Move destination','移动目标')"><option value="">{{t('Choose folder','选择文件夹')}}</option><option v-for="f in batchFolders" :key="f.name" :value="f.name">{{folderLabel(f)}}</option></select><button :disabled="busy||!batchDestination" @click="applyBatch('move')">{{t('Move','移动')}}</button></div>
        </template>
      </div>
      <div class="items" v-if="mode==='outbox'">
        <div class="outbox-note">{{t('Scheduled messages send when due while Mail EX is running, including minimized. Overdue queued mail sends on the next launch. Uncertain or interrupted sends are never automatically retried.','Mail EX 运行期间（含最小化）会发送到期邮件；关机或退出期间到期的邮件在下次启动时处理。结果不确定或中断的邮件绝不自动重发。')}}</div>
        <div v-for="o in currentOutbox" :key="o.id" class="outbox-row">
          <div class="outbox-heading"><strong>{{o.subject||t('(No subject)','（无主题）')}}</strong><span class="outbox-state" :data-state="o.state">{{outboxState(o)}}</span></div>
          <small>{{o.recipients}} · {{owner(o.account_id)?.name||o.account_id}}</small><small>{{o.due_at?date(o.due_at):t('Immediate delivery','立即发送')}}</small>
          <p v-if="o.detail" class="hint">{{o.detail}}</p>
          <div class="outbox-actions"><button v-if="o.state==='queued'" :disabled="busy" @click="cancelQueued(o)">{{t('Cancel','取消')}}</button><button v-if="o.state==='sending'" :disabled="busy" @click="resolveInterrupted(o)">{{t('Mark interrupted','标为中断')}}</button><template v-if="['accepted','uncertain','canceled'].includes(o.state)"><button :disabled="busy" @click="copyQueued(o)">{{t('Copy to draft','复制为草稿')}}</button><button class="danger-text" :disabled="busy" @click="removeQueued(o)">{{t('Delete record','删除记录')}}</button></template></div>
        </div>
        <div class="empty" v-if="!currentOutbox.length"><span>↗</span><p>{{t('No outbox records','没有发件箱记录')}}</p></div>
      </div>
      <div class="items" v-else-if="mode==='drafts'">
        <div v-for="d in currentDrafts" :key="d.id" class="draft-row"><button class="mail-row" @click="editDraft(d)" :disabled="busy"><div class="mail-line"><strong>{{d.mail.subject||t('(No subject)','（无主题）')}}</strong><time>{{date(d.updated_at)}}</time></div><div class="subject">{{d.mail.to||t('No recipient','未填写收件人')}}</div><small>{{owner(d.mail.account_id)?.name}}</small></button><button class="delete-draft" @click="deleteDraft(d)" :disabled="busy" :title="t('Delete local draft','删除本地草稿')">×</button></div>
        <div class="empty" v-if="!currentDrafts.length"><span>✎</span><p>{{t('No local drafts','没有本地草稿')}}</p></div>
      </div>
      <div v-else class="items">
        <div v-for="m in messages" :key="m.id" class="mail-select-row" :class="{checked:batchIds.includes(m.id)}"><label class="mail-checkbox" :title="t('Select message','选择邮件')"><input type="checkbox" :checked="batchIds.includes(m.id)" :disabled="busy" :aria-label="t('Select message','选择邮件')" @change="toggleBatch(m.id)"></label><button class="mail-row" :class="{selected:selected?.id===m.id,unseen:!m.seen}" @click="open(m)" @dblclick="detachReader(m)">
          <div class="mail-line"><span class="dot" :style="{background:owner(m.account_id)?.color||'#aaa'}"></span><strong>{{m.sender||t('(Unknown sender)','（未知发件人）')}}</strong><span v-if="m.starred" class="star">★</span><time>{{date(m.timestamp)}}</time></div>
          <div class="subject">{{m.subject||t('(No subject)','（无主题）')}}</div><small>{{owner(m.account_id)?.name}}<template v-if="m.folder!=='INBOX'"> · {{folderOf(m)?.label||m.folder}}</template></small>
        </button></div>
        <div class="empty" v-if="!messages.length&&!loading"><span>✉</span><p>{{t('No cached messages','没有本地邮件')}}</p><button v-if="sendingAccounts.length&&currentAccount?.auth_kind!=='local'&&currentFolder?.available!==false" @click="sync()" :disabled="busy">{{t('Sync mail','同步邮件')}}</button></div>
        <button v-if="more" class="load-more" @click="load(false)" :disabled="loading">{{t('Load more','加载更多')}}</button>
      </div>
    </section>
    <div class="split" role="separator" aria-orientation="vertical" @pointerdown="resize($event,'list')"></div>
    <section class="reading">
      <template v-if="selected">
        <div class="reader-toolbar">
          <button @click="reply('reply')" :disabled="busy||!sendingAccounts.length">{{t('Reply','回复')}}</button><button @click="reply('reply_all')" :disabled="busy||!sendingAccounts.length">{{t('Reply all','回复全部')}}</button><button @click="reply('forward')" :disabled="busy||!sendingAccounts.length">{{t('Forward','转发')}}</button><button @click="detachReader(selected)" :disabled="busy">{{t('Open window','独立窗口')}}</button><button @click="exportSelected" :disabled="busy">{{t('Export EML','导出 EML')}}</button><button v-if="owner(selected.account_id)?.auth_kind==='local'" class="danger-text" @click="deleteLocal" :disabled="busy">{{t('Delete local mail','删除本地邮件')}}</button>
          <button @click="toggleStar" :disabled="busy">{{selected.starred?t('Unstar','取消星标'):t('Star','星标')}}</button><button @click="toggleSeen" :disabled="busy">{{selected.seen?t('Mark unread','标为未读'):t('Mark read','标为已读')}}</button>
          <button v-if="archiveFolder" @click="moveTo(archiveFolder.name)" :disabled="busy">{{t('Archive','归档')}}</button><button v-if="trashFolder" class="danger-text" @click="moveTo(trashFolder.name)" :disabled="busy">{{t('Move to trash','移至废纸篓')}}</button>
          <div v-if="selectedFolders.length" class="move-control"><select v-model="destination" :disabled="busy" :aria-label="t('Destination folder','目标文件夹')"><option value="">{{t('Choose folder','选择文件夹')}}</option><option v-for="f in selectedFolders" :key="f.name" :value="f.name">{{folderLabel(f)}}</option></select><button @click="moveTo(destination)" :disabled="busy||!destination">{{t('Move','移动')}}</button></div>
        </div>
        
        <article>
          <InvitationPanel v-if="invitations.length" :zh="zh" :message-id="selected.id" :invitations="invitations" :accounts="accounts" :disabled="busy" @compose="handleCompose" @status="status=$event" @error="error=$event"/>
          <h2>{{selected.subject||t('(No subject)','（无主题）')}}</h2><div class="sender">{{selected.sender}}</div><div class="muted">{{t('To: ','收件人：')}}{{selected.recipients}}</div><div class="muted">{{owner(selected.account_id)?.email}} · {{date(selected.timestamp)}}</div><div v-if="messageLabels.length" class="message-labels"><button v-for="label in messageLabels" :key="label" @click="viewLabel(label)">{{label}}</button></div>
          <div v-if="selected.attachments.length" class="attachment-panel">
            <div class="attachment-title">{{t('Attachments','附件')}} ({{selected.attachments.length}})</div>
            <div class="attachment-row" v-for="a in selected.attachments" :key="a.index">
              <span class="attachment-icon">▤</span><div class="attachment-name"><strong>{{a.name}}</strong><small>{{fileSize(a.size)}} · {{a.media_type}}</small></div>
              <button v-if="a.previewable" @click="viewAttachment(a.index)" :disabled="busy">{{t('Preview','预览')}}</button><button @click="downloadAttachment(a.index)" :disabled="busy">{{t('Save…','另存为…')}}</button>
            </div>
          </div>
          <div v-if="preview" class="attachment-preview"><div class="preview-heading"><strong>{{preview.name}}</strong><button @click="preview=null">×</button></div><pre v-if="preview.media_type==='text/plain'" class="body">{{preview.data}}</pre><img v-else-if="['image/png','image/jpeg','image/gif','image/webp'].includes(preview.media_type)" :src="'data:'+preview.media_type+';base64,'+preview.data" :alt="preview.name"></div>
          <hr>
          <div v-if="selected.html!==null" class="read-mode"><button :class="{active:readMode==='html'}" @click="readMode='html'">HTML</button><button :class="{active:readMode==='plain'}" @click="readMode='plain'">{{t('Plain text','纯文本')}}</button><small>{{t('External images and active content blocked','已拦截外部图片和活动内容')}}</small></div>
          <div v-if="readMode==='html'&&selected.html!==null" class="safe-mail-html" v-html="selected.html"></div>
          <pre v-else class="body">{{selected.body}}</pre>
        </article>
      </template>
      <div v-else class="empty reading-empty"><span>✉</span><h2>{{mode==='outbox'?t('Local delivery queue','本地发送队列'):mode==='drafts'?t('Choose a draft to edit','选择草稿进行编辑'):t('Select a message','选择一封邮件')}}</h2><p>{{t('Your mail stays on your device.','邮件保存在你的设备中。')}}</p></div>
    </section>
  </main>
  <footer v-if="!detachedCompose" role="status"><span>{{busy?t('Working…','处理中…'):t('Ready','就绪')}}</span><span>{{status}}</span><span class="error" v-if="error">{{error}}</span></footer>
  <div v-if="detachedCompose&&startupError" class="empty reading-empty"><p class="error">{{startupError}}</p><button @click="getCurrentWindow().close()">{{t('Close','关闭')}}</button></div>
  <div class="overlay" v-if="dialog" @click.self="closeDialog">
    <form v-if="dialog==='account'" class="modal" @submit.prevent="saveAccount">
      <div class="modal-head"><h2>{{editId?t('Edit mailbox','编辑邮箱'):t('Add mailbox','添加邮箱')}}</h2><button type="button" @click="closeDialog" :disabled="busy">×</button></div>
      <button v-if="!editId" type="button" class="oauth-entry" @click="startOAuth()">{{t('Sign in with Google or Microsoft (OAuth 2.0)','使用 Google 或 Microsoft 登录（OAuth 2.0）')}}</button>
      <label>{{t('Incoming protocol','收件协议')}}<select v-model="incomingProtocol" :disabled="!!editId" @change="changeIncoming"><option value="imap">IMAP TLS</option><option value="pop3">POP3 TLS</option></select></label>
      <p v-if="incomingProtocol==='pop3'" class="hint">{{t('POP3 downloads inbox mail and keeps server copies. Read/star flags and optional Sent copy are local.','POP3 下载收件箱并保留服务器副本，已读、星标及可选已发送副本仅保存在本地。')}}</p>
      <label v-if="!editId&&incomingProtocol==='imap'">{{t('Provider','服务商')}}<select v-model="preset" @change="applyPreset"><option value="custom">{{t('Custom IMAP / SMTP','自定义 IMAP / SMTP')}}</option><option value="qq">QQ Mail</option><option value="net163">163 Mail</option><option value="net126">126 Mail</option><option value="icloud">iCloud Mail</option><option value="gmail">Gmail (app password)</option></select></label>
      <p class="hint">{{editId?t('Leave password blank to keep it. To change the incoming mailbox identity, add a new account.','密码留空则保留原凭据。若需更改收件账号身份，请添加新账号。'):t('Use the provider’s app password or authorization code where required. Google and Microsoft OAuth use the separate sign-in flow.','按服务商要求使用应用专用密码或授权码。Google 和 Microsoft 使用单独的 OAuth 登录入口。')}}</p>
      <label>{{t('Display name','显示名称')}}<input v-model="form.name"></label>
      <label>{{t('Email address','邮箱地址')}}<input v-model="form.email" type="email" required :disabled="!!editId"></label>
      <label>{{t('Login username (optional)','登录名（可选）')}}<input v-model="form.username" autocomplete="off" :disabled="!!editId"></label>
      <label v-if="!editingOAuth">{{t('Password / authorization code','密码 / 授权码')}}<input v-model="form.password" type="password" autocomplete="new-password" :required="!editId"></label>
      <div class="field-grid"><label>{{incomingProtocol==='pop3'?'POP3 TLS':'IMAP TLS'}}<input v-model="form.imap_host" :placeholder="incomingProtocol==='pop3'?'pop.example.com':'imap.example.com'" required :disabled="!!editId"></label><label>{{t('Port','端口')}}<input type="number" v-model.number="form.imap_port" min="1" max="65535" required :disabled="!!editId"></label><label>SMTP<input v-model="form.smtp_host" placeholder="smtp.example.com" required :disabled="editingOAuth"></label><label>{{t('Port','端口')}}<input type="number" v-model.number="form.smtp_port" min="1" max="65535" required :disabled="editingOAuth"></label></div>
      <label class="inline-label"><input type="checkbox" v-model="form.smtp_starttls" :disabled="editingOAuth"> {{t('SMTP required STARTTLS (otherwise implicit TLS)','SMTP 强制 STARTTLS（否则使用隐式 TLS）')}}</label>
      <label class="inline-label">{{t('Account color','账号颜色')}} <input type="color" v-model="form.color"></label>
      <label>{{t('Default email signature','默认邮件签名')}}<textarea v-model="form.signature" rows="3" :placeholder="t('Optional plain-text signature','可选纯文本签名')"></textarea></label>
      <div v-if="error" class="error">{{error}}</div>
      <div class="modal-actions"><button type="button" @click="closeDialog" :disabled="busy">{{t('Cancel','取消')}}</button><button class="primary" :disabled="busy">{{t('Save account','保存账号')}}</button></div>
    </form>
    <form v-else-if="dialog==='oauth'" class="modal" @submit.prevent="authorizeOAuth">
      <div class="modal-head"><h2>{{oauthForm.account_id?t('Reauthorize mailbox','重新授权邮箱'):t('OAuth mailbox sign-in','OAuth 邮箱登录')}}</h2><button type="button" @click="cancelOAuth" :disabled="oauthCanceling">×</button></div>
      <p class="hint">{{t('Use your own registered desktop OAuth client. Mail EX opens the official authorization page in your system browser. No mailbox password is collected.','使用你自己的桌面应用 OAuth 注册信息。Mail EX 在系统浏览器打开官方授权页面，不收集邮箱登录密码。')}}</p>
      <fieldset class="compose-fields" :disabled="oauthWaiting">
        <label>{{t('Provider','服务商')}}<select v-model="oauthForm.provider" :disabled="!!oauthForm.account_id"><option value="google">Gmail / Google Workspace</option><option value="outlook">Outlook.com / Hotmail</option><option value="microsoft">Microsoft 365 / Exchange Online</option></select></label>
        <label>{{t('Email address','邮箱地址')}}<input v-model="oauthForm.email" type="email" required :disabled="!!oauthForm.account_id"></label>
        <label v-if="!oauthForm.account_id">{{t('Display name','显示名称')}}<input v-model="oauthForm.name"></label>
        <label>OAuth Client ID<input v-model="oauthForm.client_id" required autocomplete="off" spellcheck="false"></label>
        <label v-if="oauthForm.provider==='google'">{{t('Desktop client secret (if issued)','桌面客户端 Secret（如注册时提供）')}}<input v-model="oauthForm.desktop_client_secret" type="password" autocomplete="off"></label>
        <p class="hint">{{oauthForm.provider==='google'?t('Register a Desktop app OAuth client and allow the Gmail mail scope. Google may require consent-screen verification.','注册桌面应用 OAuth Client，授权 Gmail 邮件范围；Google 可能要求同意屏幕验证。'):t('Register a Mobile and desktop application / public client with http://localhost redirect and delegated Exchange Online IMAP and SMTP permissions.','注册移动和桌面应用（公共客户端），设置 http://localhost 回调以及 Exchange Online IMAP、SMTP 委托权限。')}}</p>
        <label v-if="!oauthForm.account_id" class="inline-label">{{t('Account color','账号颜色')}} <input v-model="oauthForm.color" type="color"></label>
      </fieldset>
      <div v-if="oauthWaiting" class="notice">{{oauthCanceling?t('Canceling authorization…','正在取消授权…'):t('Waiting for authorization in the system browser…','等待系统浏览器授权…')}}</div>
      <div v-if="error" class="error">{{error}}</div>
      <div class="modal-actions"><button type="button" @click="cancelOAuth" :disabled="oauthCanceling">{{t('Cancel','取消')}}</button><button class="primary" :disabled="busy">{{t('Authorize in browser','在浏览器授权')}}</button></div>
    </form>
    <form v-else class="modal compose-modal" @submit.prevent="send">
      <div class="modal-head"><h2>{{draftId?t('Edit draft','编辑草稿'):t('New message','新邮件')}}</h2><div class="compose-window-actions"><button v-if="!detachedCompose" type="button" :disabled="busy" @click="detachCurrent">{{t('Open in separate window','在独立窗口打开')}}</button><button type="button" @click="closeDialog" :disabled="busy">×</button></div></div>

      <fieldset class="compose-fields" :disabled="busy">
        <label>{{t('From','发件人')}}<select v-model="draft.account_id" required><option v-for="a in sendingAccounts" :key="a.id" :value="a.id">{{a.email}}</option></select></label>
        <label>{{t('To (comma-separated addresses)','收件人（逗号分隔地址）')}}<RecipientField v-model="draft.to" :contacts="contacts" :zh="zh" required/></label>
        <div class="compose-addresses"><label>Cc<RecipientField v-model="draft.cc" :contacts="contacts" :zh="zh"/></label><label>Bcc<RecipientField v-model="draft.bcc" :contacts="contacts" :zh="zh"/></label></div>
        <label>{{t('Subject','主题')}}<input v-model="draft.subject"></label>
        <div class="template-controls"><select v-model="templateChoice" :aria-label="t('Mail template','邮件模板')"><option value="">{{t('Choose a template','选择邮件模板')}}</option><option v-for="m in templates" :key="m.id" :value="m.id">{{m.name}}</option></select><button type="button" :disabled="!templateChoice" @click="applyTemplate">{{t('Apply','应用')}}</button><input v-model="templateName" :placeholder="t('New template name','新模板名称')"><button type="button" :disabled="!templateName.trim()" @click="saveCurrentTemplate">{{t('Save as template','保存为模板')}}</button></div>
        <div class="compose-options"><button type="button" :class="{active:!rich}" @click="changeFormat('plain')">{{t('Plain text','纯文本')}}</button><button type="button" :class="{active:rich}" @click="changeFormat('rich')">{{t('Rich text','富文本')}}</button><button v-if="owner(draft.account_id)?.signature" type="button" @click="insertSignature">{{t('Insert signature','插入签名')}}</button><button type="button" @click="attachmentInput?.click()">＋ {{t('Attach files','添加附件')}}</button><input ref="attachmentInput" class="file-input" type="file" multiple @change="addFiles"></div>
        <RichEditor v-if="rich" :html="draft.html||''" :zh="zh" :disabled="busy" @update="richUpdate"/>
        <textarea v-else class="compose-body" v-model="draft.body" :placeholder="t('Write your message…','撰写邮件正文…')" rows="12"></textarea>
        <div v-if="composeFiles.length" class="attachment-panel"><div class="attachment-title">{{t('Attachments','附件')}} ({{composeFiles.length}})</div><div v-for="(a,i) in composeFiles" :key="i" class="attachment-row"><span class="attachment-icon">▤</span><div class="attachment-name"><strong>{{a.name}}</strong><small>{{fileSize(attachedSize(a))}}</small></div><button type="button" @click="removeAttachment(i)">×</button></div></div>
        <label class="inline-label"><input type="checkbox" v-model="draft.save_sent_copy"> {{owner(draft.account_id)?.auth_kind==='pop3'?t('Save a local Sent copy','保存本地已发送副本'):t('Save an additional copy to the server Sent folder (turn off when the provider does this automatically)','额外保存副本到服务器已发送文件夹（服务商自动保存时请关闭）')}}</label>
        <label class="inline-label"><input type="checkbox" v-model="scheduleEnabled"> {{t('Schedule delivery','定时发送')}}</label><label v-if="scheduleEnabled">{{t('Local send time','本地发送时间')}}<input type="datetime-local" v-model="scheduleAt" required><small>{{t('Mail EX must be running; overdue mail sends on the next launch.','Mail EX 需保持运行；退出期间到期的邮件将在下次启动时发送。')}}</small></label>
        <label class="inline-label"><input type="checkbox" v-model="autosave"> {{t('Automatically save local drafts while editing','编辑时自动保存本地草稿')}} <small>{{dirty?t('Unsaved changes','未保存修改'):draftId?t('Saved','已保存'):''}}</small></label>
      </fieldset>
      <div class="error" v-if="error">{{error}}</div><div class="modal-actions"><button type="button" @click="closeDialog" :disabled="busy">{{t('Close','关闭')}}</button><button type="button" @click="saveDraft" :disabled="busy">{{t('Save draft','保存草稿')}}</button><button class="primary" :disabled="busy">{{scheduleEnabled?t('Schedule','加入定时队列'):t('Send','发送')}}</button></div>
    </form>
  </div>
  <div v-if="settingsPanel&&!detachedCompose" class="overlay" @click.self="settingsPanel=false">
    <section class="modal preferences-modal">
      <div class="modal-head"><h2>{{t('Settings','全局设置')}}</h2><button @click="settingsPanel=false">×</button></div>
      <p class="hint">{{t('Preferences are saved on this device.','设置保存在本机，自动生效。')}}</p>
      <div class="data-section"><h3>{{t('Appearance','界面外观')}}</h3>
        <div class="organizer-fields">
          <label>{{t('Language','语言')}}<select v-model="language"><option value="system">{{t('Follow system','跟随系统')}}</option><option value="en">English</option><option value="zh">简体中文</option></select></label>
          <label>{{t('Theme','主题')}}<select v-model="theme"><option value="system">{{t('Follow system','跟随系统')}}</option><option value="light">{{t('Light','浅色')}}</option><option value="soft">{{t('Soft light','柔和浅色')}}</option><option value="gray">{{t('Gray','灰色')}}</option><option value="dark">{{t('Dark','深色')}}</option></select></label>
          <label>{{t('Message list density','邮件列表密度')}}<select v-model="density"><option value="comfortable">{{t('Comfortable','舒适')}}</option><option value="compact">{{t('Compact','紧凑')}}</option></select></label>
          <label>{{t('Reading font size','阅读字号')}}<input v-model.number="readerFontSize" type="number" min="10" max="32" step="1" @blur="readerFontSize=Math.min(32,Math.max(10,Number(readerFontSize)||14))"></label>
        </div>
        <button @click="resetLayout">{{t('Reset layout','重置布局')}}</button>
      </div>
      <div class="data-section"><h3>{{t('Mail behavior','邮件行为')}}</h3>
        <label>{{t('Automatic inbox sync interval (minutes; 0 = manual)','自动同步间隔（分钟；0 为手动）')}}<input v-model.number="autoMinutes" type="number" min="0" step="1" @blur="autoMinutes=Math.max(0,Math.floor(Number(autoMinutes)||0))"></label>
        <label class="inline-label"><input type="checkbox" v-model="separateCompose">{{t('Open new compose in an independent window','默认使用独立邮件撰写窗口')}}</label>
        <label class="inline-label"><input type="checkbox" v-model="autosave">{{t('Automatically save drafts','自动保存草稿')}}</label>
        <div class="organizer-action-row"><button @click="settingsPanel=false;dataPanel=true">{{t('Storage and backups','存储及备份')}}</button><button @click="settingsPanel=false;calendarPanel=true">{{t('Calendar and notifications','日历及通知')}}</button></div>
      </div>
      <div class="data-section"><h3>{{t('Keyboard shortcuts','快捷键')}}</h3><div class="shortcut-list"><span>{{t('New message','写邮件')}}</span><kbd>Ctrl / ⌘ + N</kbd><span>{{t('Search','搜索')}}</span><kbd>Ctrl / ⌘ + F</kbd><span>{{t('Synchronize','同步')}}</span><kbd>F5</kbd><span>{{t('Settings','设置')}}</span><kbd>Ctrl / ⌘ + ,</kbd><span>{{t('Save draft','保存草稿')}}</span><kbd>Ctrl / ⌘ + S</kbd></div></div>
      <div class="modal-actions"><button class="primary" @click="settingsPanel=false">{{t('Done','完成')}}</button></div>
    </section>
  </div>
  <CalendarPanel v-if="calendarPanel" :zh="zh" :accounts="accounts" :contacts="contacts" @close="calendarPanel=false" @compose="calendarPanel=false;handleCompose($event)"/>
  <DataPanel v-if="dataPanel" :zh="zh" :accounts="accounts" @close="dataPanel=false" @changed="dataChanged"/>
  <OrganizerPanel v-if="organizerTab" :initial-tab="organizerTab" :zh="zh" :accounts="accounts" :search-seed="searchSeed" @close="organizerTab=null" @changed="organizerChanged" @use-search="useSavedSearch"/>
</div>
</template>
