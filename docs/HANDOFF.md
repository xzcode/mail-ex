# Handoff — Mail EX v0.1.0 preview published

Repository: xzcode/mail-ex, master. Public pre-release: https://github.com/xzcode/mail-ex/releases/tag/v0.1.0. Release ID 460455442, tag v0.1.0, exact application source 21f26bf23c7ebd3c51c3fae953fd87e2fb382ad3.

Verified assets:
- Linux x86-64 optimized DEB: 8,839,336 bytes, SHA-256 08e5f49b161678f4d70abf61703fdc5642d0437bb6e1a3eaa1d71038385a2731; extracted binary real IPC smoke passed.
- Windows x64 NSIS EXE: 5,314,961 bytes, SHA-256 aac7cc375dc678c608d0b0a4419ef8d96f02c950738928aa00bdee83ccc8ec7d; hosted CI job succeeded, archive digest and PE header verified.
- SHA256SUMS.txt and build-manifest.json uploaded. Release is a preview, not an assertion of production quality. No formal signing certificate.

Next priorities:
1. Run the corrected manual workflow on master, platforms=macos. See https://github.com/xzcode/mail-ex/actions/workflows/desktop-packages.yml. The previous direct dispatch was blocked; do not bypass that restriction. Existing run 45981417804 uses the older DMG verification. ARM64 DMG was created but the job failed after the temporary .app was removed. Workflow-only fix 31d1684 validates the mounted DMG. The Intel job was still running at the last check. Do not publish failed artifacts. Verify the new application tree matches v0.1.0 (excluding docs and workflow), exact workflow revision, SHA-256, and embedded app code signature. Add verified macOS assets to existing preview with updated manifest/checksums only after evidence. Do not create another release with the same tag.
2. Actual Windows/macOS installation and UI interactions, system tray/notifications, attachment dialog, data-location switch/restart, scheduled delivery; approved provider test mailboxes for OAuth/IMAP/SMTP. No real external sending without explicit direction.
3. Remaining functional enhancements documented in ROADMAP: threading, attachment filters, incremental sync, folder management, encrypted backup and CalDAV discovery/write/recurrence. Review imap-proto compatibility warning.

Tokyo workspace: /app/agent/mail-ex-validation, m9-native-work and m9-native-target; m9-release-final.log and m9-release-smoke.log are passing final Linux logs. Staged release files and checksums in m9-release-staging. Existing publisher helper refuses overwriting an existing release; do not rerun it for v0.1.0. All tracked source hashes verified. Final core regression passed 17 unit and 2 loopback TLS tests. Do not touch unrelated projects. See BUILD_EVIDENCE.md.
