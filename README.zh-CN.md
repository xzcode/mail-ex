# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

当前为 M0/M1 开发版本：账号配置、TLS IMAP/SMTP、统一收件箱、SQLite 邮件缓存、纯文本撰写及发送、已读状态。OAuth、HTML 安全渲染、附件管理、完整文件夹同步及后台同步尚未完成。暂未发布安装包，尚未进行真实邮箱联调。

开发环境：Node.js 22+、Rust stable、Tauri 2 系统依赖。运行 `npm install`、`npm run check`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md)
