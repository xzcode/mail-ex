# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Development status

Development baseline, not a production release. Features include TLS IMAP/SMTP, unified inbox and folder sync, read/star/move, safe HTML/plain reading, attachments, rich text, signatures, draft autosave, optional Sent copy, opt-in polling, desktop OAuth implementation for Gmail/Outlook.com/Microsoft 365 (bring your own registered client), FTS5 local search, batch mail actions, saved searches, automatic local category rules, contacts, rich/plain templates and a durable local outbox with scheduled delivery while the application runs. Frontend/core builds, 12 core tests and an expanded loopback TLS integration test passed. Actual provider authorization, native desktop runtime/scheduler and native notifications remain pending. See [organizer and outbox notes](docs/M5_M6.md) and [OAuth and search notes](docs/M4_M5.md).

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
