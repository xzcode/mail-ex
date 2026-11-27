<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { api, type Account, type NewAccount, type Summary, type Mail, type Compose, type Folder, type Draft } from './api'

type View = 'inbox'|'unread'|'starred'|'all'|'folder'|'drafts'
const zh=ref(localStorage.getItem('language')?localStorage.getItem('language')==='zh':navigator.language.toLowerCase().startsWith('zh'))
const t=(en:string,cn:string)=>zh.value?cn:en
const theme=ref(localStorage.getItem('theme')||'system')
const autoMinutes=ref(Number(localStorage.getItem('autoMinutes')||0))
const navWidth=ref(Number(localStorage.getItem('navWidth'))||234), listWidth=ref(Number(localStorage.getItem('listWidth'))||355)
watch(zh,v=>localStorage.setItem('language',v?'zh':'en'))
watch(theme,v=>localStorage.setItem('theme',v))
watch(autoMinutes,v=>{localStorage.setItem('autoMinutes',String(v));lastAuto=Date.now()})
watch(navWidth,v=>localStorage.setItem('navWidth',String(v)))
watch(listWidth,v=>localStorage.setItem('listWidth',String(v)))
const accounts=ref<Account[]>([]), folders=ref<Folder[]>([]), messages=ref<Summary[]>([]), drafts=ref<Draft[]>([])
const selected=ref<Mail|null>(null), expanded=ref<string[]>([])
const accountId=ref<string|null>(null), mode=ref<View>('inbox'), folderName=ref('INBOX')
const query=ref(''), offset=ref(0), more=ref(false), destination=ref('')
const status=ref(''), error=ref(''), busy=ref(false), loading=ref(false)
const dialog=ref<'account'|'compose'|null>(null), editId=ref<string|null>(null), draftId=ref<string|null>(null)
const preset=ref('custom')
const blankAccount=():NewAccount=>({name:'',email:'',username:'',password:'',imap_host:'',imap_port:993,smtp_host:'',smtp_port:465,smtp_starttls:false,color:'#507ed8'})
const form=ref<NewAccount>(blankAccount())
const blankCompose=():Compose=>({account_id:accountId.value||accounts.value[0]?.id||'',to:'',cc:'',bcc:'',subject:'',body:''})
const draft=ref<Compose>(blankCompose())
const draftBaseline=ref('')
const currentAccount=computed(()=>accounts.value.find(a=>a.id===accountId.value))
const currentFolder=computed(()=>folders.value.find(f=>f.account_id===accountId.value&&f.name===folderName.value))
const selectedFolders=computed(()=>folders.value.filter(f=>f.account_id===selected.value?.account_id&&f.name!==selected.value?.folder&&f.available))
const archiveFolder=computed(()=>selectedFolders.value.find(f=>f.kind==='archive'))
const trashFolder=computed(()=>selectedFolders.value.find(f=>f.kind==='trash'))
const folderLabel=(f:Folder)=>f.kind==='inbox'?t('Inbox','收件箱'):f.kind==='sent'?t('Sent','已发送'):f.kind==='drafts'?t('Server drafts','服务器草稿'):f.kind==='trash'?t('Trash','废纸篓'):f.kind==='junk'?t('Junk','垃圾邮件'):f.kind==='archive'?t('Archive','归档'):f.label
const title=computed(()=>mode.value==='folder'?(currentFolder.value?folderLabel(currentFolder.value):folderName.value):({inbox:t('All inboxes','全部收件箱'),unread:t('Unread inboxes','未读邮件'),starred:t('Starred','星标邮件'),all:t('All cached mail','全部本地邮件'),drafts:t('Local drafts','本地草稿')}[mode.value]))
const currentDrafts=computed(()=>drafts.value.filter(d=>!query.value||[d.mail.subject,d.mail.to,d.mail.body].some(s=>s.toLowerCase().includes(query.value.toLowerCase()))))
const totalUnread=computed(()=>folders.value.filter(f=>f.kind==='inbox').reduce((n,f)=>n+f.unread,0))
const errorText=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:String(e)
const owner=(id:string)=>accounts.value.find(a=>a.id===id)
const folderOf=(m:Summary)=>folders.value.find(f=>f.account_id===m.account_id&&f.name===m.folder)
const date=(n:number)=>n?new Date(n*1000).toLocaleString(zh.value?'zh-CN':undefined,{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}):''
let generation=0,detailGeneration=0,lastAuto=Date.now(),timer:number|undefined
async function refreshAccounts(){
  const old=new Set(accounts.value.map(a=>a.id))
  accounts.value=await api.accounts()
  expanded.value=expanded.value.filter(id=>accounts.value.some(a=>a.id===id))
  for(const a of accounts.value)if(!old.has(a.id))expanded.value.push(a.id)
  folders.value=await api.folders()
}
async function refreshDrafts(){drafts.value=await api.drafts()}
async function load(reset=true,preserve=false){
  const token=++generation
  const previous=preserve?selected.value:null
  if(reset){offset.value=0;selected.value=null;detailGeneration++}
  if(mode.value==='drafts'){messages.value=[];more.value=false;loading.value=false;try{await refreshDrafts()}catch(e){error.value=errorText(e)}return}
  loading.value=true
  try{
    const folder=mode.value==='folder'?folderName.value:mode.value==='inbox'||mode.value==='unread'?'INBOX':null
    const result=await api.messages(accountId.value,folder,mode.value==='unread',mode.value==='starred',query.value,reset?0:offset.value)
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
  accountId.value=id;mode.value=kind;folderName.value=folder;query.value='';void load()
}
function toggleExpand(id:string){
  expanded.value=expanded.value.includes(id)?expanded.value.filter(x=>x!==id):[...expanded.value,id]
}
function newAccount(){form.value=blankAccount();preset.value='custom';editId.value=null;error.value='';dialog.value='account'}
function editAccount(){
  const a=currentAccount.value;if(!a)return
  const {id,...data}=a;editId.value=id;form.value={...data,password:''};preset.value='custom';error.value='';dialog.value='account'
}
function applyPreset(){
  const p:Record<string,[string,string,number,boolean]>={qq:['imap.qq.com','smtp.qq.com',465,false],net163:['imap.163.com','smtp.163.com',465,false],net126:['imap.126.com','smtp.126.com',465,false],icloud:['imap.mail.me.com','smtp.mail.me.com',587,true],gmail:['imap.gmail.com','smtp.gmail.com',465,false]}
  const v=p[preset.value]
  if(v){[form.value.imap_host,form.value.smtp_host,form.value.smtp_port,form.value.smtp_starttls]=v;form.value.imap_port=993}
}
function closeDialog(){
  if(busy.value)return
  if(dialog.value==='compose'&&JSON.stringify(draft.value)!==draftBaseline.value&&!confirm(t('Discard unsaved changes? Use Save draft to keep them.','放弃尚未保存的修改？使用保存草稿可保留。')))return
  form.value.password='';dialog.value=null
}
async function saveAccount(){await run(async()=>{
  const input={...form.value,username:form.value.username.trim()||form.value.email.trim(),name:form.value.name.trim()||form.value.email.trim()}
  const a=editId.value?await api.update(editId.value,input):await api.add(input)
  form.value=blankAccount();dialog.value=null;await refreshAccounts();view(a.id,'folder');status.value=t('Account saved. Test or sync to verify access.','账号已保存，请测试连接或同步。')
})}
async function testAccount(){const id=accountId.value;if(id)await run(async()=>{status.value=await api.test(id)})}
async function sync(all=false,automatic=false){await run(async()=>{
  const active=currentAccount.value,targets=automatic?accounts.value:active?[active]:accounts.value
  if(!targets.length){if(!automatic)newAccount();return}
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
async function remove(){const a=currentAccount.value;if(!a||!confirm(t('Remove this account and its local cache? Server mail is not deleted.','移除此账号及本地缓存？不会删除服务器邮件。')))return;await run(async()=>{await api.remove(a.id);await refreshAccounts();await refreshDrafts();view(null)})}
async function open(m:Summary){const token=++detailGeneration;try{const result=await api.message(m.id);if(token===detailGeneration){selected.value=result;destination.value=''}}catch(e){if(token===detailGeneration)error.value=errorText(e)}}
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
function startCompose(mail:Compose,id:string|null=null){
  draft.value={...mail};draftId.value=id;draftBaseline.value=JSON.stringify(draft.value);dialog.value='compose';error.value=''
}
function compose(){if(!accounts.value.length){newAccount();return}startCompose(blankCompose())}
async function reply(kind:'reply'|'reply_all'|'forward'){const id=selected.value?.id;if(id!==undefined)await run(async()=>{startCompose(await api.composeFrom(id,kind))})}
async function saveDraft(){await run(async()=>{
  draftId.value=await api.saveDraft(draftId.value,{...draft.value});draftBaseline.value=JSON.stringify(draft.value)
  dialog.value=null;await refreshDrafts();status.value=t('Draft saved locally.','草稿已保存到本地。')
})}
async function deleteDraft(d:Draft){if(!confirm(t('Delete this local draft?','删除这份本地草稿？')))return;await run(async()=>{await api.deleteDraft(d.id);await refreshDrafts()})}
async function send(){await run(async()=>{
  const id=draftId.value;await api.send({...draft.value})
  dialog.value=null;draftId.value=null;draft.value=blankCompose()
  status.value=t('Accepted by SMTP server.','SMTP 服务器已接受邮件。')
  if(id){try{await api.deleteDraft(id);await refreshDrafts()}catch(e){error.value=t('Message accepted; local draft cleanup failed: ','邮件已被接受，但本地草稿清理失败：')+errorText(e)}}
})}
function resize(event:PointerEvent,which:'nav'|'list'){
  event.preventDefault()
  const start=event.clientX,initial=which==='nav'?navWidth.value:listWidth.value
  const move=(e:PointerEvent)=>{const value=Math.max(which==='nav'?180:260,Math.min(which==='nav'?440:650,initial+e.clientX-start));if(which==='nav')navWidth.value=value;else listWidth.value=value}
  const done=()=>{document.removeEventListener('pointermove',move);document.removeEventListener('pointerup',done);document.removeEventListener('pointercancel',done)}
  document.addEventListener('pointermove',move);document.addEventListener('pointerup',done);document.addEventListener('pointercancel',done)
}
onMounted(async()=>{
  try{await refreshAccounts();await refreshDrafts();await load()}catch(e){error.value=errorText(e)}
  timer=window.setInterval(()=>{if(autoMinutes.value>0&&!busy.value&&!loading.value&&!dialog.value&&Date.now()-lastAuto>=autoMinutes.value*60000)void sync(false,true)},15000)
})
onUnmounted(()=>{if(timer!==undefined)clearInterval(timer)})
</script>

<template>
<div class="shell" :data-theme="theme">
  <header class="top">
    <div class="brand"><span class="logo">✉</span> Mail EX</div>
    <form class="search" @submit.prevent="load()"><input v-model="query" :placeholder="t('Search current view','搜索当前视图')" :aria-label="t('Search','搜索')"><button>{{t('Search','搜索')}}</button></form>
    <div class="top-actions">
      <select v-model.number="autoMinutes" :aria-label="t('Automatic inbox sync','自动同步收件箱')" :title="t('Automatic inbox sync while open','应用运行时自动同步收件箱')"><option :value="0">{{t('Manual sync','手动同步')}}</option><option :value="1">{{t('Every 1 min','每 1 分钟')}}</option><option :value="5">{{t('Every 5 min','每 5 分钟')}}</option><option :value="15">{{t('Every 15 min','每 15 分钟')}}</option></select>
      <select v-model="theme" :aria-label="t('Theme','主题')"><option value="system">{{t('System','跟随系统')}}</option><option value="light">{{t('Light','浅色')}}</option><option value="dark">{{t('Dark','深色')}}</option></select><button @click="zh=!zh">{{zh?'EN':'中文'}}</button>
    </div>
  </header>
  <main class="workspace" :style="{gridTemplateColumns:navWidth+'px 5px '+listWidth+'px 5px minmax(0,1fr)'}">
    <aside class="navigation">
      <button class="primary compose" @click="compose">＋ {{t('New message','写邮件')}}</button>
      <div class="section">{{t('UNIFIED MAIL','统一邮箱')}}</div>
      <button class="nav-row" :class="{active:mode==='inbox'}" @click="view(null,'inbox')">▤ <span class="nav-label">{{t('All inboxes','全部收件箱')}}</span><small v-if="totalUnread" class="count">{{totalUnread}}</small></button>
      <button class="nav-row" :class="{active:mode==='unread'}" @click="view(null,'unread')">● <span class="nav-label">{{t('Unread','未读邮件')}}</span></button>
      <button class="nav-row" :class="{active:mode==='starred'}" @click="view(null,'starred')">★ <span class="nav-label">{{t('Starred','星标邮件')}}</span></button>
      <button class="nav-row" :class="{active:mode==='all'}" @click="view(null,'all')">▧ <span class="nav-label">{{t('All cached mail','全部本地邮件')}}</span></button>
      <button class="nav-row" :class="{active:mode==='drafts'}" @click="view(null,'drafts')">✎ <span class="nav-label">{{t('Local drafts','本地草稿')}}</span><small v-if="drafts.length" class="count">{{drafts.length}}</small></button>
      <div class="section accounts-header">{{t('ACCOUNTS','邮箱账号')}}<button @click="newAccount" :title="t('Add account','添加账号')">＋</button></div>
      <div v-for="a in accounts" :key="a.id" class="account-group">
        <div class="account-heading"><button class="expand" @click="toggleExpand(a.id)" :aria-expanded="expanded.includes(a.id)" :aria-label="t('Toggle folders','展开或收起文件夹')">{{expanded.includes(a.id)?'⌄':'›'}}</button>
          <button class="nav-row account" :class="{active:accountId===a.id&&mode==='folder'&&folderName==='INBOX'}" @click="view(a.id,'folder')"><span class="dot" :style="{background:a.color}"></span><span class="account-text">{{a.name}}<small>{{a.email}}</small></span></button>
        </div>
        <template v-if="expanded.includes(a.id)">
          <button v-for="f in folders.filter(f=>f.account_id===a.id)" :key="f.name" class="nav-row folder-row" :class="{active:accountId===a.id&&mode==='folder'&&folderName===f.name,unavailable:!f.available}" @click="view(a.id,'folder',f.name)" :title="f.available?f.label:t('Unavailable on server; cached mail retained','服务器文件夹不可用，已保留缓存')"><span class="folder-symbol">{{f.kind==='inbox'?'▤':f.kind==='sent'?'↗':f.kind==='drafts'?'✎':f.kind==='trash'?'♧':f.kind==='archive'?'▣':'▱'}}</span><span class="nav-label">{{folderLabel(f)}}</span><small v-if="f.unread" class="count">{{f.unread}}</small></button>
        </template>
      </div>
      <div v-if="!accounts.length" class="hint">{{t('Add your first mailbox to get started.','添加第一个邮箱以开始使用。')}}</div>
      <button class="add" @click="newAccount">＋ {{t('Add account','添加账号')}}</button>
      <div class="nav-footer">{{t('Local-first · No Mail EX cloud','本地优先 · 无强制云服务')}}</div>
    </aside>
    <div class="split" role="separator" aria-orientation="vertical" @pointerdown="resize($event,'nav')"></div>
    <section class="message-list">
      <div class="list-head"><h1>{{title}}</h1><div class="toolbar">
        <template v-if="mode!=='drafts'"><button @click="sync()" :disabled="busy||currentFolder?.available===false">↻ {{t('Sync','同步')}}</button><button v-if="currentAccount" @click="sync(true)" :disabled="busy">{{t('Sync all folders','同步全部文件夹')}}</button><button v-if="currentAccount" @click="testAccount" :disabled="busy">{{t('Test','测试')}}</button><button v-if="currentAccount" @click="editAccount" :disabled="busy">{{t('Edit account','编辑账号')}}</button><button v-if="currentAccount" class="danger-text" @click="remove" :disabled="busy">{{t('Remove','移除')}}</button></template>
        <button v-else @click="compose">{{t('New draft','新建草稿')}}</button>
      </div></div>
      <div class="list-meta">{{mode==='drafts'?currentDrafts.length:messages.length}} {{t('loaded','封已加载')}} <span v-if="loading">{{t('Loading…','加载中…')}}</span></div>
      <div class="items" v-if="mode==='drafts'">
        <div v-for="d in currentDrafts" :key="d.id" class="draft-row"><button class="mail-row" @click="startCompose(d.mail,d.id)"><div class="mail-line"><strong>{{d.mail.subject||t('(No subject)','（无主题）')}}</strong><time>{{date(d.updated_at)}}</time></div><div class="subject">{{d.mail.to||t('No recipient','未填写收件人')}}</div><small>{{owner(d.mail.account_id)?.name}}</small></button><button class="delete-draft" @click="deleteDraft(d)" :disabled="busy" :title="t('Delete local draft','删除本地草稿')">×</button></div>
        <div class="empty" v-if="!currentDrafts.length"><span>✎</span><p>{{t('No local drafts','没有本地草稿')}}</p></div>
      </div>
      <div v-else class="items">
        <button v-for="m in messages" :key="m.id" class="mail-row" :class="{selected:selected?.id===m.id,unseen:!m.seen}" @click="open(m)">
          <div class="mail-line"><span class="dot" :style="{background:owner(m.account_id)?.color||'#aaa'}"></span><strong>{{m.sender||t('(Unknown sender)','（未知发件人）')}}</strong><span v-if="m.starred" class="star">★</span><time>{{date(m.timestamp)}}</time></div>
          <div class="subject">{{m.subject||t('(No subject)','（无主题）')}}</div><small>{{owner(m.account_id)?.name}}<template v-if="m.folder!=='INBOX'"> · {{folderOf(m)?.label||m.folder}}</template></small>
        </button>
        <div class="empty" v-if="!messages.length&&!loading"><span>✉</span><p>{{t('No cached messages','没有本地邮件')}}</p><button v-if="accounts.length&&currentFolder?.available!==false" @click="sync()" :disabled="busy">{{t('Sync mail','同步邮件')}}</button></div>
        <button v-if="more" class="load-more" @click="load(false)" :disabled="loading">{{t('Load more','加载更多')}}</button>
      </div>
    </section>
    <div class="split" role="separator" aria-orientation="vertical" @pointerdown="resize($event,'list')"></div>
    <section class="reading">
      <template v-if="selected">
        <div class="reader-toolbar">
          <button @click="reply('reply')" :disabled="busy">{{t('Reply','回复')}}</button><button @click="reply('reply_all')" :disabled="busy">{{t('Reply all','回复全部')}}</button><button @click="reply('forward')" :disabled="busy">{{t('Forward','转发')}}</button>
          <button @click="toggleStar" :disabled="busy">{{selected.starred?t('Unstar','取消星标'):t('Star','星标')}}</button><button @click="toggleSeen" :disabled="busy">{{selected.seen?t('Mark unread','标为未读'):t('Mark read','标为已读')}}</button>
          <button v-if="archiveFolder" @click="moveTo(archiveFolder.name)" :disabled="busy">{{t('Archive','归档')}}</button><button v-if="trashFolder" class="danger-text" @click="moveTo(trashFolder.name)" :disabled="busy">{{t('Move to trash','移至废纸篓')}}</button>
          <div v-if="selectedFolders.length" class="move-control"><select v-model="destination" :disabled="busy" :aria-label="t('Destination folder','目标文件夹')"><option value="">{{t('Choose folder','选择文件夹')}}</option><option v-for="f in selectedFolders" :key="f.name" :value="f.name">{{folderLabel(f)}}</option></select><button @click="moveTo(destination)" :disabled="busy||!destination">{{t('Move','移动')}}</button></div>
        </div>
        <article><h2>{{selected.subject||t('(No subject)','（无主题）')}}</h2><div class="sender">{{selected.sender}}</div><div class="muted">{{t('To: ','收件人：')}}{{selected.recipients}}</div><div class="muted">{{owner(selected.account_id)?.email}} · {{date(selected.timestamp)}}</div><hr><p class="notice" v-if="selected.html_only">{{t('HTML-only message: safe preview is not available yet. Original MIME is cached.','此邮件仅有 HTML 内容；安全预览尚未实现，原始邮件已缓存。')}}</p><pre class="body">{{selected.body}}</pre></article>
      </template>
      <div v-else class="empty reading-empty"><span>✉</span><h2>{{mode==='drafts'?t('Choose a draft to edit','选择草稿进行编辑'):t('Select a message','选择一封邮件')}}</h2><p>{{t('Your mail stays on your device.','邮件保存在你的设备中。')}}</p></div>
    </section>
  </main>
  <footer role="status"><span>{{busy?t('Working…','处理中…'):t('Ready','就绪')}}</span><span>{{status}}</span><span class="error" v-if="error">{{error}}</span></footer>
  <div class="overlay" v-if="dialog" @click.self="closeDialog">
    <form v-if="dialog==='account'" class="modal" @submit.prevent="saveAccount">
      <div class="modal-head"><h2>{{editId?t('Edit mailbox','编辑邮箱'):t('Add mailbox','添加邮箱')}}</h2><button type="button" @click="closeDialog" :disabled="busy">×</button></div>
      <label v-if="!editId">{{t('Provider','服务商')}}<select v-model="preset" @change="applyPreset"><option value="custom">{{t('Custom IMAP / SMTP','自定义 IMAP / SMTP')}}</option><option value="qq">QQ Mail</option><option value="net163">163 Mail</option><option value="net126">126 Mail</option><option value="icloud">iCloud Mail</option><option value="gmail">Gmail (app password)</option></select></label>
      <p class="hint">{{editId?t('Leave password blank to keep it. To change the incoming mailbox identity, add a new account.','密码留空则保留原凭据。若需更改收件账号身份，请添加新账号。'):t('Use the provider’s app password or authorization code where required. OAuth is not implemented yet.','按服务商要求使用应用专用密码或授权码。OAuth 尚未实现。')}}</p>
      <label>{{t('Display name','显示名称')}}<input v-model="form.name"></label>
      <label>{{t('Email address','邮箱地址')}}<input v-model="form.email" type="email" required :disabled="!!editId"></label>
      <label>{{t('Login username (optional)','登录名（可选）')}}<input v-model="form.username" autocomplete="off" :disabled="!!editId"></label>
      <label>{{t('Password / authorization code','密码 / 授权码')}}<input v-model="form.password" type="password" autocomplete="new-password" :required="!editId"></label>
      <div class="field-grid"><label>IMAP TLS<input v-model="form.imap_host" placeholder="imap.example.com" required :disabled="!!editId"></label><label>{{t('Port','端口')}}<input type="number" v-model.number="form.imap_port" min="1" max="65535" required :disabled="!!editId"></label><label>SMTP<input v-model="form.smtp_host" placeholder="smtp.example.com" required></label><label>{{t('Port','端口')}}<input type="number" v-model.number="form.smtp_port" min="1" max="65535" required></label></div>
      <label class="inline-label"><input type="checkbox" v-model="form.smtp_starttls"> {{t('SMTP required STARTTLS (otherwise implicit TLS)','SMTP 强制 STARTTLS（否则使用隐式 TLS）')}}</label>
      <label class="inline-label">{{t('Account color','账号颜色')}} <input type="color" v-model="form.color"></label>
      <div v-if="error" class="error">{{error}}</div>
      <div class="modal-actions"><button type="button" @click="closeDialog" :disabled="busy">{{t('Cancel','取消')}}</button><button class="primary" :disabled="busy">{{t('Save account','保存账号')}}</button></div>
    </form>
    <form v-else class="modal compose-modal" @submit.prevent="send">
      <div class="modal-head"><h2>{{draftId?t('Edit draft','编辑草稿'):t('New message','新邮件')}}</h2><button type="button" @click="closeDialog" :disabled="busy">×</button></div>
      <label>{{t('From','发件人')}}<select v-model="draft.account_id" required><option v-for="a in accounts" :key="a.id" :value="a.id">{{a.email}}</option></select></label>
      <label>{{t('To (comma-separated addresses)','收件人（逗号分隔地址）')}}<input v-model="draft.to" required></label>
      <label>Cc<input v-model="draft.cc"></label><label>Bcc<input v-model="draft.bcc"></label>
      <label>{{t('Subject','主题')}}<input v-model="draft.subject"></label><textarea v-model="draft.body" :placeholder="t('Write your message…','撰写邮件正文…')" rows="12"></textarea>
      <div class="error" v-if="error">{{error}}</div><div class="modal-actions"><button type="button" @click="closeDialog" :disabled="busy">{{t('Cancel','取消')}}</button><button type="button" @click="saveDraft" :disabled="busy">{{t('Save draft','保存草稿')}}</button><button class="primary" :disabled="busy">{{t('Send','发送')}}</button></div>
    </form>
  </div>
</div>
</template>
