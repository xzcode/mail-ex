<script setup lang="ts">
import {ref,watch} from 'vue'
import {api,type Invitation,type Account,type Compose} from '../api'
const props=defineProps<{zh:boolean;messageId:number;invitations:Invitation[];accounts:Account[];disabled:boolean}>()
const emit=defineEmits<{compose:[mail:Compose];status:[text:string];error:[text:string]}>()
const t=(en:string,cn:string)=>props.zh?cn:en
const busy=ref(false),choices=ref<Record<number,string>>({})
const valid=(inv:Invitation)=>props.accounts.filter(a=>a.auth_kind!=='local'&&inv.event.attendees.some(e=>e.toLowerCase()===a.email.toLowerCase()))
watch(()=>props.messageId,()=>{choices.value={}})
const choice=(inv:Invitation)=>choices.value[inv.index]||valid(inv)[0]?.id||''
async function run(fn:()=>Promise<void>){if(busy.value||props.disabled)return;busy.value=true;try{await fn()}catch(e){emit('error',typeof e==='string'?e:String(e))}finally{busy.value=false}}
async function add(inv:Invitation){await run(async()=>{const r=await api.addInvitation(props.messageId,inv.index);emit('status',r.added?t('Invitation added to local calendar.','会议已加入本地日历。'):t('Invitation already exists.','会议已存在。'))})}
async function reply(inv:Invitation,partstat:string){await run(async()=>{const m=await api.invitationReply(props.messageId,inv.index,choice(inv),partstat);emit('compose',m)})}
</script>
<template>
  <div v-if="invitations.length" class="invitation-panel">
    <div v-for="i in invitations" :key="i.index" class="invitation-card">
      <div><strong>{{t('Meeting invitation','会议邀请')}} · {{i.event.summary}}</strong><small>{{i.event.start}} – {{i.event.end}} <template v-if="i.event.timezone">· {{i.event.timezone}}</template></small><small>{{t('Organizer: ','组织者：')}}{{i.event.organizer||'—'}}</small><small v-if="i.event.location">{{i.event.location}}</small></div>
      <div v-if="valid(i).length&&i.event.organizer" class="invitation-actions"><select :value="choice(i)" :disabled="busy||disabled" @change="choices[i.index]=($event.target as HTMLSelectElement).value"><option v-for="a in valid(i)" :key="a.id" :value="a.id">{{a.email}}</option></select><button :disabled="busy||disabled" @click="reply(i,'ACCEPTED')">{{t('Accept','接受')}}</button><button :disabled="busy||disabled" @click="reply(i,'TENTATIVE')">{{t('Tentative','暂定')}}</button><button :disabled="busy||disabled" @click="reply(i,'DECLINED')">{{t('Decline','拒绝')}}</button></div>
      <small>{{t('Responses open a draft; no email is sent automatically.','回复会打开草稿，不自动发件。')}}</small>
      <button :disabled="busy||disabled" @click="add(i)">{{t('Add to local calendar','添加到本地日历')}}</button>
    </div>
  </div>
</template>
