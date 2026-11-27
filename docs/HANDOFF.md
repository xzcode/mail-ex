# Handoff — M7 data portability and storage

Repository: xzcode/mail-ex, master. M0 design, M1–M6 functionality baselines and M7 archive/backup/data maintenance implemented. Latest verification: 14 Rust unit tests, 1 loopback TLS protocol fixture and frontend production build passed. See BUILD_EVIDENCE.md, M7.md and ROADMAP.md.

Priorities:
1. Complete native desktop build and interaction acceptance, especially dialogs and restart-based storage activation, independent compose/read windows, scheduling and notifications. Verify actual provider OAuth/IMAP/SMTP using approved test accounts.
2. M8 implement CalDAV calendar, iCalendar invitations, POP3 and further provider/enterprise adapters; design and test locally without using real credentials.
3. M7 remaining: optional passphrase-encrypted backup, migration-aware backward-schema restoration, streaming large messages, security audit. Keep restore non-destructive and never automatically resend historical/uncertain queued mail. Backups include the key; keep them private and trusted.
4. M5 conversation threading, attachment filters, label continuity and batch connection reuse/cancellation; M2 incremental checkpoints/IDLE, safe folder management, sync cancellation.
5. M9 polish, accessibility, desktop package verification and releases. Review imap-proto future-incompatibility warning.

Reproduce: npm ci && npm run build; cargo test --manifest-path core/Cargo.toml --locked; on Linux with openssl and Python 3, python3 scripts/protocol-fixture.py.

Limitations: native desktop/real providers not yet verified; no release. Local database and backup are not encrypted, credentials are encrypted with an adjacent per-install key. Exported account JSON contains no credentials. Backups include database and key, and checksum is not authentication. Restore accepts same schema and copies into a new directory; original remains untouched. Restored queued/sending rows are suspended, not retried; active source queue is unchanged. Changes after a copy are not transferred, and new location activates on restart. Cache cleanup excludes local imports; cleared server messages can reappear after sync. MBOX may normalize final newline, EML retains original bytes. HTML strict sanitized rendering, no large attachment streaming.
