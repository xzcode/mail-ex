# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Development status

Development baseline, not a production release. The code includes password/app-code TLS IMAP/SMTP, SQLite cache, unified inbox, folder sync, starred/read status, UID MOVE, account editing, safe HTML/plain reading, attachment preview/save/compose/forward, basic rich text, per-account signatures, local drafts with autosave, replies, optional server Sent copy and opt-in inbox polling while open. Frontend/core builds, six core tests and a loopback TLS integration test passed. Native desktop UI and provider compatibility remain unverified. OAuth, independent native windows, attachment streaming and robust background synchronization/notifications are pending.

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
