# Handoff — M9 release hardening

Repository: xzcode/mail-ex, master. See BUILD_EVIDENCE.md for exact commits and verified status.

- Application source selected for release candidate: 21f26bf23c7ebd3c51c3fae953fd87e2fb382ad3. 31d1684 changes macOS verification workflow only. Latest documentation commits do not change app code.
- Tokyo /app/agent/mail-ex-validation: m9-native-work, m9-native-target, m9-release-source, m9-release-build.log, m9-release-final.log, m9-release-smoke.log. Docker image mail-ex-native-build:bookworm-m9. Initial optimized native build succeeded; final build/bundle/smoke pipeline launched, completion not confirmed at this checkpoint. Check logs and exact source hashes; the temporary .source-revision marker may still contain the earlier commit, so verify all tracked blob hashes before publication.
- GitHub run 45981417804 is the corrected application-source run. All three core tests steps passed. Check jobs and uploaded artifacts rather than assuming completion. Previous Apple Silicon DMG build succeeded but the verification failed because the intermediate .app had been cleaned. 31d1684 fixes workflow by verifying mounted DMG. Mac-only dispatch was blocked; no corrected run confirmed.
- Staging folder m9-release-staging has local collector and publisher helpers; no release created at this checkpoint. These are not project code. Review and verify source, logs, exact assets and permissions before invoking. Do not overwrite a pre-existing release. If a workflow-only follow-up changes the build SHA, prove application code equivalence and record exact workflow SHA. Do not publish mixed app revisions or unverified assets.
- Prior verified baseline: M8 17 unit/2 loopback TLS tests; M9 Linux debug three-window IPC smoke and debug DEB.
- Finish release-profile Linux startup/package, Windows NSIS, macOS ad-hoc signed DMG (not notarized), and publish preview only when assets verified. No live provider compatibility or comprehensive GUI acceptance claim.

Tray remains opt-in for background closing; explicit quit checks open compose windows and active outbox dispatch. Linux without a compatible status notifier host stays visible. No formal Authenticode or Developer ID certificate configured.
