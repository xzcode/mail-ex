<script setup lang="ts">
import {computed,onMounted,ref} from 'vue'
import {api,type Account,type StorageStats,type ImportReport,type CacheReport,type SnapshotReport} from '../api'
const props=defineProps<{zh:boolean;accounts:Account[]}>()
const emit=defineEmits<{close:[];changed:[]}>()
const t=(en:string,cn:string)=>props.zh?cn:en
const stats=ref<StorageStats|null>(null),pending=ref<string|null>(null),busy=ref(false),error=ref(''),status=ref('')
const scope=ref(''),before=ref(''),preview=ref<CacheReport|null>(null)
const tab=ref<'storage'|'transfer'|'backup'>('storage')
const realAccounts=computed(()=>props.accounts.filter(a=>a.auth_kind!=='local'))
const bytes=(n:number)=>n<1024?n+' B':n<1024**2?(n/1024).toFixed(1)+' KiB':n<1024**3?(n/1024**2).toFixed(1)+' MiB':(n/1024**3).toFixed(2)+' GiB'
const importText=(r:ImportReport)=>t('Added: ','新增：')+r.added+' · '+t('Skipped: ','跳过：')+r.skipped+' · '+t('Invalid: ','无效：')+r.invalid
const errorText=(e:unknown)=>typeof e==='string'?e:e instanceof Error?e.message:String(e)
async function refresh(){[stats.value,pending.value]=await Promise.all([api.storageStats(),api.pendingStorage()])}
async function run(fn:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';status.value='';try{await fn()}catch(e){error.value=errorText(e)}finally{busy.value=false}}
async function imported(action:()=>Promise<ImportReport|null>){await run(async()=>{const result=await action();if(result){status.value=importText(result);await refresh();emit('changed')}})}
async function exported(action:()=>Promise<number|null>){await run(async()=>{const n=await action();if(n!==null)status.value=t('Exported: ','已导出：')+n})}
function cutoff(){if(!before.value)return null;const [y,m,d]=before.value.split('-').map(Number);return Math.floor(new Date(y,m-1,d).getTime()/1000)}
async function previewCache(){await run(async()=>{preview.value=await api.cachePreview(scope.value||null,cutoff())})}
async function clean(){
  if(!preview.value)return
  if(!confirm(t('Remove matching server-mail cache only? These messages may return on the next synchronization. Local imports, drafts and outbox are retained.','仅清理匹配的服务器邮件缓存？下次同步时可能重新下载。本地导入邮件、草稿和发件箱将保留。')))return
  await run(async()=>{const r=await api.clearCache(scope.value||null,cutoff());status.value=t('Removed cached messages: ','已移除缓存邮件：')+r.messages;preview.value=null;await refresh();emit('changed')})
}
async function compact(){if(!confirm(t('Compact the local database? This may require temporary disk space.','压缩本地数据库？操作可能需要临时磁盘空间。')))return;await run(async()=>{await api.optimizeStorage();status.value=t('Database maintenance completed.','数据库维护完成。');await refresh()})}
async function snapshot(kind:'backup'|'restore'|'copy'){
  const message=kind==='backup'?t('The backup contains private mail, attachments, credentials and their decryption key. Keep it in a trusted private folder.','备份包含私人邮件、附件、凭据及解密密钥，请保存在可信的私有目录。'):t('Create a separate data copy and use it after restart? The current data is retained. Changes made after the copy will not be in it. Pending sends in the copied data are disabled. The running app still uses the current data until exit.','创建独立数据副本并在重启后使用？当前数据会保留，复制后的新修改不会进入副本。副本中的待发送任务会停用；当前应用在退出前仍使用原数据。')
  if(!confirm(message))return
  await run(async()=>{const r:SnapshotReport|null=await (kind==='backup'?api.createBackup():kind==='restore'?api.restoreBackup():api.copyStorage());if(r){status.value=t('Created: ','已创建：')+r.directory+' · '+t('Suspended sends: ','已停用发送任务：')+r.suspended_sends;await refresh()}})
}
onMounted(()=>run(refresh))
</script>
<template>
  <div class="overlay">
    <section class="modal data-modal">
      <div class="modal-head"><h2>{{t('Data management','数据管理')}}</h2><button :disabled="busy" @click="emit('close')">×</button></div>
      <div class="organizer-tabs"><button :class="{active:tab==='storage'}" @click="tab='storage'">{{t('Storage','存储')}}</button><button :class="{active:tab==='transfer'}" @click="tab='transfer'">{{t('Import / Export','导入 / 导出')}}</button><button :class="{active:tab==='backup'}" @click="tab='backup'">{{t('Backup / Restore','备份 / 恢复')}}</button></div>
      <div v-if="tab==='storage'" class="data-content">
        <template v-if="stats">
          <div class="data-path"><small>{{t('Current data directory','当前数据目录')}}</small><code>{{stats.directory}}</code></div>
          <div v-if="pending" class="notice">{{t('Restart to use: ','重启后使用：')}}{{pending}}</div>
          <div class="storage-grid"><div><small>{{t('Database files','数据库文件')}}</small><strong>{{bytes(stats.database_bytes+stats.wal_bytes+stats.shm_bytes)}}</strong></div><div><small>{{t('Raw mail','原始邮件')}}</small><strong>{{bytes(stats.raw_mail_bytes)}}</strong></div><div><small>{{t('Cached messages','缓存邮件')}}</small><strong>{{stats.messages}}</strong></div><div><small>{{t('Imported mail','导入邮件')}}</small><strong>{{stats.local_messages}}</strong></div><div><small>{{t('Drafts','草稿')}}</small><strong>{{stats.drafts}}</strong></div><div><small>{{t('Outbox records','发件记录')}}</small><strong>{{stats.outbox}}</strong></div></div>
        </template>
        <div class="data-section"><h3>{{t('Cache maintenance','缓存维护')}}</h3><p class="hint">{{t('Only synchronized mail is eligible. Local archives, drafts, contacts and delivery records are excluded. Raw bytes are an estimate, not guaranteed reclaimed disk space.','仅服务器同步邮件可清理。本地归档、草稿、联系人和发送记录不受影响。原始字节数不等于实际可释放空间。')}}</p><div class="organizer-fields"><label>{{t('Account','邮箱账号')}}<select v-model="scope" :disabled="busy" @change="preview=null"><option value="">{{t('All server mailboxes','所有服务器邮箱')}}</option><option v-for="a in realAccounts" :key="a.id" :value="a.id">{{a.name}}</option></select></label><label>{{t('Older than (empty = all)','早于日期（留空为全部）')}}<input type="date" v-model="before" :disabled="busy" @change="preview=null"></label></div><div class="organizer-action-row"><button :disabled="busy" @click="previewCache">{{t('Preview cleanup','预览清理')}}</button><button class="danger-text" :disabled="busy||!preview?.messages" @click="clean">{{t('Clear matching cache','清理匹配缓存')}}</button><button :disabled="busy" @click="compact">{{t('Compact database','压缩数据库')}}</button></div><p v-if="preview">{{preview.messages}} {{t('messages','封邮件')}} · {{bytes(preview.raw_bytes)}}</p></div>
      </div>
      <div v-else-if="tab==='transfer'" class="data-content">
        <div class="data-section"><h3>{{t('Mail archives','邮件归档')}}</h3><p class="hint">{{t('Imported messages remain local, searchable and read-only with respect to server mail. EML exports preserve raw MIME; MBOX supports mboxrd.','导入邮件仅保存在本地，可搜索，不会影响服务器。EML 导出保留原始 MIME；MBOX 支持 mboxrd。')}}</p><div class="organizer-action-row"><button :disabled="busy" @click="imported(()=>api.importMail('eml'))">{{t('Import EML files','导入 EML')}}</button><button :disabled="busy" @click="imported(()=>api.importMail('mbox'))">{{t('Import MBOX files','导入 MBOX')}}</button><button :disabled="busy" @click="exported(()=>api.exportMbox(null,null))">{{t('Export all cached mail as MBOX','全部缓存导出 MBOX')}}</button></div></div>
        <div class="data-section"><h3>{{t('Address book','地址簿')}}</h3><p class="hint">CSV: email, name, group, notes</p><div class="organizer-action-row"><button :disabled="busy" @click="imported(api.importContactsCsv)">{{t('Import CSV','导入 CSV')}}</button><button :disabled="busy" @click="exported(api.exportContactsCsv)">{{t('Export CSV','导出 CSV')}}</button></div></div>
        <div class="data-section"><h3>{{t('Account configuration','邮箱配置')}}</h3><p class="hint">{{t('JSON includes settings, not passwords or OAuth tokens. Imported accounts require fresh credentials before connecting.','JSON 仅含配置，不含密码或 OAuth 令牌。导入后需要重新填写凭据或授权。')}}</p><div class="organizer-action-row"><button :disabled="busy" @click="imported(api.importAccountsJson)">{{t('Import JSON','导入 JSON')}}</button><button :disabled="busy" @click="exported(api.exportAccountsJson)">{{t('Export JSON','导出 JSON')}}</button></div></div>
      </div>
      <div v-else class="data-content">
        <div class="notice">{{t('Backups are private, unencrypted snapshots containing the database and credential key. Only restore backups you created or fully trust. A checksum detects accidental corruption, not authenticity.','备份是未加密的私有快照，包含数据库与凭据密钥。仅恢复自己创建或完全信任的备份。校验和只能检测损坏，不代表来源可信。')}}</div>
        <div class="data-section"><h3>{{t('Backup and restore','备份与恢复')}}</h3><p class="hint">{{t('Create a new backup folder or restore into a new data directory. Restored pending delivery is never automatically retried. Restore requires the same schema version.','创建新的备份目录，或恢复到全新数据目录。恢复的待发送任务不会自动重发。目前仅支持相同数据库架构版本。')}}</p><div class="organizer-action-row"><button :disabled="busy" @click="snapshot('backup')">{{t('Create backup','创建备份')}}</button><button :disabled="busy" @click="snapshot('restore')">{{t('Restore to new directory','恢复到新目录')}}</button></div></div>
        <div class="data-section"><h3>{{t('Data location','数据存储位置')}}</h3><p class="hint">{{t('Copy current data to a selected parent folder and activate the new location on restart. Existing data is not overwritten or deleted.','将当前数据复制至选定目录下，重启后启用新位置。不覆盖、不删除原数据。')}}</p><button :disabled="busy" @click="snapshot('copy')">{{t('Change storage location','更改存储位置')}}</button><div v-if="pending" class="notice">{{t('Restart to use: ','重启后使用：')}}{{pending}}</div></div>
      </div>
      <div v-if="error" class="error">{{error}}</div><p v-if="status" class="data-status">{{status}}</p>
      <div class="modal-actions"><button :disabled="busy" @click="emit('close')">{{t('Close','关闭')}}</button><button :disabled="busy" @click="run(refresh)">{{t('Refresh','刷新')}}</button></div>
    </section>
  </div>
</template>
