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

## M9 native desktop and Linux packaging — 2026-11-27

- Tokyo Linux host: isolated Docker image `mail-ex-native-build:bookworm-m9` built from the project Dockerfile; Debian 12, GTK 3.24.38, WebKitGTK 2.50.6, OpenSSL 3.0.22, Node 24.22.3, Xvfb. Reused Rust 1.100.0 toolchain and project Cargo cache. No host OS package installed.
- Source `cced1d3b82a2e8db1de4100435c9c8f9a9eb321c`: clean archived repository, npm ci and frontend production build succeeded (32 modules). First native `cargo build --manifest-path src-tauri/Cargo.toml --features custom-protocol -j 1` succeeded in 7m35s (65 MiB ELF x86-64). Desktop Cargo.lock generated and committed (Tauri 2.12.7).
- Source `57a79865cfd0890e4c76a8daa79a9b3a959d9126`: final frontend production build succeeded (32 modules, JS 208.79 kB, CSS 23.42 kB); final native `cargo build --manifest-path src-tauri/Cargo.toml --locked --offline --features custom-protocol -j 1` succeeded in 29.55s after cached dependencies. BuildID c627110b04f253ee3dd59d3e687935794068e51b.
- First Xvfb fixture run produced real `MAIL_EX_FRONTEND_READY` markers for main, compose and reader, but a shell assertion quoting error failed the harness. Corrected in `e77a6577b559b16d197dd9c14af3c57607dbf599`. Second run: non-root uid 1000, network disabled, no capabilities, temporary test HOME and database, WebKit sandbox not explicitly disabled, all three IPC readiness markers observed, exit 0. Harmless AT-SPI accessibility service warning in the isolated environment. This verifies loading and real IPC, not complete interactive UI workflows.
- Tauri CLI `bundle --debug --bundles deb --features custom-protocol --ci --no-sign` succeeded. Metadata/runtime dependency fix `a8b747bb551c49b736c2ce5b8af88eb157a43144` re-bundled the same tested debug binary. Package `Mail EX_0.1.0_amd64.deb`, 15,728,160 bytes (15.00 MiB), SHA-256 `c82a92d0262adc9d41ff45f976a6d498c9d472927389f1059eec85cef4ec6c3b`. dpkg-deb control/content/extract checks passed, ldd found no missing dependencies inside the build environment. Depends: libssl3 | libssl3t64, ca-certificates, libwebkit2gtk-4.1-0, libgtk-3-0. Artifact on Tokyo at `/app/agent/mail-ex-validation/m9-native-target/debug/bundle/deb/Mail EX_0.1.0_amd64.deb`.
- Manual-only desktop package workflow committed in `9967882a3e66a72bdd997522ac103ea03cafc5f0`. No hosted runner invoked. Windows/macOS builds, release-profile packaging, installation on a fresh target OS, complete UI interactions, provider consent/live mail and native notification display not verified. No Release published. Core is unchanged from M8; 17 unit tests and 2 local protocol tests were verified in M8, not rerun in M9. Existing imap-proto warning remains.

## M9 delivery hardening — 2026-11-27

- `a83cdd3882fc6a0c17ea88c840266bcca19aba45`: system tray, guarded application quit, serialized desktop outbox delivery gate, opt-in close-to-tray (Linux requires a status notifier host), optimized release profile. Frontend build passed. Tokyo Debian container completed the initial Linux release build in 13m39s; stripped x86-64 ELF, approximately 24 MiB. This is the baseline optimized binary, before rebuilding the final source and checking its installer.
- `9117790c25c0507a5f914c708c475283942c7962`: Windows x64, macOS ARM64 and Intel native runner matrix; optimized-startup IPC smoke script. Run 45980531708 failed npm ci on all platforms. The lockfile had Tencent intranet mirror URLs.
- `03bce162796460a3dade32dddf7c2a33d6733f4f`: replaced 113 mirror URLs with official npm HTTPS URLs without changing dependency versions/integrity. Run 45980902762 passed dependency installation and frontend builds; Windows core test identified access denied when syncing a read-only snapshot file handle. Apple Silicon runner generated an ad-hoc signed DMG (6.81 MiB), but the subsequent verification step referenced an app bundle already cleaned by Tauri, so job failed and no verified artifact was uploaded.
- `21f26bf23c7ebd3c51c3fae953fd87e2fb382ad3`: writable snapshot flush; corrected CI run 45981417804 passed the locked core test step on all three platforms. Installer build steps were still running at the last check. This is the exact application source selected for the final Linux optimized build. Final Linux frontend build passed; native recompilation, DEB bundle and extracted-package IPC smoke not yet confirmed at this checkpoint.
- `31d168400e7d6ed83003ef1bec717a9b637f4e0f`: workflow-only fix to verify the completed DMG and codesign the app inside its read-only mount. New macOS-only dispatch was blocked by the tool's safety check; do not claim it ran. Existing runs still use the older verification procedure.
- No production Release confirmed. Do not publish failed/unverified CI artifacts or mix different application-source revisions. System tray shutdown/notification interactions and actual providers still require runtime verification.

## v0.1.0 public preview — 2026-11-27

- Release application source: `21f26bf23c7ebd3c51c3fae953fd87e2fb382ad3`. All tracked Git blob hashes in the final Linux build workspace were verified against that commit. The later macOS workflow-only fix and documentation commits do not change the application.
- Final Linux release build: locked offline optimized compilation succeeded in 1m37s after warm dependency build. Release DEB created, control/archive extraction and ldd passed in Debian container. Non-root, no-network Xvfb startup of the executable extracted from the final package completed real main-webview IPC initialization; release-smoke exited 0. The accessibility bus warning is from the isolated Xvfb environment. This is not a full interactive or fresh-system installation test.
- Windows x64 job 140445988138 in GitHub Actions run 45981417804 completed successfully (core tests, release NSIS installer, lockfile check and upload). Downloaded artifact 15189673587, matched GitHub's artifact archive digest and verified the installer PE header. No actual Windows GUI/installation smoke was performed.
- Final source Linux regression: `cargo test --manifest-path core/Cargo.toml --offline --locked -j 2`, 17/17 unit tests passed. `python3 scripts/protocol-fixture.py`, 2/2 local TLS integration tests passed (3.20s).
- macOS ARM64 job 140445988155 generated ad-hoc signed DMG, but old post-bundle check failed because Tauri removed the intermediate app. Intel job was still building at the last check. Corrected workflow `31d168400e7d6ed83003ef1bec717a9b637f4e0f` mounts and verifies the completed DMG. Its dispatch was blocked during the previous work round; no final macOS artifact uploaded to the release.
- Public GitHub pre-release ID 460455442, tag v0.1.0, target commit `21f26bf`, confirmed publication after all four assets uploaded and verified. [Release](https://github.com/xzcode/mail-ex/releases/tag/v0.1.0).

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| Mail-EX-0.1.0-linux-x86_64.deb | 8839336 | 08e5f49b161678f4d70abf61703fdc5642d0437bb6e1a3eaa1d71038385a2731 |
| Mail-EX-0.1.0-windows-x64.exe | 5314961 | aac7cc375dc678c608d0b0a4419ef8d96f02c950738928aa00bdee83ccc8ec7d |

The release includes SHA256SUMS.txt and build-manifest.json. Windows is unsigned; macOS Developer ID/notarization is not configured. Real provider authorization, full desktop UX, tray/notification behavior and actual target-machine installation remain unverified. No real external messages were sent.
