<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { api,type Account,type Contact,type MailTemplate,type MailRule,type SavedSearch } from '../api'
import RichEditor from './RichEditor.vue'
type Tab='contacts'|'templates'|'rules'|'searches'
const props=defineProps<{zh:boolean;accounts:Account[];initialTab:Tab;searchSeed:SavedSearch}>()
const emit=defineEmits<{close:[];changed:[];useSearch:[value:SavedSearch]}>()
const t=(en:string,cn:string)=>props.zh?cn:en
const tab=ref<Tab>(props.initialTab)
const tabs:Tab[]=['contacts','templates','rules','searches']
const tabName=(k:Tab)=>({contacts:t('Contacts','联系人'),templates:t('Templates','邮件模板'),rules:t('Mail rules','邮件规则'),searches:t('Saved searches','保存的搜索')}[k])
const contacts=ref<Contact[]>([]),templates=ref<MailTemplate[]>([]),rules=ref<MailRule[]>([]),searches=ref<SavedSearch[]>([])
const busy=ref(false),error=ref(''),status=ref(''),keyword=ref(''),editing=ref(false)
const blankContact=():Contact=>({id:'',email:'',name:'',group_name:'',notes:''})
const blankTemplate=():MailTemplate=>({id:'',name:'',subject:'',body:'',html:null})
const blankRule=():MailRule=>({id:'',name:'',account_id:null,enabled:true,sender:'',recipient:'',subject:'',body:'',label:'',match_any:false})
const blankSearch=():SavedSearch=>({...props.searchSeed,id:'',name:'',filters:{...props.searchSeed.filters}})
const contact=ref(blankContact()),template=ref(blankTemplate()),rule=ref(blankRule()),search=ref(blankSearch())
const matches=ref<number|null>(null)
const selectedId=computed(()=>tab.value==='contacts'?contact.value.id:tab.value==='templates'?template.value.id:tab.value==='rules'?rule.value.id:search.value.id)
const rows=computed(()=>{
  const items=tab.value==='contacts'?contacts.value.map(c=>({id:c.id,title:c.name||c.email,subtitle:[c.email,c.group_name].filter(Boolean).join(' · ')})):
    tab.value==='templates'?templates.value.map(x=>({id:x.id,title:x.name,subtitle:x.subject})):
    tab.value==='rules'?rules.value.map(x=>({id:x.id,title:x.name,subtitle:x.label+' · '+(x.enabled?t('Enabled','启用'):t('Disabled','停用'))})):
    searches.value.map(x=>({id:x.id,title:x.name,subtitle:x.query||x.filters.label||x.mode}))
  const q=keyword.value.toLowerCase()
  return items.filter(x=>(x.title+' '+x.subtitle).toLowerCase().includes(q))
})
const failure=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:String(e)
async function refresh(){
  const [c,m,r,s]=await Promise.all([api.contacts(),api.templates(),api.rules(),api.searches()])
  contacts.value=c;templates.value=m;rules.value=r;searches.value=s
}
async function run(fn:()=>Promise<void>){
  if(busy.value)return
  busy.value=true;error.value='';status.value=''
  try{await fn()}catch(e){error.value=failure(e)}finally{busy.value=false}
}
function add(){
  contact.value=blankContact();template.value=blankTemplate();rule.value=blankRule();search.value=blankSearch()
  editing.value=true;matches.value=null;error.value='';status.value=''
}
function select(id:string){
  if(busy.value)return
  if(tab.value==='contacts'){const v=contacts.value.find(x=>x.id===id);if(v)contact.value={...v}}
  if(tab.value==='templates'){const v=templates.value.find(x=>x.id===id);if(v)template.value={...v}}
  if(tab.value==='rules'){const v=rules.value.find(x=>x.id===id);if(v)rule.value={...v}}
  if(tab.value==='searches'){const v=searches.value.find(x=>x.id===id);if(v)search.value={...v,filters:{...v.filters}}}
  editing.value=true;matches.value=null;error.value='';status.value=''
}
watch(tab,()=>{editing.value=false;keyword.value='';error.value='';status.value='';matches.value=null})
watch(rule,()=>{matches.value=null},{deep:true})
async function save(){await run(async()=>{
  if(tab.value==='contacts')contact.value=await api.saveContact(contact.value)
  if(tab.value==='templates')template.value=await api.saveTemplate(template.value)
  if(tab.value==='rules')rule.value=await api.saveRule(rule.value)
  if(tab.value==='searches')search.value=await api.saveSearch(search.value)
  await refresh();emit('changed');status.value=t('Saved locally.','已保存到本地。')
})}
async function remove(){await run(async()=>{
  const id=selectedId.value;if(!id)return
  if(!confirm(tab.value==='rules'?t('Delete this rule and its local labels? Server mail is unchanged.','删除此规则及其产生的本地分类？不会改变服务器邮件。'):t('Delete this record?','删除此记录？')))return
  if(tab.value==='contacts')await api.deleteContact(id)
  if(tab.value==='templates')await api.deleteTemplate(id)
  if(tab.value==='rules')await api.deleteRule(id)
  if(tab.value==='searches')await api.deleteSearch(id)
  editing.value=false;await refresh();emit('changed');status.value=t('Deleted.','已删除。')
})}
async function preview(){await run(async()=>{matches.value=await api.previewRule(rule.value)})}
async function apply(){await run(async()=>{
  if(!rule.value.id)return
  if(!confirm(t('Apply the saved rule to existing cached inbox messages?','将已保存的规则应用于现有本地收件箱邮件？')))return
  const report=await api.applyRule(rule.value.id);matches.value=report.matches
  status.value=t('Newly classified: ','新增分类：')+report.newly_labeled;emit('changed')
})}
function useSearch(){if(search.value.id)emit('useSearch',{...search.value,filters:{...search.value.filters}})}
function esc(s:string){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/\r?\n/g,'<br>')}
function formatTemplate(){
  if(template.value.html===null)template.value.html=esc(template.value.body)
  else if(confirm(t('Remove rich-text formatting?','移除富文本格式？')))template.value.html=null
}
function templateUpdated(v:{html:string;body:string}){template.value.html=v.html;template.value.body=v.body}
onMounted(()=>run(async()=>{await refresh();if(tab.value==='searches')add()}))
</script>
<template>
  <div class="overlay">
    <section class="modal organizer-modal">
      <div class="modal-head"><h2>{{t('Mail EX organizer','邮件管理中心')}}</h2><button :disabled="busy" @click="emit('close')">×</button></div>
      <div class="organizer-tabs"><button v-for="k in tabs" :key="k" :class="{active:tab===k}" :disabled="busy" @click="tab=k">{{tabName(k)}}</button></div>
      <div class="organizer-grid">
        <aside class="organizer-list">
          <div class="organizer-list-head"><input v-model="keyword" :placeholder="t('Search records','搜索记录')"><button class="primary" :disabled="busy" @click="add">＋ {{t('New','新增')}}</button></div>
          <button v-for="row in rows" :key="row.id" class="organizer-row" :class="{active:editing&&selectedId===row.id}" :disabled="busy" @click="select(row.id)"><strong>{{row.title}}</strong><small>{{row.subtitle}}</small></button>
          <p v-if="!rows.length" class="hint">{{t('No records','暂无记录')}}</p>
        </aside>
        <div class="organizer-detail">
          <form v-if="editing" class="organizer-form" @submit.prevent="save">
            <fieldset class="compose-fields" :disabled="busy">
              <template v-if="tab==='contacts'">
                <label>{{t('Name','姓名')}}<input v-model="contact.name"></label>
                <label>{{t('Email address','邮箱地址')}}<input v-model="contact.email" type="email" required></label>
                <label>{{t('Group','分组')}}<input v-model="contact.group_name"></label>
                <label>{{t('Notes','备注')}}<textarea v-model="contact.notes" rows="5"></textarea></label>
              </template>
              <template v-else-if="tab==='templates'">
                <label>{{t('Template name','模板名称')}}<input v-model="template.name" required></label>
                <label>{{t('Subject','主题')}}<input v-model="template.subject"></label>
                <div><button type="button" @click="formatTemplate">{{template.html===null?t('Switch to rich text','切换富文本'):t('Switch to plain text','切换纯文本')}}</button></div>
                <RichEditor v-if="template.html!==null" :html="template.html" :zh="zh" :disabled="busy" @update="templateUpdated"/>
                <label v-else>{{t('Body','正文')}}<textarea v-model="template.body" rows="9"></textarea></label>
              </template>
              <template v-else-if="tab==='rules'">
                <p class="hint">{{t('Rules classify locally. Enabled rules run on newly cached inbox messages. No server-side move, deletion or automatic reply is performed.','规则仅进行本地分类。启用后自动处理新同步到本地的收件箱邮件，不自动移动、删除或回复服务器邮件。')}}</p>
                <label>{{t('Rule name','规则名称')}}<input v-model="rule.name" required></label>
                <label>{{t('Account scope','账号范围')}}<select v-model="rule.account_id"><option :value="null">{{t('All accounts','全部账号')}}</option><option v-for="a in accounts" :key="a.id" :value="a.id">{{a.name}} · {{a.email}}</option></select></label>
                <label>{{t('Match conditions','匹配条件')}}<select v-model="rule.match_any"><option :value="false">{{t('All filled conditions (AND)','全部已填写条件（AND）')}}</option><option :value="true">{{t('Any filled condition (OR)','任一已填写条件（OR）')}}</option></select></label>
                <div class="organizer-fields"><label>{{t('Sender contains','发件人包含')}}<input v-model="rule.sender"></label><label>{{t('Recipient contains','收件人包含')}}<input v-model="rule.recipient"></label><label>{{t('Subject contains','主题包含')}}<input v-model="rule.subject"></label><label>{{t('Body contains','正文包含')}}<input v-model="rule.body"></label></div>
                <label>{{t('Apply local category','应用本地分类')}}<input v-model="rule.label" required></label>
                <label class="inline-label"><input type="checkbox" v-model="rule.enabled"> {{t('Automatically classify newly synced inbox mail','自动分类新同步收件箱邮件')}}</label>
                <div class="organizer-action-row"><button type="button" @click="preview">{{t('Preview matches','预览匹配')}}</button><button v-if="rule.id" type="button" @click="apply">{{t('Apply saved rule to cache','将已保存规则应用到缓存')}}</button><small v-if="matches!==null">{{matches}} {{t('matches','封匹配邮件')}}</small></div>
              </template>
              <template v-else>
                <p class="hint">{{t('A new saved search copies the active view and filters. Opening it runs the search against the current local cache.','新增保存搜索会复制当前视图及筛选条件，打开时对当前本地缓存重新查询。')}}</p>
                <label>{{t('Search name','搜索名称')}}<input v-model="search.name" required></label>
                <label>{{t('Keywords','关键词')}}<input v-model="search.query"></label>
                <div class="organizer-fields"><label>{{t('Sender','发件人')}}<input v-model="search.filters.sender"></label><label>{{t('Recipient','收件人')}}<input v-model="search.filters.recipient"></label><label>{{t('Subject','主题')}}<input v-model="search.filters.subject"></label><label>{{t('Category','分类')}}<input v-model="search.filters.label"></label></div>
                <div class="notice">{{t('Scope: ','范围：')}}{{search.account_id?accounts.find(a=>a.id===search.account_id)?.name||search.account_id:t('All accounts','全部账号')}} · {{search.mode==='folder'?search.folder:search.mode}}</div>
                <button v-if="search.id" type="button" @click="useSearch">{{t('Open saved search','打开保存的搜索')}}</button>
              </template>
            </fieldset>
            <div class="modal-actions"><button v-if="selectedId" type="button" class="danger-text" :disabled="busy" @click="remove">{{t('Delete','删除')}}</button><button class="primary" :disabled="busy">{{t('Save','保存')}}</button></div>
          </form>
          <div v-else class="empty">{{t('Select a record or create a new one.','选择现有记录或新建。')}}</div>
        </div>
      </div>
      <p v-if="error" class="error">{{error}}</p><p v-else-if="status" class="hint">{{status}}</p>
    </section>
  </div>
</template>
