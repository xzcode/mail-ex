# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本包含多账号、TLS IMAP/SMTP、本地缓存、统一收件箱、文件夹同步、星标与已读、邮件移动、HTML/纯文本安全阅读、附件收发与转发、富文本编辑、账号签名、草稿自动保存、可选服务器已发送副本、运行期间自动同步、Gmail/Outlook.com/Microsoft 365 桌面 OAuth 实现（需自行注册客户端）、FTS5 本地全文搜索、批量操作、保存搜索、自动本地分类规则、联系人及收件人补全、邮件模板、持久化本地发件箱和定时发送。前端与核心构建、12 项核心测试及扩展本机 TLS 发件队列联调已通过。

真实邮箱服务商授权、原生桌面运行和定时线程、独立原生窗口、大附件流式处理及系统通知尚未验收或完成。定时发送需要应用进程运行，关闭期间到期的待发送邮件将在下次启动时处理，不确定发送不会自动重试。自动规则仅改变本地分类，不修改服务器邮件。邮件正文、草稿、发件队列及附件未加密，凭据使用独立本地密钥。暂不发布安装包。详情见 [M5/M6 说明](docs/M5_M6.md) 和 [M4/M5 说明](docs/M4_M5.md)。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
