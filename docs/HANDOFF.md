# Handoff — initial development

Repository: xzcode/mail-ex, master. M0 documents and M1 implementation baseline are included. No production package or live account verification yet.

Priorities:
1. Build frontend and Rust core, then native Tauri desktop on target systems; fix compiler and dependency issues. Generate lockfiles and icon assets.
2. Run local TLS IMAP/SMTP fixture tests (certificate validation, auth failures, Unicode MIME, UIDVALIDITY rollover, read-state verification, SMTP rejection/uncertain outcome). Never send actual email as a test without explicit user direction.
3. Add account edit/test-before-save, cancellation/progress, secure HTML and attachments, folder discovery and robust incremental sync.
4. Add OAuth separately with proper app registrations/PKCE and provider policies.
5. Add cross-platform CI and release packaging; record evidence in roadmap.

Initial security limitations: credential key is adjacent to database; email content itself is not encrypted; no secure HTML rendering; no automatic send retry. Initial IMAP sync is an atomic inbox scan, not a full folder/IDLE sync. Search uses SQLite substring matching, not FTS yet.
