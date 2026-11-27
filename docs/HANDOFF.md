# Handoff — M5 organizer and M6 durable outbox

Repository: xzcode/mail-ex, master. M0 design, M1/M2 baseline, most M3 source, M4 OAuth, M5 search/batch/rules/saved searches, M6 contacts/templates/outbox. Latest validation: frontend production build (all changed frontend files verified by Git blob hash), locked Rust core build, 12 unit tests and 1 expanded loopback TLS protocol fixture passed. See BUILD_EVIDENCE.md, M5_M6.md, M4_M5.md and M3_MIME.md. No new dependencies in this round.

Priorities:
1. Finish M5 thread/conversation grouping, attachment filters, local category continuity across folder moves, richer explicit rule actions, batch connection reuse/cancel.
2. M6 native desktop scheduler acceptance, OS notifications, contact import/export and scheduling UX; ensure uncertain or interrupted sends are never automatically requeued. Queue transitions use stable request IDs and atomic claims. Rules currently only apply local categories.
3. M7 backup/restore, EML/MBOX and contact import/export, cache maintenance/storage settings, security audit. Maintain schema migrations and ensure pending outbox records cannot be duplicated by restore.
4. M4 live test using approved Google and Microsoft desktop client registrations and test mailboxes. No production client IDs or credentials included. Never borrow third-party app IDs or embed confidential secrets.
5. Native Tauri build and interactive acceptance on supported systems; independent windows, native attachment dialog, large attachment streaming. Add desktop Cargo.lock and release CI after verified build.
6. Sync improvements (UID checkpoints/IDLE, cancellation, per-account scheduling, safe folder CRUD). No blanket EXPUNGE. Review imap-proto warning.

Reproduce: npm ci && npm run build; cargo test --manifest-path core/Cargo.toml --locked; on Linux with openssl and Python 3, python3 scripts/protocol-fixture.py (loopback only).

Limitations: live providers and native desktop runtime remain unverified. Local messages/drafts/outbox/attachments are not encrypted; credentials use adjacent per-install key. Rule categories are local and tied to cached entries, automatic rules never mutate server mail. Scheduled delivery is application-process only; overdue queued mail becomes eligible on next launch. Interrupted claims remain uncertain and never auto retry. No exact-time/recipient-delivery guarantee. Batch sequential; strict sanitized HTML; no large attachment streaming. No production installer or Release.
