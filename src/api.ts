import { invoke } from '@tauri-apps/api/core'
export interface Account { id:string; name:string; email:string; username:string; imap_host:string; imap_port:number; smtp_host:string; smtp_port:number; smtp_starttls:boolean; color:string }
export type NewAccount = Omit<Account,'id'> & { password:string }
export interface Folder { account_id:string; name:string; label:string; kind:string; available:boolean; total:number; unread:number }
export interface Summary { id:number; account_id:string; sender:string; subject:string; timestamp:number; seen:boolean; folder:string; starred:boolean }
export interface Mail extends Summary { body:string; html_only:boolean; recipients:string }
export interface Compose { account_id:string; to:string; cc:string; bcc:string; subject:string; body:string; in_reply_to?:string|null; references?:string|null }
export interface Draft { id:string; updated_at:number; mail:Compose }
export interface SyncReport { added:number; folders:number }
export const api = {
  accounts: () => invoke<Account[]>('list_accounts'),
  add: (account:NewAccount) => invoke<Account>('add_account',{account}),
  update: (id:string,account:NewAccount) => invoke<Account>('update_account',{id,account}),
  remove: (id:string) => invoke<void>('remove_account',{id}),
  folders: () => invoke<Folder[]>('list_folders'),
  test: (id:string) => invoke<string>('test_account',{id}),
  sync: (id:string,folder:string|null=null,allFolders=false) => invoke<SyncReport>('sync_account',{id,folder,allFolders}),
  messages: (accountId:string|null,folder:string|null,unread:boolean,starred:boolean,query:string,offset:number) => invoke<Summary[]>('list_messages',{accountId,folder,unread,starred,query,offset}),
  message: (id:number) => invoke<Mail>('get_message',{id}),
  seen: (id:number,seen:boolean) => invoke<void>('set_seen',{id,seen}),
  star: (id:number,starred:boolean) => invoke<void>('set_starred',{id,starred}),
  move: (id:number,destination:string) => invoke<void>('move_message',{id,destination}),
  drafts: () => invoke<Draft[]>('list_drafts'),
  saveDraft: (id:string|null,mail:Compose) => invoke<string>('save_draft',{id,mail}),
  deleteDraft: (id:string) => invoke<void>('delete_draft',{id}),
  composeFrom: (id:number,mode:'reply'|'reply_all'|'forward') => invoke<Compose>('compose_from',{id,mode}),
  send: (mail:Compose) => invoke<void>('send_mail',{mail})
}
