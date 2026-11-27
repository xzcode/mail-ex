# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Development status

Development baseline, not a production release. Features include TLS IMAP/SMTP, unified inbox and folder sync, mail flags/move, safe HTML, attachments, rich text, signatures, drafts, optional Sent copy, OAuth implementation (bring your own registered desktop client), FTS5 search, batch actions, saved searches, local classification, contacts, templates, durable scheduled outbox, EML/MBOX local archive and export, contact CSV and credential-free account JSON exchange, backup/restore, storage location copy and cache maintenance. Frontend/core builds, 14 core tests and loopback TLS protocol fixture passed. Native desktop runtime, file dialogs/restart activation, real provider authorization and notifications remain pending. See [M7 data notes](docs/M7.md).

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
