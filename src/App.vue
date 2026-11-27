<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { api, type Account, type NewAccount, type Summary, type Mail, type Compose } from './api'
const zh = ref(navigator.language.toLowerCase().startsWith('zh'))
const t = (en:string,cn:string) => zh.value ? cn : en
const accounts=ref<Account[]>([]), messages=ref<Summary[]>([]), selected=ref<Mail|null>(null)
const accountId=ref<string|null>(null), unread=ref(false), query=ref(''), offset=ref(0), more=ref(false)
const status=ref(''), error=ref(''), busy=ref(false), loading=ref(false)
const dialog=ref<'account'|'compose'|null>(null)
const preset=ref('custom'), theme=ref('system'), navWidth=ref(234), listWidth=ref(355)
const blankAccount=():NewAccount=>({name:'',email:'',username:'',password:'',imap_host:'',imap_port:993,smtp_host:'',smtp_port:465,smtp_starttls:false,color:'#507ed8'})
const form=ref<NewAccount>(blankAccount())
const blankCompose=():Compose=>({account_id:accountId.value||accounts.value[0]?.id||'',to:'',cc:'',bcc:'',subject:'',body:''})
const draft=ref<Compose>(blankCompose())
const currentAccount=computed(()=>accounts.value.find(a=>a.id===accountId.value))
const title=computed(()=>currentAccount.value?.name || (unread.value?t('Unread','未读邮件'):t('All inboxes','全部收件箱')))
const errorText=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:String(e)
let generation=0, detailGeneration=0
async function refreshAccounts(){accounts.value=await api.accounts()}
async function load(reset=true){
  const token=++generation
  if(reset){offset.value=0;selected.value=null;detailGeneration++}
  loading.value=true
  try{
    const result=await api.messages(accountId.value,unread.value,query.value,reset?0:offset.value)
    if(token!==generation)return
    messages.value=reset?result:[...messages.value,...result]
    offset.value=messages.value.length
    more.value=result.length===100
  }catch(e){if(token===generation)error.value=errorText(e)}
  finally{if(token===generation)loading.value=false}
}
async function run(fn:()=>Promise<void>){
  if(busy.value)return
  error.value='';status.value='';busy.value=true
  try{await fn()}catch(e){error.value=errorText(e)}finally{busy.value=false}
}
function view(id:string|null,onlyUnread=false){accountId.value=id;unread.value=onlyUnread;query.value='';void load()}
function newAccount(){form.value=blankAccount();preset.value='custom';dialog.value='account'}
function applyPreset(){
  const p:Record<string,[string,string,number,boolean]>={qq:['imap.qq.com','smtp.qq.com',465,false],net163:['imap.163.com','smtp.163.com',465,false],net126:['imap.126.com','smtp.126.com',465,false],icloud:['imap.mail.me.com','smtp.mail.me.com',587,true],gmail:['imap.gmail.com','smtp.gmail.com',465,false]}
  const v=p[preset.value]
  if(v){[form.value.imap_host,form.value.smtp_host,form.value.smtp_port,form.value.smtp_starttls]=v;form.value.imap_port=993}
}
function closeDialog(){if(busy.value)return;form.value.password='';dialog.value=null}
async function saveAccount(){await run(async()=>{
  const a=await api.add({...form.value,username:form.value.username.trim()||form.value.email.trim(),name:form.value.name.trim()||form.value.email.trim()})
  form.value=blankAccount();dialog.value=null;await refreshAccounts();view(a.id);status.value=t('Account saved. Test or sync to verify access.','账号已保存，请测试连接或同步。')
})}
async function testAccount(){const id=accountId.value;if(!id)return;await run(async()=>{status.value=await api.test(id)})}
async function sync(){await run(async()=>{
  const targets=currentAccount.value?[currentAccount.value]:accounts.value
  if(!targets.length){newAccount();return}
  let total=0;const failures:string[]=[]
  for(const a of targets){status.value=t('Syncing ','正在同步 ')+a.name;try{total+=await api.sync(a.id)}catch(e){failures.push(a.name+': '+errorText(e))}}
  await load();status.value=t('New cached messages: ','新增缓存邮件：')+total
  if(failures.length)error.value=failures.join('\n')
})}
async function remove(){const a=currentAccount.value;if(!a||!confirm(t('Remove this account and its local cache? Server mail is not deleted.','移除此账号及本地缓存？不会删除服务器邮件。')))return;await run(async()=>{await api.remove(a.id);await refreshAccounts();view(null)})}
async function open(m:Summary){const token=++detailGeneration;try{const result=await api.message(m.id);if(token===detailGeneration)selected.value=result}catch(e){error.value=errorText(e)}}
async function toggleSeen(){const m=selected.value;if(!m)return;await run(async()=>{await api.seen(m.id,!m.seen);await load();if(!unread.value)await open(m)})}
function compose(){if(!accounts.value.length){newAccount();return}draft.value=blankCompose();dialog.value='compose'}
async function send(){await run(async()=>{await api.send(draft.value);dialog.value=null;draft.value=blankCompose();status.value=t('Accepted by SMTP server.','SMTP 服务器已接受邮件。')})}
function resize(event:PointerEvent,which:'nav'|'list'){
  const start=event.clientX, initial=which==='nav'?navWidth.value:listWidth.value
  const move=(e:PointerEvent)=>{const value=Math.max(which==='nav'?180:260,Math.min(which==='nav'?440:650,initial+e.clientX-start));if(which==='nav')navWidth.value=value;else listWidth.value=value}
  const done=()=>{document.removeEventListener('pointermove',move);document.removeEventListener('pointerup',done);document.removeEventListener('pointercancel',done)}
  document.addEventListener('pointermove',move);document.addEventListener('pointerup',done);document.addEventListener('pointercancel',done)
}
const date=(n:number)=>n?new Date(n*1000).toLocaleString(zh.value?'zh-CN':undefined,{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'}):''
const owner=(id:string)=>accounts.value.find(a=>a.id===id)
onMounted(async()=>{try{await refreshAccounts();await load()}catch(e){error.value=errorText(e)}})
</script>

<template>
<div class="shell" :data-theme="theme">
  <header class="top">
    <div class="brand"><span class="logo">✉</span> Mail EX</div>
    <form class="search" @submit.prevent="load()"><input v-model="query" :placeholder="t('Search cached mail','搜索本地邮件')" :aria-label="t('Search','搜索')"><button>{{t('Search','搜索')}}</button></form>
    <div class="top-actions"><select v-model="theme" :aria-label="t('Theme','主题')"><option value="system">{{t('System','跟随系统')}}</option><option value="light">{{t('Light','浅色')}}</option><option value="dark">{{t('Dark','深色')}}</option></select><button @click="zh=!zh">{{zh?'EN':'中文'}}</button></div>
  </header>
  <main class="workspace" :style="{gridTemplateColumns:navWidth+'px 5px '+listWidth+'px 5px minmax(0,1fr)'}">
    <aside class="navigation">
      <button class="primary compose" @click="compose">＋ {{t('New message','写邮件')}}</button>
      <div class="section">{{t('UNIFIED MAIL','统一邮箱')}}</div>
      <button class="nav-row" :class="{active:accountId===null&&!unread}" @click="view(null)">▤ <span>{{t('All inboxes','全部收件箱')}}</span></button>
      <button class="nav-row" :class="{active:accountId===null&&unread}" @click="view(null,true)">● <span>{{t('Unread','未读邮件')}}</span></button>
      <div class="section accounts-header">{{t('ACCOUNTS','邮箱账号')}}<button @click="newAccount" :title="t('Add account','添加账号')">＋</button></div>
      <button v-for="a in accounts" :key="a.id" class="nav-row account" :class="{active:accountId===a.id}" @click="view(a.id)"><span class="dot" :style="{background:a.color}"></span><span class="account-text">{{a.name}}<small>{{a.email}}</small></span></button>
      <div v-if="!accounts.length" class="hint">{{t('Add your first mailbox to get started.','添加第一个邮箱以开始使用。')}}</div>
      <button class="add" @click="newAccount">＋ {{t('Add account','添加账号')}}</button>
      <div class="nav-footer">{{t('Local-first · No Mail EX cloud','本地优先 · 无强制云服务')}}</div>
    </aside>
    <div class="split" role="separator" aria-orientation="vertical" @pointerdown="resize($event,'nav')"></div>
    <section class="message-list">
      <div class="list-head"><h1>{{title}}</h1><div class="toolbar"><button @click="sync" :disabled="busy">↻ {{t('Sync','同步')}}</button><button v-if="currentAccount" @click="testAccount" :disabled="busy">{{t('Test','测试')}}</button><button v-if="currentAccount" class="danger-text" @click="remove" :disabled="busy">{{t('Remove','移除')}}</button></div></div>
      <div class="list-meta">{{messages.length}} {{t('loaded','封已加载')}} <span v-if="loading">{{t('Loading…','加载中…')}}</span></div>
      <div class="items">
        <button v-for="m in messages" :key="m.id" class="mail-row" :class="{selected:selected?.id===m.id,unseen:!m.seen}" @click="open(m)">
          <div class="mail-line"><span class="dot" :style="{background:owner(m.account_id)?.color||'#aaa'}"></span><strong>{{m.sender||t('(Unknown sender)','（未知发件人）')}}</strong><time>{{date(m.timestamp)}}</time></div>
          <div class="subject">{{m.subject||t('(No subject)','（无主题）')}}</div><small>{{owner(m.account_id)?.name}}</small>
        </button>
        <div class="empty" v-if="!messages.length&&!loading"><span>✉</span><p>{{t('No cached messages','没有本地邮件')}}</p><button v-if="accounts.length" @click="sync" :disabled="busy">{{t('Sync inbox','同步收件箱')}}</button></div>
        <button v-if="more" class="load-more" @click="load(false)" :disabled="loading">{{t('Load more','加载更多')}}</button>
      </div>
    </section>
    <div class="split" role="separator" aria-orientation="vertical" @pointerdown="resize($event,'list')"></div>
    <section class="reading">
      <template v-if="selected">
        <div class="reader-toolbar"><button @click="toggleSeen" :disabled="busy">{{selected.seen?t('Mark unread','标为未读'):t('Mark read','标为已读')}}</button></div>
        <article><h2>{{selected.subject||t('(No subject)','（无主题）')}}</h2><div class="sender">{{selected.sender}}</div><div class="muted">{{t('To: ','收件人：')}}{{selected.recipients}}</div><div class="muted">{{owner(selected.account_id)?.email}} · {{date(selected.timestamp)}}</div><hr><p class="notice" v-if="selected.html_only">{{t('HTML-only message: safe preview is not available yet. Original MIME is cached.','此邮件仅有 HTML 内容；安全预览尚未实现，原始邮件已缓存。')}}</p><pre class="body">{{selected.body}}</pre></article>
      </template>
      <div v-else class="empty reading-empty"><span>✉</span><h2>{{t('Select a message','选择一封邮件')}}</h2><p>{{t('Your mail stays on your device.','邮件保存在你的设备中。')}}</p></div>
    </section>
  </main>
  <footer role="status"><span>{{busy?t('Working…','处理中…'):t('Ready','就绪')}}</span><span>{{status}}</span><span class="error" v-if="error">{{error}}</span></footer>
  <div class="overlay" v-if="dialog" @click.self="closeDialog">
    <form v-if="dialog==='account'" class="modal" @submit.prevent="saveAccount">
      <div class="modal-head"><h2>{{t('Add mailbox','添加邮箱')}}</h2><button type="button" @click="closeDialog" :disabled="busy">×</button></div>
      <label>{{t('Provider','服务商')}}<select v-model="preset" @change="applyPreset"><option value="custom">{{t('Custom IMAP / SMTP','自定义 IMAP / SMTP')}}</option><option value="qq">QQ Mail</option><option value="net163">163 Mail</option><option value="net126">126 Mail</option><option value="icloud">iCloud Mail</option><option value="gmail">Gmail (app password)</option></select></label>
      <p class="hint">{{t('Use the provider’s app password or authorization code where required. OAuth is not implemented yet.','按服务商要求使用应用专用密码或授权码。OAuth 尚未实现。')}}</p>
      <label>{{t('Display name','显示名称')}}<input v-model="form.name"></label>
      <label>{{t('Email address','邮箱地址')}}<input v-model="form.email" type="email" required></label>
      <label>{{t('Login username (optional)','登录名（可选）')}}<input v-model="form.username" autocomplete="off"></label>
      <label>{{t('Password / authorization code','密码 / 授权码')}}<input v-model="form.password" type="password" autocomplete="new-password" required></label>
      <div class="field-grid"><label>IMAP TLS<input v-model="form.imap_host" placeholder="imap.example.com" required></label><label>{{t('Port','端口')}}<input type="number" v-model.number="form.imap_port" min="1" max="65535" required></label><label>SMTP<input v-model="form.smtp_host" placeholder="smtp.example.com" required></label><label>{{t('Port','端口')}}<input type="number" v-model.number="form.smtp_port" min="1" max="65535" required></label></div>
      <label class="inline-label"><input type="checkbox" v-model="form.smtp_starttls"> {{t('SMTP required STARTTLS (otherwise implicit TLS)','SMTP 强制 STARTTLS（否则使用隐式 TLS）')}}</label>
      <label class="inline-label">{{t('Account color','账号颜色')}} <input type="color" v-model="form.color"></label>
      <div v-if="error" class="error">{{error}}</div>
      <div class="modal-actions"><button type="button" @click="closeDialog" :disabled="busy">{{t('Cancel','取消')}}</button><button class="primary" :disabled="busy">{{t('Save account','保存账号')}}</button></div>
    </form>
    <form v-else class="modal compose-modal" @submit.prevent="send">
      <div class="modal-head"><h2>{{t('New message','新邮件')}}</h2><button type="button" @click="closeDialog" :disabled="busy">×</button></div>
      <label>{{t('From','发件人')}}<select v-model="draft.account_id" required><option v-for="a in accounts" :key="a.id" :value="a.id">{{a.email}}</option></select></label>
      <label>{{t('To (comma-separated addresses)','收件人（逗号分隔地址）')}}<input v-model="draft.to" required></label>
      <label>Cc<input v-model="draft.cc"></label><label>Bcc<input v-model="draft.bcc"></label>
      <label>{{t('Subject','主题')}}<input v-model="draft.subject"></label><textarea v-model="draft.body" :placeholder="t('Write your message…','撰写邮件正文…')" rows="12"></textarea>
      <div class="error" v-if="error">{{error}}</div><div class="modal-actions"><button type="button" @click="closeDialog" :disabled="busy">{{t('Cancel','取消')}}</button><button class="primary" :disabled="busy">{{t('Send','发送')}}</button></div>
    </form>
  </div>
</div>
</template>
