# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本包含多账号、TLS IMAP/SMTP/POP3、统一收件箱、文件夹同步、星标与已读、邮件移动、HTML 安全阅读、附件、富文本、签名、草稿、OAuth 授权实现、全文搜索、批量操作、保存搜索、自动分类、联系人、模板、持久化发件箱、定时发送、EML/MBOX/CSV/JSON 导入导出、备份恢复、存储维护、本地日历、ICS 导入导出、CalDAV 只读同步、会议邀请与响应草稿、可选系统通知和日程提醒。前端和核心构建、17 项核心测试、2 项本机 TLS 联调通过。

POP3 永不删除服务器邮件。CalDAV 暂仅只读同步，复杂时区保留原当地时间，重复事件尚未展开。真实邮箱服务商、原生桌面运行、系统通知和安装包未验收；暂无 Release。备份包含数据库及凭据密钥，未整体加密，仅恢复可信的同架构版本备份。详情见 [M8 说明](docs/M8.md)。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
