# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

开发版本包含多账号、TLS IMAP/SMTP/POP3、统一收件箱、文件夹同步、星标与已读、邮件移动、HTML 安全阅读、附件、富文本、签名、草稿、OAuth 授权实现、全文搜索、批量操作、保存搜索、自动分类、联系人、模板、持久化发件箱、定时发送、EML/MBOX/CSV/JSON 导入导出、备份恢复、存储维护、本地日历、ICS、CalDAV 只读同步、会议邀请和回复草稿、提醒与通知实现。M9 增加独立撰写／阅读窗口、全局设置和更多主题。

Linux x86-64 原生调试版编译、主窗口／撰写窗口／阅读窗口 Xvfb IPC 启动检查及调试版 DEB 打包已通过。M8 的 17 项核心测试和 2 项本机 TLS 联调通过。真实邮箱、完整桌面交互、原生系统通知、Windows/macOS、正式构建及 Release 尚未验收。详情见 [M9 说明](docs/M9.md) 和 [验证记录](docs/BUILD_EVIDENCE.md)。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
