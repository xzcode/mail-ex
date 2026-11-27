# Handoff — M4 OAuth and partial M5

Repository: xzcode/mail-ex, master. M0 design, M1/M2 baseline, most M3 source, M4 OAuth implementation, partial M5 indexed search and batch. Latest validation: frontend production build; Rust core locked build, 10 unit tests, 1 expanded loopback TLS integration test passed. See BUILD_EVIDENCE.md, M4_M5.md and M3_MIME.md.

Priorities:
1. M5 finish saved searches, rule configuration/execution, attachment filters and threading/grouped views; batch connection reuse and cancellation. All server-side mutations must remain UIDVALIDITY-aware and server-confirmed.
2. M6 contacts, reusable templates, durable local outbox/scheduled send, delivery uncertainty/retry safeguards and notifications.
3. M4 live test using approved Google and Microsoft desktop client registrations and test mailboxes; verify actual browser callback, token exchange/refresh, XOAUTH2 IMAP/SMTP, SMTP policy failures. No production client IDs or credentials included. Do not borrow third-party app IDs or embed confidential secrets.
4. Native Tauri build and interactive acceptance on supported systems; independent native read/compose windows, native attachment dialog, large attachment streaming. Desktop lockfile and release CI once verified.
5. Sync improvements: UID checkpoints/IDLE, cancellation, per-account scheduler, native notifications, safe folder CRUD. No blanket EXPUNGE. Review imap-proto warning.

Reproduce: npm ci && npm run build; cargo test --manifest-path core/Cargo.toml --locked; on Linux with openssl and Python 3, python3 scripts/protocol-fixture.py (loopback only).

Limitations: OAuth requires user-provided registered desktop client IDs; native OAuth and live providers not tested. Refresh serialization is process-local. FTS5 is a local index; short terms use substring matching. Batch uses sequential individual operations. Local messages/drafts and attachments are not encrypted; credentials use adjacent per-install key. Strict HTML strips all attributes/images/links; no streaming. Polling while UI runs. MOVE required for moving messages. No production installer or Release.
