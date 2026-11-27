<script setup lang="ts">
import {computed,onMounted,ref} from 'vue'
import {api,type Account,type CalendarEvent,type CalendarSource,type NewCalendarSource,type Compose} from '../api'
import RecipientField from './RecipientField.vue'
import type {Contact} from '../api'
const props=defineProps<{zh:boolean;accounts:Account[];contacts:Contact[]}>()
const emit=defineEmits<{close:[];compose:[mail:Compose]}>()
const t=(en:string,cn:string)=>props.zh?cn:en
const events=ref<CalendarEvent[]>([]),sources=ref<CalendarSource[]>([]),busy=ref(false),error=ref(''),status=ref('')
const tab=ref<'calendar'|'sources'>('calendar'),notifications=ref(false),selected=ref<CalendarEvent|null>(null)
const source=ref<NewCalendarSource>({name:'',url:'',username:'',password:''})
const realAccounts=computed(()=>props.accounts.filter(a=>a.auth_kind!=='local'))
const invitationAccount=ref(''),invitationRecipients=ref('')
function pad(n:number){return String(n).padStart(2,'0')}
function dayKey(d:Date){return d.getFullYear()+'-'+pad(d.getMonth()+1)+'-'+pad(d.getDate())}
function monthKey(d:Date){return d.getFullYear()+'-'+pad(d.getMonth()+1)}
const today=dayKey(new Date()),month=ref(monthKey(new Date())),day=ref(today)
function localValue(value:string){if(!value.endsWith('Z'))return value;const d=new Date(value);return Number.isFinite(d.getTime())?dayKey(d)+'T'+pad(d.getHours())+':'+pad(d.getMinutes())+':'+pad(d.getSeconds()):value}
function dayBounds(key:string){const [y,m,d]=key.split('-').map(Number);return {start:key,end:dayKey(new Date(y,m-1,d+1))}}
function containsDay(e:CalendarEvent,key:string){
  const b=dayBounds(key),start=localValue(e.start),end=localValue(e.end)
  if(e.all_day)return start<b.end&&end>b.start
  return start.slice(0,10)<=key&&((end.slice(0,16)>b.start+'T00:00')||start.slice(0,10)===key)
}
const days=computed(()=>{
  const [y,m]=month.value.split('-').map(Number)
  const first=new Date(y,m-1,1);const start=new Date(y,m-1,1-((first.getDay()+6)%7))
  return Array.from({length:42},(_,i)=>{const d=new Date(start.getFullYear(),start.getMonth(),start.getDate()+i);const key=dayKey(d);return {key,label:d.getDate(),inMonth:monthKey(d)===month.value,count:events.value.filter(e=>containsDay(e,key)).length}})
})
const agenda=computed(()=>events.value.filter(e=>containsDay(e,day.value)).sort((a,b)=>localValue(a.start).localeCompare(localValue(b.start))))
const monthLabel=computed(()=>{const [y,m]=month.value.split('-').map(Number);return new Date(y,m-1,1).toLocaleDateString(props.zh?'zh-CN':'en',{month:'long',year:'numeric'})})
const originLabel=(e:CalendarEvent)=>e.origin==='local'?t('Local','本地'):e.origin==='import'?t('Imported','导入'):sources.value.find(s=>s.id===e.source_id)?.name||'CalDAV'
const showTime=(e:CalendarEvent)=>e.all_day?t('All day','全天'):localValue(e.start).slice(11,16)+(e.timezone&&e.timezone!=='UTC'?' ('+e.timezone+')':'')
const weekdays=computed(()=>props.zh?['一','二','三','四','五','六','日']:['Mon','Tue','Wed','Thu','Fri','Sat','Sun'])
function moveMonth(n:number){const [y,m]=month.value.split('-').map(Number);const next=new Date(y,m-1+n,1);month.value=monthKey(next);day.value=dayKey(next);selected.value=null}
function chooseDay(key:string){day.value=key;month.value=key.slice(0,7);selected.value=null}
function resetToday(){chooseDay(dayKey(new Date()))}
function newEvent(){
  selected.value={id:'',source_id:null,origin:'local',uid:'',summary:'',start:day.value+'T09:00',end:day.value+'T10:00',all_day:false,timezone:'',location:'',description:'',organizer:'',attendees:[],sequence:0,recurrence_id:'',recurring:false,method:'',reminder_minutes:null}
  invitationAccount.value=realAccounts.value[0]?.id||'';invitationRecipients.value=''
}
function editEvent(e:CalendarEvent){selected.value={...e,attendees:[...e.attendees]};invitationAccount.value=realAccounts.value[0]?.id||'';invitationRecipients.value=''}
function setAllDay(value:boolean){
  const e=selected.value;if(!e)return
  e.all_day=value;e.timezone=''
  if(value){const start=e.start.slice(0,10);e.start=start;e.end=dayBounds(start).end}
  else{e.start=e.start.slice(0,10)+'T09:00';e.end=e.start.slice(0,10)+'T10:00'}
}
function editDate(key:'start'|'end',ev:Event){if(selected.value)selected.value[key]=(ev.target as HTMLInputElement).value}
async function refresh(){const [a,b,c]=await Promise.all([api.calendarEvents(),api.calendarSources(),api.notificationEnabled()]);events.value=a;sources.value=b;notifications.value=c}
const errorText=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:String(e)
async function run(fn:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';status.value='';try{await fn()}catch(e){error.value=errorText(e)}finally{busy.value=false}}
async function save(){if(!selected.value)return;await run(async()=>{const e=await api.saveEvent(selected.value!);await refresh();day.value=localValue(e.start).slice(0,10);month.value=day.value.slice(0,7);editEvent(e);status.value=t('Event saved.','日程已保存。')})}
async function removeEvent(){const e=selected.value;if(!e?.id||e.origin==='remote')return;if(!confirm(t('Delete this local calendar event?','删除这条本地日程？')))return;await run(async()=>{await api.deleteEvent(e.id);selected.value=null;await refresh()})}
async function importIcs(){await run(async()=>{const r=await api.importIcs();if(r){status.value=t('Imported: ','已导入：')+r.added+' · '+t('Skipped: ','跳过：')+r.skipped;await refresh()}})}
async function exportIcs(){const e=selected.value;if(e?.id)await run(async()=>{if(await api.exportIcs(e.id))status.value=t('ICS exported.','ICS 已导出。')})}
async function invite(){const e=selected.value;if(!e?.id||!invitationAccount.value||!invitationRecipients.value.trim())return;await run(async()=>{const mail=await api.meetingInvite(e.id,invitationAccount.value,invitationRecipients.value);emit('compose',mail)})}
async function addSource(){await run(async()=>{await api.addCalendarSource({...source.value});source.value={name:'',url:'',username:'',password:''};await refresh();status.value=t('Calendar added. Click Sync to download events.','已添加日历，请点击同步下载事件。')})}
async function syncSource(s:CalendarSource){await run(async()=>{const r=await api.syncCalendar(s.id);await refresh();if(selected.value?.source_id===s.id)selected.value=null;status.value=t('Synchronized events: ','已同步日程：')+r.events+' · '+t('Resources: ','资源：')+r.resources})}
async function removeSource(s:CalendarSource){if(!confirm(t('Remove this calendar configuration and local cache? Server calendar is unchanged.','移除此日历配置及本地缓存？服务器日历不会改变。')))return;await run(async()=>{await api.removeCalendarSource(s.id);await refresh();if(selected.value?.source_id===s.id)selected.value=null})}
async function changeNotifications(ev:Event){const enabled=(ev.target as HTMLInputElement).checked;await run(async()=>{await api.setNotifications(enabled);notifications.value=await api.notificationEnabled()});(ev.target as HTMLInputElement).checked=notifications.value}
onMounted(()=>run(refresh))
</script>
<template>
  <div class="overlay">
    <section class="modal calendar-modal">
      <div class="modal-head"><h2>{{t('Calendar','日历')}}</h2><button :disabled="busy" @click="emit('close')">×</button></div>
      <div class="organizer-tabs"><button :class="{active:tab==='calendar'}" @click="tab='calendar'">{{t('Calendar','日历')}}</button><button :class="{active:tab==='sources'}" @click="tab='sources'">CalDAV / {{t('Settings','设置')}}</button><button :disabled="busy" @click="importIcs">{{t('Import ICS','导入 ICS')}}</button></div>
      <div v-if="tab==='calendar'" class="calendar-grid">
        <div class="calendar-month">
          <div class="calendar-month-head"><button @click="moveMonth(-1)">‹</button><strong>{{monthLabel}}</strong><button @click="moveMonth(1)">›</button><button @click="resetToday">{{t('Today','今天')}}</button></div>
          <div class="calendar-days"><div v-for="w in weekdays" :key="w" class="calendar-weekday">{{w}}</div><button v-for="d in days" :key="d.key" :class="{active:day===d.key,muted:!d.inMonth,today:d.key===today}" @click="chooseDay(d.key)"><span>{{d.label}}</span><small v-if="d.count">{{d.count}}</small></button></div>
          <div class="calendar-agenda-head"><strong>{{day}}</strong><button class="primary" :disabled="busy" @click="newEvent">＋ {{t('Event','新建日程')}}</button></div>
          <div v-for="e in agenda" :key="e.id" class="calendar-event-row" @click="editEvent(e)" role="button" tabindex="0" @keydown.enter="editEvent(e)"><span>{{showTime(e)}}</span><div><strong>{{e.summary||t('(No title)','（无标题）')}}</strong><small>{{originLabel(e)}}<template v-if="e.recurring"> · {{t('Recurring series','重复日程')}}</template></small></div></div>
          <p v-if="!agenda.length" class="hint">{{t('No events for this day.','当日无日程。')}}</p>
          <p class="hint">{{t('Recurring series show their original/exception dates; recurrence expansion is not yet supported. Named time zones retain their original wall time.','重复日程显示原始日期或例外日期，尚未展开重复周期；具名时区保留原始当地时间。')}}</p>
        </div>
        <div class="calendar-detail">
          <form v-if="selected" class="organizer-form" @submit.prevent="save">
            <div class="calendar-detail-head"><h3>{{selected.id?t('Event details','日程详情'):t('New event','新建日程')}}</h3><small>{{originLabel(selected)}}</small></div>
            <fieldset class="compose-fields" :disabled="busy||selected.origin!=='local'">
              <label>{{t('Title','标题')}}<input v-model="selected.summary" required></label>
              <label class="inline-label"><input type="checkbox" :checked="selected.all_day" @change="setAllDay(($event.target as HTMLInputElement).checked)"> {{t('All day','全天')}}</label>
              <label>{{t('Start','开始')}}<input :type="selected.all_day?'date':'datetime-local'" :value="selected.all_day?selected.start:localValue(selected.start).slice(0,16)" @change="editDate('start',$event)" required></label>
              <label>{{selected.all_day?t('End date (exclusive)','结束日期（不含当天）'):t('End','结束')}}<input :type="selected.all_day?'date':'datetime-local'" :value="selected.all_day?selected.end:localValue(selected.end).slice(0,16)" @change="editDate('end',$event)" required></label>
              <label>{{t('Location','地点')}}<input v-model="selected.location"></label>
              <label>{{t('Description','描述')}}<textarea v-model="selected.description" rows="4"></textarea></label>
              <label v-if="selected.origin==='local'">{{t('Reminder (minutes before start; empty = none)','提前提醒分钟数（留空不提醒）')}}<input type="number" min="0" step="1" :value="selected.reminder_minutes??''" @input="selected.reminder_minutes=($event.target as HTMLInputElement).value===''?null:Number(($event.target as HTMLInputElement).value)"></label>
            </fieldset>
            <div v-if="selected.origin!=='local'" class="notice">{{t('Imported and CalDAV events preserve their source data and are read-only.','导入及 CalDAV 日程保留原始数据，目前只读。')}}<span v-if="selected.timezone"> · {{selected.timezone}}</span></div>
            <p v-if="selected.organizer" class="hint">{{t('Organizer: ','组织者：')}}{{selected.organizer}}</p><p v-if="selected.attendees.length" class="hint">{{t('Attendees: ','与会者：')}}{{selected.attendees.join(', ')}}</p>
            <div class="organizer-action-row"><button v-if="selected.origin==='local'" class="primary" :disabled="busy">{{t('Save','保存')}}</button><button v-if="selected.id" type="button" :disabled="busy" @click="exportIcs">{{t('Export ICS','导出 ICS')}}</button><button v-if="selected.id&&selected.origin!=='remote'" type="button" class="danger-text" :disabled="busy" @click="removeEvent">{{t('Delete','删除')}}</button></div>
            <div v-if="selected.id&&selected.origin==='local'&&realAccounts.length" class="data-section"><h3>{{t('Meeting invitation','会议邀请')}}</h3><p class="hint">{{t('Creates a draft from the saved event; review and send it manually.','根据已保存日程生成邀请草稿，请确认后发送。')}}</p><label>{{t('Sending mailbox','发件邮箱')}}<select v-model="invitationAccount"><option v-for="a in realAccounts" :key="a.id" :value="a.id">{{a.email}}</option></select></label><label>{{t('Attendees (comma-separated)','与会者（逗号分隔）')}}<RecipientField v-model="invitationRecipients" :contacts="contacts" :zh="zh"/></label><button type="button" :disabled="busy||!invitationRecipients.trim()" @click="invite">{{t('Create invitation draft','生成会议邀请草稿')}}</button></div>
          </form>
          <div v-else class="empty">{{t('Select a date and event, or create a new event.','选择日期及日程，或新建日程。')}}</div>
        </div>
      </div>
      <div v-else class="data-content">
        <div class="data-section"><h3>{{t('Notifications','系统通知')}}</h3><label class="inline-label"><input type="checkbox" :checked="notifications" :disabled="busy" @change="changeNotifications">{{t('New mail and calendar reminders','新邮件与日程提醒')}}</label><p class="hint">{{t('Notifications contain only counts, not mail subjects or event titles. Reminders work while Mail EX runs, subject to OS permissions.','通知仅显示数量，不显示邮件主题或日程标题。应用运行期间提醒，受系统权限影响。')}}</p></div>
        <div class="data-section"><h3>{{t('CalDAV calendars','CalDAV 日历')}}</h3><p class="hint">{{t('Use the exact HTTPS collection URL and app password. Read-only synchronization; no remote changes.','填写实际 HTTPS 日历集合地址及应用专用密码。只读同步，不修改远程日历。')}}</p><div v-for="s in sources" :key="s.id" class="calendar-source"><div><strong>{{s.name}}</strong><small>{{s.url}}</small><small v-if="s.last_sync">{{new Date(s.last_sync*1000).toLocaleString()}}</small></div><button :disabled="busy" @click="syncSource(s)">{{t('Sync','同步')}}</button><button class="danger-text" :disabled="busy" @click="removeSource(s)">{{t('Remove','移除')}}</button></div><form class="organizer-form" @submit.prevent="addSource"><fieldset class="compose-fields" :disabled="busy"><label>{{t('Name','名称')}}<input v-model="source.name" required></label><label>{{t('Calendar collection URL','日历集合地址')}}<input v-model="source.url" type="url" placeholder="https://calendar.example.org/calendars/user/work/" required></label><label>{{t('Username','用户名')}}<input v-model="source.username" required></label><label>{{t('App password','应用密码')}}<input v-model="source.password" type="password" autocomplete="off" required></label></fieldset><button class="primary" :disabled="busy">{{t('Add calendar','添加日历')}}</button></form></div>
      </div>
      <p v-if="error" class="error">{{error}}</p><p v-if="status" class="data-status">{{status}}</p>
      <div class="modal-actions"><button :disabled="busy" @click="emit('close')">{{t('Close','关闭')}}</button><button :disabled="busy" @click="run(refresh)">{{t('Refresh','刷新')}}</button></div>
    </section>
  </div>
</template>
