# Mail EX

[English](README.md)

本地优先、开源、跨平台邮件聚合客户端。直接连接邮箱服务商，不要求注册 Mail EX 账号。

## 下载

[Mail EX v0.1.0 预览版](https://github.com/xzcode/mail-ex/releases/tag/v0.1.0) 已公开发布。

| 平台 | 安装包 | 验证情况 |
| --- | --- | --- |
| Windows x64 | [EXE](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/Mail-EX-0.1.0-windows-x64.exe) | 原生 CI 构建、核心测试通过 |
| Linux x86-64 | [DEB](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/Mail-EX-0.1.0-linux-x86_64.deb) | 优化编译、解包启动及 IPC 检查通过 |
| macOS ARM64 / Intel | 待补充 | DMG 最终签名验收未完成 |

[SHA-256 校验文件](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/SHA256SUMS.txt) · [构建清单](https://github.com/xzcode/mail-ex/releases/download/v0.1.0/build-manifest.json)

本版本为预览版，建议先使用测试邮箱。Windows 安装包没有 Authenticode 正式签名。

## 功能

目前包含多账号、TLS IMAP/SMTP/POP3、统一收件箱、文件夹同步、星标与已读、邮件移动、HTML 安全阅读、附件、富文本、签名、草稿、OAuth 授权实现、全文搜索、批量操作、保存搜索、自动分类、联系人、模板、持久化发件箱、定时发送、EML/MBOX/CSV/JSON 导入导出、备份恢复、存储维护、本地日历、ICS、CalDAV 只读同步、会议邀请和回复草稿、提醒与通知实现。M9 增加独立撰写／阅读窗口、全局设置和更多主题。

发布源码 `21f26bf`：Linux 优化版安装包启动、17 项核心测试及 2 项本机 TLS 联调通过；Windows 原生构建成功。三个托管平台核心测试通过。macOS 最终签名验收、真实服务商、完整桌面交互和系统通知展示尚未验收。详情见 [M9 说明](docs/M9.md) 和 [验证记录](docs/BUILD_EVIDENCE.md)。

开发环境：Node.js 22+、当前稳定版 Rust、Tauri 2 系统依赖。运行 `npm ci`、`npm run build`、`npm run tauri dev`；独立核心测试：`cargo test --manifest-path core/Cargo.toml`。

[产品规范](docs/PRODUCT.md) · [架构](docs/ARCHITECTURE.md) · [路线图](docs/ROADMAP.md) · [交接](docs/HANDOFF.md)
