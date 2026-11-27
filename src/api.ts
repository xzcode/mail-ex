import { invoke } from '@tauri-apps/api/core'
export interface Account { id: string; name: string; email: string; username: string; imap_host: string; imap_port: number; smtp_host: string; smtp_port: number; smtp_starttls: boolean; color: string }
export type NewAccount = Omit<Account,'id'> & { password: string }
export interface Summary { id: number; account_id: string; sender: string; subject: string; timestamp: number; seen: boolean }
export interface Mail extends Summary { body: string; html_only: boolean; recipients: string }
export interface Compose { account_id: string; to: string; cc: string; bcc: string; subject: string; body: string }
export const api = {
  accounts: () => invoke<Account[]>('list_accounts'),
  add: (account:NewAccount) => invoke<Account>('add_account',{account}),
  remove: (id:string) => invoke<void>('remove_account',{id}),
  test: (id:string) => invoke<string>('test_account',{id}),
  sync: (id:string) => invoke<number>('sync_account',{id}),
  messages: (accountId:string|null, unread:boolean, query:string, offset:number) => invoke<Summary[]>('list_messages',{accountId,unread,query,offset}),
  message: (id:number) => invoke<Mail>('get_message',{id}),
  seen: (id:number,seen:boolean) => invoke<void>('set_seen',{id,seen}),
  send: (mail:Compose) => invoke<void>('send_mail',{mail})
}
