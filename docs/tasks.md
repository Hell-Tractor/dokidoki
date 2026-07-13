# Dokidoki 任务清单

按前后端分列；`- [x]` 已完成，`- [ ]` 未完成。优先级：P0 → P1 → P2+。

> **维护约定**：每次代码变更完成后，同步更新本文件（勾选已完成项、调整里程碑与「暂不做」说明）。
>
> **当前里程碑**：M-08 按角色设置已完成 → 图片消息（M-04）降至 P1。

---

## 后端（dokidoki-server）

### 基础设施

- [x] 项目骨架（Axum、config、error、AppState、MySQL 连接与迁移）
- [x] 统一响应 `{ data }` / `{ error }`（`ApiResponse`、`AppError`）
- [x] `ValidatedJson` 请求校验
- [x] 集成测试框架（`TEST_DATABASE_URL`、`test_support`）
- [x] MySQL 会话固定 UTC（`db/pool` `SET time_zone = '+00:00'`）
- [x] `time` 模块（用户时区日界线、勿扰判断）
- [ ] Docker Compose 部署（server + mysql + caddy）
- [ ] `config.toml.example` 与部署文档

### M-01 连接与鉴权（P0）

- [x] `GET /health`
- [x] `POST /auth/register`
- [x] `POST /auth/login`
- [x] Bearer Token 鉴权（`user_sessions`、SHA-256 存哈希）
- [x] `require_auth` 中间件 + 公开/受保护路由分组
- [x] `AuthUser` extractor
- [ ] Token 登出 / 失效（删 session 或 `expires_at`）
- [ ] 集成测试覆盖率 > 90%（全模块度量）

### M-26 用户档案（P1）

- [x] `GET /me`
- [x] `PATCH /me`（display_name、birthday、timezone、max_proactive_per_day）
- [x] 注册必填 `timezone`（IANA；客户端默认设备时区）
- [ ] 支持 PATCH 清空 birthday（`null` 语义）

### M-02 多角色会话（P0）

- [x] `GET /characters`
- [x] `GET /conversations`
- [x] `POST /conversations`（幂等创建；破冰待 M-28）
- [x] 角色种子数据（`seeds/dev_characters.sql`；开发手动执行，不进 migration）

### 角色头像（P0，当前）

- [x] 本地存储目录（`config.upload.dir`，`avatars/` 子目录）
- [x] `GET /characters/{id}/avatar`（鉴权；读 `characters.avatar_path`；无文件时占位图）
- [x] 种子/运维：为角色写入 `avatar_path`（小爱静态图 + 启动时 bootstrap）
- [x] `GET /characters` 返回 `avatar_url`（前端已对接）

### M-03 消息 · 文本（P0，已完成）

- [x] `GET /conversations/{id}/messages`（分页）
- [x] `POST /conversations/{id}/messages`（文本）
- [x] 用户发文本后触发角色回复（Fake / HTTP LLM）

### M-04 消息 · 图片（P1，暂缓）

- [ ] `POST /conversations/{id}/messages/image`（multipart）
- [ ] `GET /messages/{id}/image`（鉴权 + 归属校验）
- [ ] 聊天图片本地存储（`/data/uploads`）
- [ ] _暂不做_：LLM 多模态识图回复（`llm/vision.rs`）

### 基础聊天（P0，前端启动前）

- [x] `llm` 模块：`LlmClient` + `FakeLlmClient`
- [x] `config.llm.mode = "fake"` + `fake_default`
- [x] `POST /dev/llm/queue`（debug 构建；预设下一条 LLM 输出）
- [x] `chat` 模块：`on_user_text_sent` → LLM → 解析 `[REPLY]` → 落库
- [x] `chat/context.rs`：组装 system prompt + 最近消息历史
- [x] `chat/parser.rs`：`[REPLY]` + `|||` 分气泡
- [x] `ws_hub` + `GET /api/v1/ws` 鉴权
- [x] WS：`connected` / `subscribe` / `ping` / `pong`
- [x] WS 推送：`message`（角色回复）
- [x] 集成测试：`tests/chat_api.rs`（DB + WS）
- [ ] _暂不做_：burst、回复延迟、typing、`send_message` WS 发送

### M-08 按角色设置（P0，已完成）

- [x] `GET /characters/{id}/settings`（勿扰时段等）
- [x] `PUT /characters/{id}/settings`（`dnd_start` / `dnd_end`，结合 `users.timezone`）
- [x] `user_character_settings` 查询与 upsert

### WebSocket（P0，扩展）

- [ ] 客户端 → `send_message`（与 REST 二选一，可后补）
- [ ] 服务端 → `character_typing` / `message_read` / `turn_cancelled` / `conversation_status`

### M-05 角色人设（P0）

- [x] `persona` 模块：读取 `persona_json`、Prompt 静态层（MVP：T-01 + T-02）
- [x] Prompt 模板拼接（`persona/prompt.rs`）
- [ ] 完整 Prompt 组装（T-03～T-11 场景附加、CurrentState、记忆、摘要）

### M-06 日程与活动状态（P0）

- [ ] `schedule` 模块：周模板 + 随机事件
- [ ] CurrentState 解析（活动、心情、繁忙程度）

### M-09 人设/日程维护（P0，运维侧）

- [ ] 角色/日程数据维护方式（SQL 或脚本，MVP 无 App 内编辑）

### M-13 Burst Chat（P1）

