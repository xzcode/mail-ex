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
