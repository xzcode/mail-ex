# Handoff — M2 / partial M3

Repository: xzcode/mail-ex, master. M0 design established. M1/M2 implementation baseline plus partial M3. Latest validation: frontend production build, Rust core build, 4 unit tests and 1 loopback TLS integration test passed. See BUILD_EVIDENCE.md. Core Cargo.lock and npm lockfile committed.

Priorities:
1. Finish safe HTML preview and MIME attachment listing/download/upload, rich editor, signatures, autosave, sent-folder APPEND and independent compose/read windows.
2. Build native Tauri desktop on supported OS with required GTK/WebKitGTK dependencies. Run actual desktop interaction and provider tests without sending real mail absent explicit user request. Add desktop Cargo.lock when native dependency resolution is verified.
3. Improve synchronization: UID checkpoints/IDLE, cancellation, per-account scheduler, native notifications, safe folder CRUD. No blanket EXPUNGE.
4. Add OAuth PKCE and provider-specific app registration/token refresh. Check imap-proto upgrade path for future Rust compatibility.
5. Cross-platform CI and release packaging.

Test reproduction: npm ci && npm run build; cargo test --manifest-path core/Cargo.toml --locked; on Linux with openssl and Python 3, python3 scripts/protocol-fixture.py (loopback only).

Limitations: credentials encrypted with adjacent per-install key; messages and drafts not encrypted. No remote HTML execution. Polling is opt-in and runs only while UI runs. IMAP MOVE support required for move; archive/trash roles from server. No actual provider or desktop runtime acceptance, no production installer.
