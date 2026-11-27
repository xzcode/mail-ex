# M0 — Architecture

## Layers
Vue 3 + TypeScript/Vite (view and state), Tauri 2 (native IPC + desktop window), independent Rust core (protocols, database and credential vault). The Rust core does not depend on the webview, enabling separate unit/integration tests.

## Storage
App data: mail.sqlite3 (WAL, foreign keys, schema version), vault.key (random per-install 32-byte key; Unix 0600, user app directory permissions). AES-256-GCM encrypts each credential with fresh random 96-bit nonce and account ID associated data. The secret and key are never returned by IPC. This protects against casual plaintext disclosure, not an attacker who can read both key and database; OS-protected key storage and encrypted backup are planned. Mail itself is locally cached unencrypted in this version, so full-disk encryption is recommended.

Account IDs are UUIDs. Each mail row uses account, folder, UIDVALIDITY and UID. Raw RFC822 MIME is retained alongside decoded metadata and safe plain text. Never render untrusted HTML in the app webview. Cache changes apply transactionally only after a successful inbox scan; missing IDs are reconciled on completion. Initial and subsequent scans use bounded network batches, with no cap on total messages. Flags are fetched from UNSEEN search. Later: mailbox discovery, incremental UID checkpoints, QRESYNC/CONDSTORE, IDLE, FTS5, attachment disk streaming, migration framework.

## Protocols
IMAP 2.4.1 and native-tls (implicit TLS, hostname validation, connection/read/write deadlines); Lettre SMTP 0.11 (implicit TLS or required STARTTLS). Mailparse handles MIME and charset decoding. All network operations execute off the UI thread. OAuth is explicitly unsupported in M1; provider adapters will own authentication and token refresh. No insecure fallback.

## IPC
- list_accounts, add_account, remove_account (remove local account/cache only)
- test_account, sync_account
- list_messages (account/unread/query/offset/page size), get_message
- set_seen (UIDVALIDITY verified), send_mail
Secrets accepted only at account creation, never serialized in list responses. Frontend is not a network client. Strict CSP; local assets only. No remote HTML or remote pictures.

## Reliability
Transactions preserve previous cache on failed sync. Account deletion cascades to local cached mail. SMTP result means server accepted the message, not end-recipient delivery; uncertain outcomes must not be automatically retried. Actual provider testing, large-mail streaming, concurrency, error localization and resilient outbox are follow-up items.

## Schema and IPC extension (M2 / M3)
See [M2/M3 notes](M2_M3.md) for schema v2, folder discovery, flags, MOVE, polling and draft/reply IPC. The original sections above describe the M1 baseline; no FTS5 or background service yet.
