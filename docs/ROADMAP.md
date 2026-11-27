# Roadmap and status

- M0: product specification, design, architecture and project setup — baseline established.
- M1: multi-account, TLS IMAP/SMTP, manual sync, SQLite, unified inbox, desktop UI, plain text compose, read/unread — core and frontend build passed, loopback protocol verification passed; native desktop and actual providers still pending.
- M2: folder discovery, per-folder/all-folder sync, unified inbox/starred/all, flags, UID MOVE, opt-in polling and account editing — implemented; covered core functions passed fixture. Remaining: real provider acceptance, incremental checkpoints/IDLE, cancellable jobs, virtual list, native notification, non-MOVE safe strategy and folder CRUD.
- M3: strict safe HTML/plain text reading, attachment metadata/preview/save/upload/forward, rich text editing, per-account signature, local drafts with payload and autosave, reply/forward, opt-in server Sent APPEND — source implemented; frontend build, MIME unit tests, extended TLS protocol fixture passed. Remaining: native save dialog and interactive editor acceptance, independent native windows, streaming/large attachment optimization, inline CID images, richer HTML formatting and PDF preview.
- M4: Google, Outlook.com and Microsoft 365 desktop OAuth authorization code/PKCE, local callback/state/cancel, encrypted credentials, pre-expiry refresh, XOAUTH2 IMAP/SMTP and reauthorization — implemented; core unit tests passed. Live provider registration/consent, refresh integration and native browser/runtime acceptance pending; Graph adapter optional.
- M5: FTS5 trigram local index (schema v5), short-query fallback, sender/recipient/subject/date filters, multi-select, batch read/unread/star/unstar/move/archive/trash and per-message results — implemented; index tests and batch TLS fixture passed. Remaining: saved searches, rules, attachment filters, grouping/threading, batch connection reuse/cancel.
- M6: contacts, templates, outbox, scheduled send, notifications.
- M7: import/export, backup/restore, security audit, storage settings/cleanup.
- M8: CalDAV, invitations, POP3 and additional provider adapters.
- M9: polish, accessibility, themes and languages, OS integrations, packages and releases.
- M10: optional self-hosted sync, mobile and opt-in AI.

See [M2/M3 notes](M2_M3.md), [M3 MIME notes](M3_MIME.md) [M4/M5 notes](M4_M5.md) and [build evidence](BUILD_EVIDENCE.md). Never equate loopback verification with provider or native desktop acceptance.
