# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本已编写多账号、TLS IMAP/SMTP、本地缓存、统一收件箱、文件夹发现和同步、星标与已读标记、服务端邮件移动、编辑账号、本地草稿、纯文本撰写、回复/回复全部/转发、应用运行时可选自动同步。服务商实际兼容性仍需联调。暂不发布安装包。

OAuth、HTML 安全渲染、附件管理、后台服务与 IDLE、系统通知、已发送邮件 APPEND 尚未完成。邮件正文未加密，凭据使用每次安装独立的本地加密密钥。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
