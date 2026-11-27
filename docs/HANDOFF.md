# Handoff — M8 POP3 and calendar

Repository: xzcode/mail-ex, master. M0 design and M1–M8 functionality baselines implemented. Latest verification: 17 Rust unit tests, 2 loopback TLS integration tests, frontend production build passed. See BUILD_EVIDENCE.md, M8.md and ROADMAP.md.

Priorities:
1. M9: unblock actual Tauri native compilation and launch on supported OS. Resolve/lock desktop dependencies, validate real UI (mail list, rich editor, attachment dialogs, OAuth browser, scheduler, calendar, notification permissions, data-directory restart), polish layout and accessibility, build OS packages and publish only verified artifacts. Use test mailboxes and approved provider client registrations; no real external test mail without explicit request.
2. Finish native independent read/compose windows, storage/security audit and passphrase-protected backup; avoid automatically requeueing uncertain historical sends after restore.
3. M8 remaining: CalDAV discovery, conditional remote writes and incremental sync, OAuth CalDAV, recurrence/timezone expansion, meeting lifecycle and actual provider iMIP compatibility. The current CalDAV source is explicit HTTPS Basic/app-password read-only sync; ICS imported/remote events are read-only. Meeting responses are drafts, not automatically sent.
4. M5 threading, attachment filters, local classification continuity, batch connection reuse/cancellation; M2 incremental sync/IDLE, safe folder operations and cancellation. Review old imap-proto dependency.
5. Packaging and release workflow after native build; do not claim platform packages or provider compatibility without evidence.

Reproduce: npm ci && npm run build; cargo test --manifest-path core/Cargo.toml --locked; python3 scripts/protocol-fixture.py (local TLS fixture using openssl, Python 3 and Rust).

Limitations: no native runtime or real provider acceptance yet, no production installer or release. POP3 uses implicit TLS and stable UIDL; never deletes server mail, only local flags/Sent, and excluded from generic cache cleanup. CalDAV credentials are encrypted with the same per-install vault key; actual mail, calendar data and backups are not encrypted as a whole. Backups include the credential key; restore only trusted same-schema data. Reminder claims are once per event sequence while app runs; OS notification permissions/display remain untested. No remote writes by CalDAV, full recurrence expansion or named timezone conversion. Meeting invitations/replies use text/calendar attachments; provider-native interoperability pending.
