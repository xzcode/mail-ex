import { invoke } from '@tauri-apps/api/core'
export interface Account { id:string; name:string; email:string; username:string; imap_host:string; imap_port:number; smtp_host:string; smtp_port:number; smtp_starttls:boolean; color:string; signature:string; auth_kind:'password'|'google'|'microsoft'|'outlook' }
export type NewAccount = Omit<Account,'id'|'auth_kind'> & { password:string }
export interface OAuthRequest { provider:'google'|'microsoft'|'outlook'; client_id:string; desktop_client_secret:string; email:string; name:string; color:string; account_id:string|null }
export interface SearchFilters { sender:string; recipient:string; subject:string; since:number|null; until:number|null }
export type BatchOperation='read'|'unread'|'star'|'unstar'|'move'|'archive'|'trash'
export interface BatchReport { succeeded:number[]; failed:{id:number;reason:string}[] }
export interface Folder { account_id:string; name:string; label:string; kind:string; available:boolean; total:number; unread:number }
export interface Summary { id:number; account_id:string; sender:string; subject:string; timestamp:number; seen:boolean; folder:string; starred:boolean }
export interface AttachmentInfo { index:number; name:string; media_type:string; size:number; previewable:boolean }
export interface AttachmentPreview { name:string; media_type:string; data:string }
export interface OutgoingAttachment { name:string; media_type:string; data:string }
export interface Mail extends Summary { body:string; html_only:boolean; recipients:string; html:string|null; attachments:AttachmentInfo[] }
export interface Compose { account_id:string; to:string; cc:string; bcc:string; subject:string; body:string; in_reply_to?:string|null; references?:string|null; html?:string|null; attachments?:OutgoingAttachment[]; save_sent_copy?:boolean }
export interface Draft { id:string; updated_at:number; mail:Compose }
export interface SendReport { sent_copy:'saved'|'not_requested'|'failed_or_uncertain' }
export interface SyncReport { added:number; folders:number }
export const api = {
  accounts: () => invoke<Account[]>('list_accounts'),
  authorize: (request:OAuthRequest) => invoke<Account>('authorize_account',{request}),
  cancelOAuth: () => invoke<void>('cancel_oauth'),
  add: (account:NewAccount) => invoke<Account>('add_account',{account}),
  update: (id:string,account:NewAccount) => invoke<Account>('update_account',{id,account}),
  remove: (id:string) => invoke<void>('remove_account',{id}),
  folders: () => invoke<Folder[]>('list_folders'),
  test: (id:string) => invoke<string>('test_account',{id}),
  sync: (id:string,folder:string|null=null,allFolders=false) => invoke<SyncReport>('sync_account',{id,folder,allFolders}),
  messages: (accountId:string|null,folder:string|null,unread:boolean,starred:boolean,query:string,offset:number,filters:SearchFilters) => invoke<Summary[]>('list_messages',{accountId,folder,unread,starred,query,offset,filters}),
  message: (id:number) => invoke<Mail>('get_message',{id}),
  seen: (id:number,seen:boolean) => invoke<void>('set_seen',{id,seen}),
  star: (id:number,starred:boolean) => invoke<void>('set_starred',{id,starred}),
  batch: (ids:number[],operation:BatchOperation,destination:string|null=null) => invoke<BatchReport>('batch_messages',{ids,operation,destination}),
  move: (id:number,destination:string) => invoke<void>('move_message',{id,destination}),
  drafts: () => invoke<Draft[]>('list_drafts'),
  draft: (id:string) => invoke<Draft>('get_draft',{id}),
  saveDraft: (id:string|null,mail:Compose) => invoke<string>('save_draft',{id,mail}),
  deleteDraft: (id:string) => invoke<void>('delete_draft',{id}),
  composeFrom: (id:number,mode:'reply'|'reply_all'|'forward') => invoke<Compose>('compose_from',{id,mode}),
  preview: (id:number,index:number) => invoke<AttachmentPreview>('attachment_preview',{id,index}),
  download: (id:number,index:number) => invoke<boolean>('attachment_save',{id,index}),
  send: (mail:Compose) => invoke<SendReport>('send_mail',{mail})
}
