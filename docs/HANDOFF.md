# Handoff — M9 native desktop baseline

Repository: xzcode/mail-ex, master. M0–M8 feature baselines, M9 native Linux compilation and three-window smoke verified, debug Linux amd64 DEB generated. See BUILD_EVIDENCE.md, M9.md and ROADMAP.md. No production Release published.

Verified package on Tokyo host: `/app/agent/mail-ex-validation/m9-native-target/debug/bundle/deb/Mail EX_0.1.0_amd64.deb` (15,728,160 bytes), SHA-256 `c82a92d0262adc9d41ff45f976a6d498c9d472927389f1059eec85cef4ec6c3b`. It is a debug candidate, not an optimized production build; not installed on a clean user OS. Native code source 57a7986, smoke harness e77a657, bundle metadata a8b747b.

Priorities:
1. Build production release-profile packages on Linux and target-platform macOS/Windows. Manual workflow `.github/workflows/desktop-packages.yml` exists, not run. Use native target runner architecture, no fabricated packages. Verify installers, signatures/ad-hoc as applicable and real interactive flows; publish first release only with verified artifacts and clear limitations.
2. Complete native UI interaction: actual compose edits/save/send through local fixtures, independent reader (read-only by design), dialogs, setting persistence, storage-directory switch/restart, scheduling, native notifications/tray. Native startup/IPC only is proven, not every action.
3. Complete remaining M2/M5/M7/M8 gaps: threading, attachment filters, incremental sync/IDLE, CalDAV write/discovery/recurrence and timezone conversion, encrypted backup/security, real provider OAuth/IMAP/SMTP acceptance. Do not use real external mail without explicit user direction.
4. Improve accessibility, theme and localization beyond English/Chinese, layout and performance. Review imap-proto future-incompatibility warning.

Build workspace: Tokyo `/app/agent/mail-ex-validation/m9-native-work`, native target `m9-native-target`, Docker image `mail-ex-native-build:bookworm-m9`, cached toolchain path `/app/agent/my-tools-m6-20261127/rustup/toolchains/stable-x86_64-unknown-linux-gnu` mounted read-only. Mount only project source/cargo/target. Logs: `m9-native-build.log`, `m9-final-build.log`, `m9-smoke.log`, `m9-deb-final.log`. Do not touch other projects. Both core and desktop Cargo.lock are committed. Core M8 tests: 17 unit, 2 TLS fixtures passed; no core source change in M9.

Limitations: Linux headless startup verified; no real provider or native notification display validation, no Windows/macOS builds and no production release. Detached reader is an immutable cached snapshot; attachments/download/reply/mutations in main. No arbitrary mailbox or window quota. Settings language system/English/Chinese, themes system/light/soft/gray/dark. Backups and mail/calendar data not encrypted as a whole; credentials encrypted with adjacent key. CalDAV read-only; no full recurrence/timezone expansion.
