<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import {getCurrentWindow} from '@tauri-apps/api/window'
import {api,type ReaderSnapshot} from '../api'
const data=ref<ReaderSnapshot|null>(null),error=ref(''),plain=ref(false),theme=ref(localStorage.getItem('theme')||'system'),language=ref(localStorage.getItem('language')||'system')
const zh=computed(()=>language.value==='zh'||(language.value==='system'&&navigator.language.toLowerCase().startsWith('zh')))
const t=(en:string,cn:string)=>zh.value?cn:en
const size=ref(Number(localStorage.getItem('readerFontSize')||14))
const date=(n:number)=>n?new Date(n*1000).toLocaleString(zh.value?'zh-CN':undefined):''
const bytes=(n:number)=>n<1024?n+' B':n<1024**2?(n/1024).toFixed(1)+' KiB':(n/1024**2).toFixed(1)+' MiB'
function prefs(e:StorageEvent){if(e.key==='theme')theme.value=e.newValue||'system';if(e.key==='language')language.value=e.newValue||'system';if(e.key==='readerFontSize')size.value=Number(e.newValue||14)}
onMounted(async()=>{
  window.addEventListener('storage',prefs)
  try{data.value=await api.readerSnapshot();await api.frontendReady()}catch(e){error.value=typeof e==='string'?e:String(e)}
})
onUnmounted(()=>window.removeEventListener('storage',prefs))
</script>
<template>
  <div class="shell detached-reader" :data-theme="theme" :style="{'--reader-font':size+'px'}">
    <header class="detached-header">
      <div class="brand"><span class="logo">✉</span> Mail EX</div>
      <span class="hint">{{t('Read-only cached snapshot','只读邮件快照')}}</span>
      <button @click="getCurrentWindow().close()">{{t('Close','关闭')}}</button>
    </header>
    <section v-if="data" class="reader-window-content">
      <article>
        <h2>{{data.mail.subject||t('(No subject)','（无主题）')}}</h2>
        <div class="sender">{{data.mail.sender}}</div>
        <div class="muted">{{t('To: ','收件人：')}}{{data.mail.recipients}}</div>
        <div class="muted">{{date(data.mail.timestamp)}} · {{data.mail.folder}}</div>
        <div v-if="data.labels.length" class="message-labels"><span v-for="label in data.labels" :key="label" class="reader-category">{{label}}</span></div>
        <div v-if="data.invitations.length" class="attachment-panel">
          <div class="attachment-title">{{t('Calendar invitations','会议邀请')}}</div>
          <div v-for="i in data.invitations" :key="i.index" class="attachment-row"><div class="attachment-name"><strong>{{i.event.summary}}</strong><small>{{i.event.start}} – {{i.event.end}}</small></div></div>
        </div>
        <div v-if="data.mail.attachments.length" class="attachment-panel">
          <div class="attachment-title">{{t('Attachments','附件')}} ({{data.mail.attachments.length}})</div>
          <div v-for="a in data.mail.attachments" :key="a.index" class="attachment-row"><span class="attachment-icon">▤</span><div class="attachment-name"><strong>{{a.name}}</strong><small>{{bytes(a.size)}} · {{a.media_type}}</small></div></div>
          <p class="hint">{{t('Use the main mail window to save attachments or modify the message.','请在主窗口下载附件、回复或修改邮件。')}}</p>
        </div>
        <hr>
        <div v-if="data.mail.html!==null" class="read-mode"><button :class="{active:!plain}" @click="plain=false">HTML</button><button :class="{active:plain}" @click="plain=true">{{t('Plain text','纯文本')}}</button><small>{{t('External content blocked','已阻止外部内容')}}</small></div>
        <div v-if="data.mail.html!==null&&!plain" class="safe-mail-html" v-html="data.mail.html"></div><pre v-else class="body">{{data.mail.body}}</pre>
      </article>
    </section>
    <div v-else class="empty reading-empty"><p v-if="error" class="error">{{error}}</p><p v-else>{{t('Loading message…','正在加载邮件…')}}</p></div>
    <footer>{{t('Independent local reading window','独立本地阅读窗口')}}</footer>
  </div>
</template>
