# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Development status

Development baseline, not a production release. Features include TLS IMAP/SMTP/POP3, unified inbox, folder sync, flags/move, safe HTML, attachments, rich text, signatures, drafts, OAuth implementation (bring your own registered desktop client), FTS5 search, batch actions, saved searches, local rules, contacts, templates, scheduled outbox, EML/MBOX/CSV/JSON exchange, backup/restore, storage maintenance, local calendar, ICS exchange, read-only CalDAV, meeting drafts and reminder implementation. M9 adds native compose/reader windows, global preferences and more themes.

Linux x86-64 native debug build, main/compose/reader Xvfb IPC smoke, and a debug DEB bundle passed. Core M8 verification: 17 unit tests and two loopback TLS integration tests. Real provider acceptance, complete native interaction, notifications, production installers and Windows/macOS builds remain pending. See [M9 notes](docs/M9.md) and [build evidence](docs/BUILD_EVIDENCE.md).

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
