# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Development status

Development baseline, not a production release. Features include TLS IMAP/SMTP and POP3, unified inbox, folder sync, flags/move, safe HTML, attachments, rich text, signatures, drafts, OAuth implementation (bring your own registered desktop client), FTS5 search, batch actions, saved searches, local rules, contacts, templates, scheduled outbox, EML/MBOX/CSV/JSON exchange, backup/restore, storage maintenance, local calendar, ICS import/export, read-only CalDAV sync, meeting invitation/response drafts and opt-in notification/reminder implementation. Frontend/core builds, 17 core tests and two loopback TLS protocol tests passed. Native desktop/runtime, real provider authorization, notification display and packaging remain pending. See [M8 notes](docs/M8.md) for limitations.

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