- [ ] `chat/burst.rs`：静默窗口合并、turn 管理
- [ ] `chat/delivery.rs`：多气泡分时投递
- [ ] `chat/reply_scheduler.rs`：回复延迟队列
- [ ] `chat/conversation_fsm.rs`：active / winding_down / paused

### M-14 选择性回复（P1）

- [ ] `NO_REPLY` / `END_TOPIC` 动作处理
- [ ] 会话 `paused` 与恢复

### M-15 忙碌时回复延迟（P1）

- [ ] 基于 availability + 性格 + 随机性的首响延迟
- [ ] 忙碌期间不发送 typing

### M-16 轻量记忆（P1）

- [ ] `memory` 模块：STORE / FORGET / 过期清理
- [ ] 记忆去重与覆盖

### M-17 延迟已读（P1）

- [ ] 忙碌时「已送达 → 已读」分阶段推送

### M-27 长会话摘要（P1）

- [ ] 超阈值触发摘要（默认 30 轮触发，保留 10 轮原文）

### M-28 初识破冰（P1）

- [ ] 首次 `POST /conversations` 触发破冰（1–3 条，不计入主动上限）

### M-07 主动消息（P1）

- [ ] `proactive` 模块：六类触发器
- [ ] 全局日上限校验（`max_proactive_per_day`，按用户时区自然日）
- [ ] 勿扰时段校验（`users.timezone` + 本地墙钟）
- [ ] `scheduler` 定时任务

### 推送与设备（P1）

- [ ] `POST /devices`（FCM token 注册）
- [ ] `push` 模块：FCM 发送
- [ ] 离线主动消息通知 payload

### 外部适配器

- [x] `llm/http.rs`：OpenAI 兼容 HTTP 客户端（`mode = "http"`）
- [ ] `llm/vision.rs`：图片多模态（P1，随 M-04 图片消息一并实现）

---

## 前端（dokidoki-app）

> M-03 文本聊天 + Settings + 角色头像 + 按角色设置已完成；图片发收降至 P1。

### 项目骨架

- [x] Flutter 工程初始化（Android 优先）
- [x] 路由（go_router）
- [x] 全局状态（Riverpod：`apiClientProvider`、`wsClientProvider`、`authConfigProvider` 等）
- [x] API 客户端（dio + Bearer Token 注入）
- [x] WebSocket 客户端（自动重连、subscribe）
- [x] SecureStorage（Token）+ SharedPreferences（Server URL）

### M-01 连接与鉴权（P0）

- [x] P-01 SplashPage：读配置，决定跳转 Setup / Home
- [x] P-02 SetupPage 步骤 1：Server URL + `GET /health` 测试连接
- [x] P-02 SetupPage 步骤 2：注册 / 登录 Tab
- [x] 注册表单（username、password 二次确认、display_name、birthday、**timezone**）
- [x] 登录表单
- [x] Token 持久化；401 清 Token 回 Setup
- [x] 不含 LLM API Key（FR-01-03）

### M-26 用户档案（P1）

- [x] P-05 SettingsPage：称呼、生日、时区编辑（`PATCH /me`）
- [x] 全局主动消息日上限 Stepper

### M-02 多角色会话（P0）

- [x] P-03 HomePage：会话列表（`GET /conversations`）
- [x] 空状态 + 角色列表入口（`GET /characters`）
- [x] 点击角色创建会话（`POST /conversations`）
- [x] 进入 Home 建立全局 WS

### M-03 聊天 · 文本（P0，已完成）

- [x] P-04 ChatPage：消息列表分页（`GET /messages`）
- [x] WS `subscribe(conversation_id)`
- [x] 文本发送（`POST /messages`；后续可切 WS `send_message`）
- [x] 收角色回复（WS `message`）
- [x] 双方头像（连续同方仅首条显示；角色图经 `GET /avatar`）
- [x] 历史消息加载（上拉 `before` 分页）

### M-04 聊天 · 图片（P1，暂缓）

- [ ] 图片发送（相册/相机 + 可选 caption）
- [ ] 图片气泡展示与全屏预览

### M-13 Burst Chat 体验（P1）

- [ ] AppBar 副标题「对方正在输入…」（`character_typing`）
- [ ] 多气泡逐条展示
- [ ] `turn_cancelled` 移除未展示气泡
- [ ] 纯 emoji 收发

### M-17 延迟已读（P1）

- [ ] 用户消息「已送达 → 已读」状态展示（`message_read`）

### M-08 按角色设置（P0，已完成）

- [x] P-06 CharacterSettingsPage：勿扰时段（`GET/PUT /characters/{id}/settings`）
- [x] ChatPage ⋮ 菜单入口

### M-01 设置与连接（P0）

- [x] P-05 SettingsPage：Server URL 修改、退出登录

### 推送（P1）

- [ ] FCM 集成（Android）
- [ ] `POST /devices` 上报 token
- [ ] 后台通知点击跳转会话

### Phase 2+（暂不实现）

- [ ] iOS 构建
- [ ] 会话列表 `current_activity` 副标题
- [ ] TTS、好感度、引用回复等（见需求 §4.18）

---

## 文档与规范

- [x] 需求 / 概要 / 详细 / 接口设计说明书
- [x] Prompt 规范
- [x] `CONSTITUTION.md` 实现约定（含 §8.1 时区）
- [ ] README 部署与开发指南（MVP 完成后）
