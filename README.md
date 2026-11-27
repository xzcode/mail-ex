# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Downloads

[v0.1.0 Preview](https://github.com/xzcode/mail-ex/releases/tag/v0.1.0) is the first public pre-release.

| Platform | Installer | Verification |
| --- | --- | --- |
| Windows x64 | [EXE](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/Mail-EX-0.1.0-windows-x64.exe) | Native CI build and core tests |
| Linux x86-64 | [DEB](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/Mail-EX-0.1.0-linux-x86_64.deb) | Optimized native build, extracted-package startup and IPC |
| macOS ARM64 / Intel | Pending | Final DMG verification required |

[SHA-256 checksums](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/SHA256SUMS.txt) · [Build manifest](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/build-manifest.json)

This is a preview, not a production stability or actual mail-provider compatibility guarantee. Use a test mailbox first. Windows installer is not Authenticode-signed. Linux requires the declared GTK/WebKitGTK and TLS runtime dependencies.

## Features

 Features include TLS IMAP/SMTP/POP3, unified inbox, folder sync, flags/move, safe HTML, attachments, rich text, signatures, drafts, OAuth implementation (bring your own registered desktop client), FTS5 search, batch actions, saved searches, local rules, contacts, templates, scheduled outbox, EML/MBOX/CSV/JSON exchange, backup/restore, storage maintenance, local calendar, ICS exchange, read-only CalDAV, meeting drafts and reminder implementation. M9 adds native compose/reader windows, global preferences and more themes.

Release source `21f26bf`: Linux optimized startup and installer extraction verified; 17 core unit tests and two loopback TLS protocol integration tests passed. Hosted Windows, Apple Silicon and Intel macOS core test steps passed; Windows installer artifact verified. macOS installer verification remains pending. Real provider authorization, full desktop interactions and notification display remain unverified. See [M9 notes](docs/M9.md) and [build evidence](docs/BUILD_EVIDENCE.md).

## Development

Prerequisites: Node.js 22+, current stable Rust, Tauri 2 system prerequisites, and a C/C++ toolchain. Linux additionally needs WebKitGTK 4.1, GTK 3, OpenSSL development packages and Tauri system dependencies.

```sh
npm ci
npm run build
cargo test --manifest-path core/Cargo.toml
npm run tauri dev
```

To build a native package on the target OS: `npm run tauri build`. No Mail EX server required.

## Security notes

Only strictly sanitized HTML is rendered; remote images and all HTML attributes are stripped. Credentials are encrypted using a per-install key stored locally next to the database; this is not full database encryption. Move is permitted only if the server advertises IMAP MOVE. No blanket EXPUNGE or insecure fallback. Do not connect an important mailbox until integration and security validation are complete.

## License

MIT.
