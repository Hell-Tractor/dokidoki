# Dokidoki 任务清单

按前后端分列；`- [x]` 已完成，`- [ ]` 未完成。优先级：P0 → P1 → P2+。

---

## 后端（dokidoki-server）

### 基础设施

- [x] 项目骨架（Axum、config、error、AppState、MySQL 连接与迁移）
- [x] 统一响应 `{ data }` / `{ error }`（`ApiResponse`、`AppError`）
- [x] `ValidatedJson` 请求校验
- [x] 集成测试框架（`TEST_DATABASE_URL`、`test_support`）
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
- [x] `PATCH /me`（display_name、birthday、max_proactive_per_day）
- [ ] 支持 PATCH 清空 birthday（`null` 语义）

### M-02 多角色会话（P0）

- [x] `GET /characters`
- [x] `GET /conversations`
- [x] `POST /conversations`（幂等创建；破冰待 M-28）
- [ ] 角色种子数据 / 迁移

### M-03 / M-04 消息（P0）

- [x] `GET /conversations/{id}/messages`（分页）
- [x] `POST /conversations/{id}/messages`（文本）
- [ ] `POST /conversations/{id}/messages/image`（multipart）
- [ ] `GET /messages/{id}/image`（鉴权 + 归属校验）
- [ ] 图片本地存储（`/data/uploads`）

### M-08 用户设置（P1）

- [ ] `GET /characters/{id}/settings`
- [ ] `PUT /characters/{id}/settings`（勿扰时段）

### WebSocket（P0）

- [ ] `GET /api/v1/ws` 连接与鉴权
- [ ] 事件：`connected`
- [ ] 客户端 → `subscribe` / `send_message` / `ping`
- [ ] 服务端 → `message` / `character_typing` / `message_read` / `turn_cancelled` / `conversation_status`
- [ ] `pong` 心跳

### M-05 角色人设（P0）

- [ ] `persona` 模块：读取 `persona_json`、Prompt 静态层
- [ ] Prompt 模板拼接（`persona/prompt.rs`）

### M-06 日程与活动状态（P0）

- [ ] `schedule` 模块：周模板 + 随机事件
- [ ] CurrentState 解析（活动、心情、繁忙程度）

### M-09 人设/日程维护（P0，运维侧）

- [ ] 角色/日程数据维护方式（SQL 或脚本，MVP 无 App 内编辑）

### M-13 Burst Chat（P1）

- [ ] `chat/burst.rs`：静默窗口合并、turn 管理
- [ ] `chat/delivery.rs`：多气泡分时投递
- [ ] `chat/reply_scheduler.rs`：回复延迟队列
- [ ] `chat/parser.rs`：LLM 动作头解析
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
- [ ] 全局日上限校验（`max_proactive_per_day`）
- [ ] 勿扰时段校验
- [ ] `scheduler` 定时任务

### 推送与设备（P1）

- [ ] `POST /devices`（FCM token 注册）
- [ ] `push` 模块：FCM 发送
- [ ] 离线主动消息通知 payload

### 外部适配器

- [ ] `llm` 模块：OpenAI 兼容 HTTP 客户端
- [ ] `llm/vision.rs`：图片多模态

---

## 前端（dokidoki-app）

> Flutter 客户端尚未创建；以下均为待办。

### 项目骨架

- [ ] Flutter 工程初始化（Android 优先）
- [ ] 路由（go_router 等）
- [ ] 全局状态（Riverpod：`apiClientProvider`、`wsClientProvider`、`authConfigProvider` 等）
- [ ] API 客户端（dio + Bearer Token 注入）
- [ ] WebSocket 客户端（自动重连、subscribe）
- [ ] SecureStorage（Token）+ SharedPreferences（Server URL）

### M-01 连接与鉴权（P0）

- [ ] P-01 SplashPage：读配置，决定跳转 Setup / Home
- [ ] P-02 SetupPage 步骤 1：Server URL + `GET /health` 测试连接
- [ ] P-02 SetupPage 步骤 2：注册 / 登录 Tab
- [ ] 注册表单（username、password 二次确认、display_name、birthday）
- [ ] 登录表单
- [ ] Token 持久化；401 清 Token 回 Setup
- [ ] 不含 LLM API Key（FR-01-03）

### M-26 用户档案（P1）

- [ ] P-05 SettingsPage：称呼、生日编辑（`PATCH /me`）
- [ ] 全局主动消息日上限 Stepper

### M-02 多角色会话（P0）

- [ ] P-03 HomePage：会话列表（`GET /conversations`）
- [ ] 空状态 + 角色列表入口（`GET /characters`）
- [ ] 点击角色创建会话（`POST /conversations`）
- [ ] 进入 Home 建立全局 WS

### M-03 / M-04 聊天与图片（P0）

- [ ] P-04 ChatPage：消息列表分页（`GET /messages`）
- [ ] WS `subscribe(conversation_id)`
- [ ] 文本发送（每条独立 `POST`，不客户端合并）
- [ ] 图片发送（相册/相机 + 可选 caption）
- [ ] 图片气泡展示与全屏预览
- [ ] 双方头像（连续同方仅首条显示）
- [ ] 历史消息加载（上拉 `before` 分页）

### M-13 Burst Chat 体验（P1）

- [ ] AppBar 副标题「对方正在输入…」（`character_typing`）
- [ ] 多气泡逐条展示
- [ ] `turn_cancelled` 移除未展示气泡
- [ ] 纯 emoji 收发

### M-17 延迟已读（P1）

- [ ] 用户消息「已送达 → 已读」状态展示（`message_read`）

### M-08 用户设置（P1）

- [ ] P-06 CharacterSettingsPage：勿扰时段（`GET/PUT /characters/{id}/settings`）
- [ ] ChatPage ⋮ 菜单入口

### M-01 设置与连接（P0）

- [ ] P-05 SettingsPage：Server URL 修改、退出登录

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
- [x] `CONSTITUTION.md` 实现约定
- [ ] README 部署与开发指南（MVP 完成后）
