# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本包含多账号、TLS IMAP/SMTP、本地缓存、统一收件箱、文件夹同步、星标与已读、邮件移动、HTML/纯文本安全阅读、附件收发与转发、富文本编辑、账号签名、草稿自动保存、可选服务器已发送副本、运行期间自动同步、Gmail/Outlook.com/Microsoft 365 桌面 OAuth 实现（需自行注册客户端）、FTS5 本地全文搜索和批量操作。前端与核心构建、10 项核心测试及扩展本机 TLS 联调已通过。

真实邮箱服务商授权及令牌交换、原生桌面运行、保存搜索、规则、独立原生窗口、大附件流式处理、后台服务和系统通知尚未验收或完成。暂不发布安装包。OAuth 配置见 [M4/M5 说明](docs/M4_M5.md)。邮件正文、草稿及附件未加密，凭据使用独立本地密钥。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
