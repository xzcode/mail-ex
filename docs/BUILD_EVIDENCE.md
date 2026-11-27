# Build and verification evidence — 2026-11-27

## M1 baseline
- Source commit `9abcd9d59f3c5f40ebf547e9dc49caf44b13c1c1`: Node 24.22.3 / npm 11.19.1, npm install and production build passed on Linux.
- Tauri icon generation succeeded; desktop icon assets and npm lockfile committed.

## M2 / partial M3
- Source commit `39403d8efc6b3436942897814fe7da63d4af5839`: clean Git checkout on Guangzhou Linux host, `npm run build` (vue-tsc and Vite 6.4.6) exit 0; 14 transformed modules, JS 97.32 kB, CSS 8.09 kB.
- Source commit `cb6f4f9b9f1aacf14a2d9bc15fbd095a3a610580`: Tokyo OpenCloudOS 9.6 x86_64 host, rustc/cargo 1.100.0. `cargo test --manifest-path core/Cargo.toml --offline -j 2` exit 0: 4 unit tests passed (credential round trip/tamper, account persistence/deletion, MIME, folder views/drafts).
- `python3 scripts/protocol-fixture.py` exit 0: 1 TLS protocol integration test passed. Temporary CA, loopback-only IMAP and SMTP, account authentication, folder discovery, incremental cache, seen and starred, reply-all and draft, UID MOVE, SMTP delivery and Bcc envelope/header separation.
- System OpenSSL development headers were missing on the Tokyo host. Extracted the matching OpenCloudOS signed `openssl-devel-3.0.12-27.oc9.x86_64.rpm` into the isolated Mail EX build workspace and used OPENSSL_INCLUDE_DIR / OPENSSL_LIB_DIR; no system package installed. Stopped the slower vendored OpenSSL build. Cargo lockfile saved from successful build.
- Dependency warning: imap-proto 0.10.2 future-incompatibility warning; review upgrade before release.

## Not yet verified
Native Tauri desktop build/runtime (GTK/WebKitGTK dependencies not present on the build host), real provider compatibility, large mail performance, certificate rejection fixtures, native notifications, production installers. No actual mailbox connected or external message sent. No Release.

## M3 MIME, attachments and compose — 2026-11-27

- `300b0ea3a35a851dc92fc6cc96a9881bb272460e`: Guangzhou Linux checkout, `npm run build` passed (vue-tsc and Vite 6.4.6; 16 modules, JS 109.22 kB, CSS 11.09 kB).
- Tokyo Linux, Rust 1.100.0, isolated matching system OpenSSL headers: initial MIME attachment unit test exposed a trailing CRLF introduced by automatic transfer encoding. Corrected by explicit Base64 attachment body in `f268d01327b4bd5cab37e8debb3a4cb3dcde1616`. Rebuilt: 6 unit tests passed, including strict HTML sanitization and attachment roundtrip. Existing TLS fixture passed.
- `bdc93cfa9800e472612360f3c6398cd7bf607f75`: extended `python3 scripts/protocol-fixture.py` passed (1 integration test, 2.18 s). Inbound safe HTML, text attachment metadata/preview/exact bytes, forwarded attachment, saved draft reopen, rich multipart SMTP, exact binary payload (including NUL/0xff/line endings), opt-in Sent APPEND, sent-folder sync and absence of Bcc headers in SMTP and stored MIME all verified. Fixture tolerates MIME boundary whitespace in HTML.
- Updated core Cargo.lock with ammonia 4.2.5, html-escape 0.2.15 and serde_json 1.0.154 resolved on the build host.
- Native Tauri compilation/runtime, native save dialog and interactive editor behavior have not been tested; GTK/WebKitGTK environment still missing. No actual provider or external message used. No Release. Old imap-proto future-incompatibility warning remains.

## M4 OAuth / M5 indexed search and batch — 2026-11-27

- `bb50514fb8402d3b7c303b4228cfb04c50cf5a88`: Guangzhou Linux checkout, `npm run build` passed (vue-tsc and Vite 6.4.6; 16 modules, JS 120.23 kB, CSS 12.81 kB).
- Tokyo Linux, Rust 1.100.0 with isolated matching OpenSSL development headers: core compilation and 10 unit tests passed. New cases verify PKCE/state, OAuth callback/cancel, token rotation/SASL construction, FTS5 index and filters. The pre-existing TLS integration fixture also passed.
- `7458e008ce501c26a2bc5769ff7a9170ae95a70c`: updated Cargo.lock committed. `cargo test --manifest-path core/Cargo.toml --offline --locked -j 2`: 10/10 unit tests passed. `python3 scripts/protocol-fixture.py`: 1/1 expanded integration test passed (3.44 s), including duplicate ID handling, mixed success/failure, batch archive, read/unread/star across folders and indexed trash search.
- OAuth unit tests do not establish live Google/Microsoft compatibility. No registered project client IDs, real consent, production access/refresh token exchange or real account were used. Native Tauri desktop authorization/browser launch and runtime remain unverified. Existing imap-proto future-incompatibility warning persists. No release.

## M5 organizer / M6 contacts, templates and outbox — 2026-11-27

