# Handoff — M3 MIME and compose

Repository: xzcode/mail-ex, master. M0 design established; M1/M2 baseline and most M3 source complete. Latest validation: frontend production build, Rust core build, 6 unit tests and 1 expanded loopback TLS integration test passed. See BUILD_EVIDENCE.md, M3_MIME.md. Updated core Cargo.lock and npm lockfile committed.

Priorities:
1. Native Tauri build with GTK/WebKitGTK dependencies or supported macOS/Windows runner. Verify native Save As, interactive rich editor, autosave and actual desktop launch. Add desktop Cargo.lock when resolved. Independent read/compose windows and attachment streaming still pending.
2. M4 OAuth: provider registration, loopback authorization code + PKCE/state, token exchange/refresh, encrypted token storage, XOAUTH2 IMAP/SMTP and/or Graph adapter. Google/Microsoft app registration and provider policies required for live testing; never embed a confidential client secret or prompt for account passwords in OAuth mode. No existing OAuth implementation.
3. Improve synchronization: UID checkpoints/IDLE, cancellation, per-account scheduler, native notifications, safe folder CRUD. No blanket EXPUNGE.
4. FTS5, advanced filters, rules and batch operations.
5. Cross-platform CI and release packaging. Review imap-proto warning.

Reproduce: npm ci && npm run build; cargo test --manifest-path core/Cargo.toml --locked; on Linux with openssl and Python 3, python3 scripts/protocol-fixture.py (loopback only).

Limitations: credentials encrypted with adjacent per-install key; messages/drafts and attachment payload not encrypted. Strict tag allowlist strips all HTML attributes, images and links, not full fidelity. Attachments buffered in memory/base64 IPC; no artificial size/count caps. Polling only while UI runs. IMAP MOVE capability required. Sent copy optional, off by default. No actual provider/native desktop runtime acceptance, no installer.
