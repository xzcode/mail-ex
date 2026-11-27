# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本包含多账号、TLS IMAP/SMTP、本地缓存、统一收件箱、文件夹同步、星标与已读、邮件移动、HTML/纯文本安全阅读、附件收发与转发、富文本编辑、签名、草稿、可选服务器已发送副本、OAuth 桌面授权实现、FTS5 全文搜索、批量操作、保存搜索、自动分类、联系人、模板、持久化发件箱、定时发送、EML/MBOX 本地归档及导出、联系人 CSV、无凭据的邮箱配置 JSON、备份恢复、存储目录复制及缓存维护。前端与核心构建、14 项核心测试和本机 TLS 联调已通过。

真实邮箱服务商、原生桌面运行、文件对话框、重启后数据路径启用、系统通知尚未验收或完成。备份包含数据库与凭据密钥，未整体加密，仅恢复可信且同架构版本备份。恢复时待发送/发送中的记录停用，不自动重发。原始数据不覆盖，重启后启用新位置。暂未发布安装包。详情见 [M7 说明](docs/M7.md)。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
