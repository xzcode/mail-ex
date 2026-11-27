# Build evidence — 2026-11-27

- Source commit `9abcd9d59f3c5f40ebf547e9dc49caf44b13c1c1` was cloned from GitHub into an isolated build directory on an authorized Linux host.
- Node 24.22.3 / npm 11.19.1: `npm install --no-audit --no-fund` succeeded (52 packages); `npm run build` succeeded (vue-tsc and Vite 6.4.6; 14 transformed modules). npm emitted an esbuild install-script approval warning but the production build succeeded. npm lockfile was generated on that host.
- Tauri CLI icon generation from `assets/app-icon.svg` succeeded; desktop PNG, ICO and ICNS included.
- Rust core compilation/tests and native desktop build not yet verified; Rust toolchain installation stalled. No real mailbox was connected, no real message sent. No release package built.
