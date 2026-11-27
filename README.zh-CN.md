# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本包含多账号、TLS IMAP/SMTP、本地缓存、统一收件箱、文件夹同步、星标与已读、服务端邮件移动、编辑账号、HTML/纯文本安全阅读、附件预览/保存/收发/转发、基础富文本编辑、独立账号签名、含附件的草稿和自动保存、回复、可选服务器已发送副本、应用运行期间自动同步。前端与核心构建、6 项核心测试及本机 TLS 联调已通过。尚未完成真实邮箱服务商及原生桌面验收，暂不发布安装包。

OAuth、独立原生阅读/撰写窗口、大附件流式处理、后台服务与 IDLE、系统通知尚未完成。HTML 严格过滤所有属性、外链和图片；邮件正文、草稿及附件数据未加密，凭据使用每次安装独立的本地加密密钥。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