- Source `f5c89654fba8118bb68ad39d1c63ec2d466cf72a`: Tokyo Linux, Rust 1.100.0 and previously isolated OpenSSL development headers. `cargo test --manifest-path core/Cargo.toml --offline --locked -j 2` passed (12/12 unit tests); existing TLS protocol fixture passed.
- Source `a12dbb318c56ea7310cd20eb4f2ec20e3f47c9b9`: locked core build and all 12 unit tests passed. Added schema v6 coverage verifies local rule AND matching and account scope, auto classification only for new inbox records, preview/explicit additive application, category filtering, saved searches, contacts, sanitized templates, durable queue/draft transfer, idempotent enqueue, cancel, pending-account protection and interrupted claims across Store reopen.
- Expanded `python3 scripts/protocol-fixture.py` passed (1/1, 3.40s). Actual loopback TLS SMTP delivery through the durable queue, Sent APPEND, replay of accepted request, no duplicate delivery and single rejection with no automatic retry verified. MIME/Bcc/attachment/IMAP flags and MOVE regression still passed.
- Guangzhou Linux: Git fetch timed out. Created separate frontend validation directory from existing `bb50514fb8402d3b7c303b4228cfb04c50cf5a88` frontend baseline (unchanged dependency lockfile/config), copied all five changed frontend source files from `a12dbb3` and verified each with its Git blob hash. Reused the existing npm dependencies. `npm run build` passed: vue-tsc and Vite 6.4.6, 21 transformed modules, JS 148.30 kB, CSS 16.32 kB.
- No external mailbox contacted and no production mail sent. Tauri native scheduler, interactive UI, native notifications and installers are not verified; native notification feature remains pending. No real OAuth consent/refresh integration. Existing imap-proto future-incompatibility warning remains.

## M7 local data portability, backup and storage — 2026-11-27

- Source `ffbc953be9fa3061627588464c1d2ffcf38d8a22`: Tokyo Linux, Rust 1.100.0, previously isolated matching OpenSSL headers; core compilation succeeded, 14/14 unit tests passed. New portability tests verify EML/MBOX archive, deduplication, CSV contact exchange, account configuration import/export without credentials, cache exclusion of local imports, consistent SQLite backup/restore, checksum rejection and suspension of restored queued/sending delivery.
- `python3 scripts/protocol-fixture.py`: 1/1 loopback TLS integration test passed (3.20s), including the durable outbox and existing mail protocol regressions. No external mail sent.
- Guangzhou Linux: reused the M6 hash-verified frontend baseline and unchanged npm dependencies, copied all four M7 frontend source changes from the commit, verified Git blob hashes, and ran `npm run build` successfully (vue-tsc and Vite 6.4.6; 23 modules; JS 162.66 kB, CSS 17.61 kB).
- Updated core Cargo.lock with csv 1.4.0 and csv-core 0.1.13 from this build.
- Native desktop build/runtime, file dialogs and restart-based data directory activation not verified. Restore is same-schema only. Backup is unencrypted, contains database and credential key, and must be treated as private trusted data. Security audit and real provider acceptance remain pending; no Release.

## M8 POP3, calendar, invitations and reminders — 2026-11-27

- Source `124bf504260f2890ab79750e1cb817780f75b045`: frontend production build passed; initial core compilation exposed an ambiguous Vec type in the iCalendar parser. Fixed in `4f3e35c581b3ef268f38ae99df54117577003d55`.
- Tokyo Linux, Rust 1.100.0, existing isolated matching OpenSSL headers, `cargo test --manifest-path core/Cargo.toml --offline -j 2`: 17/17 core unit tests passed at `4f3e35c`. New cases cover ICS parsing/import/export and local calendar CRUD, XML multistatus/invalid report handling, invitation MIME recognition, opt-in reminder claims and duplicate prevention after reopening the database.
- `python3 scripts/protocol-fixture.py`: 2/2 loopback TLS integration tests passed (3.19 s). POP3 USER/PASS/UIDL/RETR, dot unescaping, single download across repeated synchronization, retention when server no longer lists mail, no DELE, local flags/local Sent copy, cache exclusion, invitation import/deduplication, REQUEST and three REPLY drafts, authenticated HTTPS CalDAV REPORT, cache preservation on incomplete response, and existing IMAP/SMTP/MIME/outbox regression verified. No actual provider mailbox or external message was used.
- Guangzhou Linux: copied all five changed frontend source files into an isolated M8 validation directory based on the M7 hash-verified baseline and verified each Git blob hash. Final CalendarPanel patch hash also verified. Existing npm dependencies unchanged. `npm run build` passed (vue-tsc + Vite 6.4.6, 27 modules, JS 183.46 kB, CSS 20.66 kB).
- Core lockfile updated with roxmltree 0.20.0 and the already resolved chrono dependency. Schema v8 extends credential verification to CalDAV sources and protects POP3 cache from generic cache cleanup.
- Native Tauri build/runtime, system notifications, native reminder worker, live CalDAV/provider interoperability, actual iMIP delivery and installers remain unverified. CalDAV is read-only, recurrence not expanded, named timezone wall time preserved. Existing imap-proto future-incompatibility warning remains.
