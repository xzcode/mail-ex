# Roadmap and status

- M0: product specification, design, architecture and project setup — baseline written.
- M1: accounts, secure IMAP/SMTP, manual sync, SQLite cache, desktop three-pane UI, plain text compose, read/unread — implementation baseline, build/live validation pending.
- M2: complete folders, unified views, incremental sync checkpoints, per-account scheduler/IDLE, reconnect, flags/move/archive/trash, virtual list and offline support.
- M3: safe HTML, rich editor, attachment streaming/download, drafts, signatures, reply/forward and independent windows.
- M4: OAuth PKCE, Gmail and Microsoft Graph/IMAP OAuth, token refresh and provider tests.
- M5: FTS5 index, advanced search, rules, grouping and batch operations.
- M6: contacts, templates, outbox, scheduled send, notifications.
- M7: import/export, backup/restore, security audit, storage settings/cleanup.
- M8: CalDAV, invitations, POP3 and additional provider adapters.
- M9: polish, accessibility, themes and languages, OS integrations, signed/unsigned packages and releases.
- M10: optional self-hosted sync, mobile and opt-in AI.

## Acceptance evidence
Build, test and protocol verification results must be recorded, not assumed. No release until usable binaries and the milestone's real integration criteria are verified.
