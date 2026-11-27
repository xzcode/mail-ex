# Handoff

Repository: xzcode/mail-ex, master. M0 design, M1 implementation and M2/M3 baseline are in source. See M2_M3.md. Frontend production build passed on previous baseline; current revision requires rerun. Rust toolchain located on Tokyo host but baseline compile blocked by missing OpenSSL development package. Do not use real accounts or release until verification.

Priorities:
1. Build core and run unit tests on Linux; validate latest frontend. Add Cargo lockfiles after successful resolution, verify Tauri desktop on a supported build host.
2. Run local TLS IMAP/SMTP fixture tests: cert validation, auth failures, modified UTF-7 and MIME, folder discovery, UIDVALIDITY rollover, flag confirmation, MOVE and uncertain send. No blanket EXPUNGE.
3. Finish safe HTML preview, MIME attachment listing/download/upload and rich editor, drafts autosave, sent-folder APPEND, cancellation and status.
4. OAuth PKCE and provider-specific app registration / token refresh.
5. Cross-platform CI and release packaging. No fake provider claims or live emails sent as tests.

Limitations: credentials encrypted with adjacent per-install key; local messages/drafts not encrypted. No full-disk access or remote HTML execution. Automatic sync is opt-in polling only while UI runs, not a durable background service.
