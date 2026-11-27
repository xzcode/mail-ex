# Roadmap and status

- M0: product specification, design, architecture and project setup — baseline written.
- M1: accounts, TLS IMAP/SMTP, manual sync, SQLite cache, three-pane UI, plain-text compose, read/unread — implementation baseline; frontend build passed, core/desktop and protocol acceptance pending.
- M2: folder discovery, per-folder and all-folder sync, unified inbox/starred/all, flags, UID MOVE, opt-in polling and account editing implemented in source. Remaining: protocol fixtures/live verification, incremental checkpoint/IDLE, cancellable jobs, virtual list, native notification, non-MOVE safe strategy and folder CRUD.
- M3: local multi-draft store, reply/reply all/forward and reply headers implemented in source. Remaining: safe HTML, rich editor, attachment streaming/download and forwarding, signatures, independent windows, sent-folder APPEND.
- M4: OAuth PKCE, Gmail and Microsoft Graph/IMAP OAuth, token refresh and provider tests.
- M5: FTS5 index, advanced search, rules, grouping and batch operations.
- M6: contacts, templates, outbox, scheduled send, notifications.
- M7: import/export, backup/restore, security audit, storage settings/cleanup.
- M8: CalDAV, invitations, POP3 and additional provider adapters.
- M9: polish, accessibility, themes and languages, OS integrations, packages and releases.
- M10: optional self-hosted sync, mobile and opt-in AI.

See [M2/M3 notes](M2_M3.md) and [build evidence](BUILD_EVIDENCE.md). Update acceptance evidence only after actual tests.
