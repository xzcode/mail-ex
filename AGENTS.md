# Mail EX development rules

- Read docs/PRODUCT.md, docs/ARCHITECTURE.md and docs/ROADMAP.md first. Update phase status with evidence.
- Default branch master; inspect HEAD and working tree before modifying. Keep unrelated projects untouched.
- Local-first. Never transmit mail or credentials to a Mail EX service. No artificial account/message/tab/cache quotas; paginate/batch for resource control.
- Do not log passwords, authorization codes, refresh tokens, raw message bodies or recipient addresses. Never use insecure TLS fallback. No HTML execution or remote-image loads in mail preview.
- Never report mock data, unsupported providers or successful builds as real integration. Do not send real email without the user's explicit choice of recipient and content. Use local fixtures for automated tests.
- Preserve cached data on sync failure; distinguish IMAP server identity by account/folder/UIDVALIDITY/UID.
- Default English README with Chinese alternative. UI supports localization, follows system language by default; support light/dark themes and resize panes.
- Run targeted frontend and Rust checks when available; record limitations. Commit logical milestones and update handoff documentation.
