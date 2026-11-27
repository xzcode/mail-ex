import { invoke } from '@tauri-apps/api/core'
export interface Account { id:string; name:string; email:string; username:string; imap_host:string; imap_port:number; smtp_host:string; smtp_port:number; smtp_starttls:boolean; color:string; signature:string; auth_kind:'password'|'google'|'microsoft'|'outlook'|'local'|'pop3' }
export type NewAccount = Omit<Account,'id'|'auth_kind'> & { password:string }
export interface CalendarEvent { id:string; source_id:string|null; origin:'local'|'import'|'remote'; uid:string; summary:string; start:string; end:string; all_day:boolean; timezone:string; location:string; description:string; organizer:string; attendees:string[]; sequence:number; recurrence_id:string; recurring:boolean; method:string; reminder_minutes:number|null }
export interface CalendarSource { id:string; name:string; url:string; username:string; last_sync:number|null }
export interface NewCalendarSource { name:string; url:string; username:string; password:string }
export interface Invitation { index:number; event:CalendarEvent }
export interface CalendarSyncReport { events:number; resources:number }
export interface ReaderSnapshot { mail:Mail; invitations:Invitation[]; labels:string[] }
export interface ImportReport { added:number; skipped:number; invalid:number }
export interface StorageStats { directory:string; database_bytes:number; wal_bytes:number; shm_bytes:number; raw_mail_bytes:number; messages:number; local_messages:number; accounts:number; contacts:number; drafts:number; outbox:number }
export interface CacheReport { messages:number; raw_bytes:number }
export interface SnapshotReport { directory:string; suspended_sends:number }
export interface OAuthRequest { provider:'google'|'microsoft'|'outlook'; client_id:string; desktop_client_secret:string; email:string; name:string; color:string; account_id:string|null }
export interface SearchFilters { sender:string; recipient:string; subject:string; since:number|null; until:number|null; label?:string }
export interface Contact { id:string; email:string; name:string; group_name:string; notes:string }
export interface MailTemplate { id:string; name:string; subject:string; body:string; html:string|null }
export interface SavedSearch { id:string; name:string; account_id:string|null; mode:string; folder:string; query:string; filters:SearchFilters }
export interface MailRule { id:string; name:string; account_id:string|null; enabled:boolean; sender:string; recipient:string; subject:string; body:string; label:string; match_any:boolean }
export interface LabelCount { name:string; total:number; unread:number }
export interface RuleReport { matches:number; newly_labeled:number }
export interface OutboxItem { id:string; account_id:string; recipients:string; subject:string; due_at:number; state:'queued'|'sending'|'accepted'|'uncertain'|'canceled'; detail:string; sent_copy:string; created_at:number; updated_at:number }
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
  addPop: (account:NewAccount) => invoke<Account>('add_pop_account',{account}),
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
  send: (mail:Compose) => invoke<SendReport>('send_mail',{mail}),
  contacts: () => invoke<Contact[]>('list_contacts'),
  saveContact: (contact:Contact) => invoke<Contact>('save_contact',{contact}),
  deleteContact: (id:string) => invoke<void>('delete_contact',{id}),
  templates: () => invoke<MailTemplate[]>('list_templates'),
  saveTemplate: (template:MailTemplate) => invoke<MailTemplate>('save_template',{template}),
  deleteTemplate: (id:string) => invoke<void>('delete_template',{id}),
  searches: () => invoke<SavedSearch[]>('list_saved_searches'),
  saveSearch: (search:SavedSearch) => invoke<SavedSearch>('save_search',{search}),
  deleteSearch: (id:string) => invoke<void>('delete_search',{id}),
  rules: () => invoke<MailRule[]>('list_rules'),
  saveRule: (rule:MailRule) => invoke<MailRule>('save_rule',{rule}),
  deleteRule: (id:string) => invoke<void>('delete_rule',{id}),
  previewRule: (rule:MailRule) => invoke<number>('preview_rule',{rule}),
  applyRule: (id:string) => invoke<RuleReport>('apply_rule',{id}),
  labels: () => invoke<LabelCount[]>('list_labels'),
  messageLabels: (id:number) => invoke<string[]>('message_labels',{id}),
  outbox: () => invoke<OutboxItem[]>('list_outbox'),
  enqueue: (requestId:string,mail:Compose,dueAt:number,draftId:string|null) => invoke<OutboxItem>('enqueue_mail',{requestId,mail,dueAt,draftId}),
  dispatch: (id:string|null=null) => invoke<OutboxItem|null>('dispatch_outbox',{id}),
  cancelOutbox: (id:string) => invoke<void>('cancel_outbox',{id}),
  interruptOutbox: (id:string) => invoke<void>('mark_interrupted_outbox',{id}),
  copyOutbox: (id:string) => invoke<Compose>('copy_outbox',{id}),
  deleteOutbox: (id:string) => invoke<void>('delete_outbox',{id}),
  storageStats: () => invoke<StorageStats>('storage_stats'),
  pendingStorage: () => invoke<string|null>('pending_storage'),
  importMail: (format:'eml'|'mbox') => invoke<ImportReport|null>('import_mail_files',{format}),
  exportEml: (id:number) => invoke<boolean>('export_mail_eml',{id}),
  exportMbox: (accountId:string|null,folder:string|null) => invoke<number|null>('export_mail_mbox',{accountId,folder}),
  deleteLocalMail: (id:number) => invoke<void>('delete_local_mail',{id}),
  importContactsCsv: () => invoke<ImportReport|null>('import_contacts_csv'),
  exportContactsCsv: () => invoke<number|null>('export_contacts_csv'),
  importAccountsJson: () => invoke<ImportReport|null>('import_accounts_json'),
  exportAccountsJson: () => invoke<number|null>('export_accounts_json'),
  cachePreview: (accountId:string|null,before:number|null) => invoke<CacheReport>('cache_preview',{accountId,before}),
  clearCache: (accountId:string|null,before:number|null) => invoke<CacheReport>('clear_cache',{accountId,before}),
  optimizeStorage: () => invoke<void>('optimize_storage'),
  createBackup: () => invoke<SnapshotReport|null>('create_backup'),
  restoreBackup: () => invoke<SnapshotReport|null>('restore_backup'),
  copyStorage: () => invoke<SnapshotReport|null>('copy_storage'),
  calendarEvents: () => invoke<CalendarEvent[]>('list_calendar_events'),
  saveEvent: (event:CalendarEvent) => invoke<CalendarEvent>('save_calendar_event',{event}),
  deleteEvent: (id:string) => invoke<void>('delete_calendar_event',{id}),
  calendarSources: () => invoke<CalendarSource[]>('list_calendar_sources'),
  addCalendarSource: (source:NewCalendarSource) => invoke<CalendarSource>('add_calendar_source',{source}),
  removeCalendarSource: (id:string) => invoke<void>('remove_calendar_source',{id}),
  syncCalendar: (id:string) => invoke<CalendarSyncReport>('sync_calendar',{id}),
  importIcs: () => invoke<ImportReport|null>('import_calendar_ics'),
  exportIcs: (id:string) => invoke<boolean>('export_calendar_ics',{id}),
  invitations: (id:number) => invoke<Invitation[]>('message_invitations',{id}),
  addInvitation: (id:number,index:number) => invoke<ImportReport>('add_invitation',{id,index}),
  invitationReply: (id:number,index:number,accountId:string,partstat:string) => invoke<Compose>('invitation_reply',{id,index,accountId,partstat}),
  meetingInvite: (id:string,accountId:string,recipients:string) => invoke<Compose>('meeting_invitation',{id,accountId,recipients}),
  openComposer: (id:string) => invoke<void>('open_compose_window',{id}),
  closeComposer: () => invoke<void>('close_compose_window'),
  openReader: (id:number) => invoke<void>('open_reader_window',{id}),
  readerSnapshot: () => invoke<ReaderSnapshot>('reader_snapshot'),
  frontendReady: () => invoke<void>('frontend_ready'),
  trayAvailable: () => invoke<boolean>('tray_available'),
  hideMainToTray: () => invoke<void>('hide_main_to_tray'),
  quitDesktop: () => invoke<void>('quit_desktop'),
  notificationEnabled: () => invoke<boolean>('notification_enabled'),
  setNotifications: (enabled:boolean) => invoke<void>('set_notifications',{enabled})
}
