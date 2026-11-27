# M0 — Product specification

## Goals and scope
A desktop client aggregating independent IMAP/SMTP and provider-API accounts, with local data, unified inbox/search, and per-account management. No Mail EX login, backend or mandatory cloud sync. Open-source desktop applications for Windows, macOS and Linux.

## UX
Three panes: navigation/account tree, message list, reading pane. A unified inbox and unread view aggregate by account; every message retains its originating account. Compose is separate from the read pane; account identity must be explicit. Resizable panes, keyboard navigation, independent mail windows later. Contextual actions use server-confirmed status. Empty, loading, offline and error states are genuine, not fabricated.

## Functional requirements
1. Unlimited user-configured accounts, groups and colors, provider presets and custom settings. Password/auth code and OAuth adapters; never imply password access works where disabled.
2. Sync incrementally by account and mailbox. Read/unread, flag, move, archive and delete converge to server state; remote destructive operations explicit.
3. New message, reply/reply all, forward, CC/BCC, attachments, signatures, drafts, outbox, scheduled send (when app runs).
4. Safe MIME and HTML rendering, attachment preview/export, remote images disabled by default. Plain text accessible without HTML.
5. Full-text and advanced local search, per-account filtering, contacts, rules and templates.
6. User-configurable data location, retention, cleanup, import/export (EML/MBOX), backup/restore and recovery.
7. Multi-language, system/light/dark themes, keyboard shortcuts, OS notifications and independent message windows.
8. Optional CalDAV and invitations, later OAuth/Graph/Gmail APIs, POP3 and Bridge support.

## First milestone acceptance
A user can add a custom TLS account, reopen the application without entering the secret again, test connectivity, manually sync inbox messages and browse the cache offline, use the unified inbox, mark a message read/unread, compose plain text with CC/BCC and send over SMTP. Offline cached data is never erased on a failed synchronization. No credentials exposed in UI list or logs. Clear unsupported OAuth, attachments and HTML-only limitations.

## Provider matrix (planned)
- QQ/163/126/iCloud/custom: password or app authorization code if provider permits; actual service verification required.
- Gmail: app password where enabled; OAuth PKCE and provider policy compliance planned.
- Outlook/Microsoft 365: OAuth/Graph planned; basic auth must not be advertised.
- Zoho/Yahoo/Proton Bridge: provider and plan dependent. Custom TLS configuration only in first milestone.

## Nonfunctional requirements
No artificial resource quotas. Batch/paginate and expose cleanup; handle disk-full and network errors. Strict certificate checks, secure credential handling, no remote content execution, local-only data by default. No unattended sending or remote deletion without explicit UI action.
