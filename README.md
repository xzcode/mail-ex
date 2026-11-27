# Mail EX

Local-first, open-source desktop email client for Windows, macOS and Linux.

[简体中文](README.zh-CN.md) · [Product specification](docs/PRODUCT.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/ROADMAP.md)

## Development status

M0 specifications and M1 implementation are in progress. This repository contains a real TLS IMAP/SMTP adapter and local SQLite cache; it is not yet a production release. Provider OAuth, HTML rendering, attachments, folder synchronization and background delivery are future work.

## Development

Prerequisites: Node.js 22+, Rust stable (1.85+), Tauri 2 system prerequisites, and a C/C++ toolchain. Linux additionally needs WebKitGTK 4.1, GTK 3, OpenSSL development packages and the Tauri system dependencies.

```sh
npm install
npm run check
npm run build
cargo test --manifest-path core/Cargo.toml
npm run tauri dev
```

To build a native package on the target OS: `npm run tauri build`. Development uses the local application data directory and requires no Mail EX server. Do not use a real mailbox until the integration tests and security review are complete.

## License

MIT.
