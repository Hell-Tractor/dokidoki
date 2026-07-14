# Dokidoki 任务清单

按前后端分列；`- [x]` 已完成，`- [ ]` 未完成。优先级：P0 → P1 → P2+。

> **维护约定**：每次代码变更完成后，同步更新本文件（勾选已完成项、调整里程碑与「暂不做」说明）。
>
> **当前里程碑**：**M-07 主动消息**（待开始）→ 图片消息（M-04）降至 P1。

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
- [~] ~~`holiday_region`~~（公共节日方案暂缓，后续重评估）

### M-02 多角色会话（P0）

- [x] `GET /characters`
- [x] `GET /conversations`
- [x] `POST /conversations`（幂等创建；破冰 M-28）
- [x] 角色种子数据（`seeds/dev_characters.sql`；开发手动执行，不进 migration）

### 角色头像（P0，已完成）

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
- [x] burst / delivery / reply_scheduler（`chat/burst.rs` 等）
- [ ] _暂不做_：`send_message` WS 发送

### M-08 按角色设置（P0，已完成）

- [x] `GET /characters/{id}/settings`（勿扰时段等）
- [x] `PUT /characters/{id}/settings`（`dnd_start` / `dnd_end`，结合 `users.timezone`）
- [x] `user_character_settings` 查询与 upsert

### WebSocket（P0，扩展）

- [ ] 客户端 → `send_message`（与 REST 二选一，可后补）
- [x] 服务端 → `character_typing` / `turn_cancelled`
- [x] 服务端 → `message_read`
- [ ] _不推送_ `conversation_status`（后端内部状态，前端无需感知）

### M-05 角色人设（P0）

- [x] `persona` 模块：读取 `persona_json`、Prompt 静态层（MVP：T-01 + T-02）
- [x] Prompt 模板拼接（`persona/prompt.rs`）
- [x] T-03 当前状态注入（随 M-06）
- [x] T-04 有效记忆注入（随 M-16）
- [x] T-05 会话摘要注入（随 M-27）
- [ ] 完整 Prompt 组装（T-06～T-11 场景附加）

### M-06 日程与活动状态（P0，已完成）

- [x] `schedule` 模块：周模板 + 随机事件解析
- [x] CurrentState 解析（活动、心情、繁忙程度）
- [x] 聊天时从 `character_states` 读取并注入 T-03
- [x] 后台 `scheduler`：30～90s 随机间隔 tick

### 用户头像（P2，暂缓）

- [ ] `users.avatar_path` + 上传/GET 接口
- [ ] Settings 选图上传
- [ ] 聊天页用户气泡显示头像图

### M-09 人设/日程维护（P0，运维侧）

- [ ] 角色/日程数据维护方式（SQL 或脚本，MVP 无 App 内编辑）

### M-13 Burst Chat（P1，已完成）

- [x] `chat/burst.rs`：静默窗口合并、turn 管理
- [x] `chat/delivery.rs`：多气泡分时投递
- [x] `chat/reply_scheduler.rs`：availability 感知首响延迟（M-15）
- [x] `chat/conversation_fsm.rs`：active / winding_down / paused

### M-14 选择性回复（P1，已完成）

- [x] `NO_REPLY` / `END_TOPIC` / `[REPLY]` 动作解析
- [x] LLM `[END_TOPIC]` → `winding_down`；`[NO_REPLY]` → 无气泡
- [x] 用户告别 / substantive 消息驱动 paused ↔ active

### M-15 忙碌时回复延迟（P1，已完成）

- [x] 基于 availability + `reply_delay_factor` + jitter 计算 `reply_wait`
- [x] 活动段剩余时长上限
- [x] `reply_wait` 等待期间不显示 typing / 已读（已读由 M-17 实现）

### M-16 轻量记忆（P1，已完成）

- [x] `memory` 模块：STORE / FORGET / 过期清理
- [x] T-02 动作协议补充 `[STORE_MEMORY]` / `[FORGET_MEMORY]`
- [x] T-04 有效记忆注入 `context.rs`
- [x] `memory_key` UPSERT 去重与覆盖
- [x] 每日过期记忆清理任务

### M-17 延迟已读（P1，已完成）

- [x] `reply_wait` 窗口内随机推送 `message_read`
- [x] `high` availability 接近即时已读
- [x] 更新 `messages.read_at` 并 WS 推送

### M-27 长会话摘要（P1，已完成）

- [x] `summary/maybe_compact`：超阈值异步触发、conversation 锁
- [x] turn 计数与窗口切分（按 `turn_id`，非 message 条数）
- [x] T-20 摘要 LLM 调用 + 增量合并
- [x] `conversations.summary` + `summary_covers_until` 持久化
- [x] `context.rs`：T-05 注入 + 按 turn 取历史
- [x] 默认 `trigger_turns=80`、`keep_recent_turns=40`
- [x] 单元测试：触发边界、增量合并

### M-28 初识破冰（P1，已完成）

- [x] 首次 `POST /conversations` 触发破冰（1–3 条，不计入主动上限）
- [x] T-19 Prompt + 异步 LLM 生成
- [x] WS 推送破冰消息

### M-07 主动消息（P1）

> 本里程碑只做服务端生成 + 落库 + **WS 在线投递**；FCM / 离线推送延后到「推送与设备」。
>
> **全局**：当前日程槽 `kind=sleep` 时，**禁止一切** proactive（含 re_engage / daily_greeting）。日程时段**不重叠**；忙段与 sleep **首尾相接**时由 **`pre_sleep`** 在切入 sleep 前发晚安（及可选关心）。
>
> **同 tick 多触发**优先级（高 → 低）：`pre_sleep` → `daily_greeting`（含 special）→ `re_engage` → `silence_wake` → `mood_followup` → `schedule_change`。

#### 骨架与闸门

- [x] `proactive` 模块骨架（`tick`、触发求值、Prompt 组装、LLM、`[REPLY]`、复用 chat 投递）
- [x] 挂到现有 `scheduler`（与 schedule refresh 同循环或邻接 tick）
- [x] 全局日上限校验（`max_proactive_per_day`，按用户时区自然日；成功投递才写 `proactive_logs`）
- [x] 勿扰时段校验（`user_character_settings` + `users.timezone` 本地墙钟）
- [x] availability / `proactive.probability_factor` 抽样
- [x] LLM 失败：本轮跳过，不计日上限、不写 log
- [x] 投递成功：`proactive_logs` + `last_proactive_at`；终态→`active` 时清 `paused_at` / `winding_reason`
- [x] **`kind=sleep` 全局禁 proactive**
- [x] `winding_down` 超时：全局 config，默认 **5 分钟**无用户回复 → 按 `winding_reason` 落地终态（与告别落地同一套逻辑）

#### 会话状态（重构，相对原三段状态）

> 保留 `paused` = **话题正常结束，不必重启**（下次找新话题）。  
> 新增两态 = **话题异常中断，需要重启**。

| 状态 | 含义 | 可触发的 proactive |
|------|------|-------------------|
| `active` | 正常对话 | `pre_sleep` / daily_greeting / schedule_change / mood…（非 sleep 内） |
| `winding_down` | 收束中，还可多聊几句；**不可** proactive | — |
| `paused` | 正常结束 | `pre_sleep` / **`silence_wake`** |
| `paused_char_busy` | 角色去忙而中断 | `pre_sleep` / **`re_engage`**（忙完 ≈ `activity_ends_at`） |
| `paused_user_busy` | 用户去忙而中断 | `pre_sleep` / **`re_engage`**（persona 时间→概率曲线） |

- [x] `conversations` schema：`status` 扩展 + `winding_reason` / `winding_started_at`（已并入 init migration）
- [x] FSM：告别 / 超时按 `winding_reason` → `paused` / `paused_char_busy` / `paused_user_busy`
- [x] **角色忙**：`[END_TOPIC]` 且当时 `availability=low` → `winding_down` + `winding_reason=char_busy`；Prompt 强约束文案须说明去忙（可多聊几句再告别）
- [x] **用户忙**：主回复 Prompt 打 tag（如 `[USER_BUSY]`，有上下文）→ `winding_down` + `user_busy`；可多聊几句收尾
- [x] **正常收束**：`[END_TOPIC]` 且非 char_busy → `winding_reason=normal` → 终态 `paused`
- [x] persona：废弃固定等待语义的 `re_engage_after_minutes`（兼容可读）；新增 `user_busy_reengage` 曲线字段
- [x] 弃用「固定 `re_engage_after_minutes` 后必触发」的旧 re_engage 条件

#### 触发器（逐个实现）

- [x] **`pre_sleep`**  
  - 即将切入 `kind=sleep` **之前**发晚安类收束（不在 sleep 槽内刷屏）  
  - 每角色每日至多一次；计入 proactive 日上限与 `proactive_logs`（`trigger_type=pre_sleep`）  
  - 若当时 `paused_char_busy`：Prompt 按性格决定是否顺便问用户忙完没  
  - 覆盖忙段与 sleep 首尾相接、不宜立刻 `re_engage` 开聊的边界；睡醒后再 `daily_greeting` / `re_engage`  
  - Prompt：T-21（及可选附加关心）
- [x] **`daily_greeting`（合并 `special_date`）**  
  - 日程 `kind=wake` 段内 30–60 分钟窗；每角色每日至多一次  
  - 同轮可叠 T-18：用户生日；公共节日 **暂缓**
- [~] ~~节日地区 / Nager~~（暂缓，见下）
- [x] **`re_engage`**
  - [x] `paused_char_busy`：`now ≥ activity_ends_at`，非 sleep，再走可用性/概率闸门 → `active`
  - [x] `paused_user_busy`：按 `user_busy_reengage` 的 \(P(t)\) × 全局闸门抽样 → `active`
- [x] **`silence_wake`**：仅 `status=paused`；距用户末条 ≥ `silence_after_hours`
- [ ] **`mood_followup`**：上次对话负面情绪标记 + 冷却
- [x] **`schedule_change`**：`status=active`；刚进入 `kind=custom` 段且在 lead-in 窗内；availability ≥ medium；`persona.proactive.schedule_change_probability` × 全局闸门（每段一次确定性抽样）；T-16

#### Prompt

- [x] T-12 / T-13 / T-14 / T-15 / T-18
- [x] T-02 / 主回复：`[USER_BUSY]` tag；`availability=low` 下 `[END_TOPIC]` 须说明去忙
- [x] **`pre_sleep` 场景模板** T-21（晚安；`paused_char_busy` 时可叠加「忙完没」关心）
- [x] 落地 T-16（`schedule_change`）
- [ ] 落地 T-17
- [ ] `re_engage` 场景区分 char_busy / user_busy（可调 T-15 或附加）

#### 延后（记入文档，本期不做）

- [ ] **日程日初扰动**：每天开始时对活动 start/end 做适度扰动，使作息更自然（原计划放在 re_engage 浮动处，改挂日程）
- [ ] 创建/校验日程：时段不重叠、必有 `end`；服务端检测

#### P2 / 暂缓（公共节日）

- [ ] 公共节日方案重评估（原 Nager.Date 覆盖不全）
- [ ] 用户可配置节日地区 + 数据源适配
- [ ] 私人纪念日经 memory（`date.*`）在 `daily_greeting` 当日注入 T-18

### 角色生日反应（P2，暂不做）

> 与用户生日主动问候相反：这是**用户在角色生日当天祝他生日快乐**时的对话反应，不是主动消息触发器。

- [ ] 角色生日数据（如 `characters.birthday` 或 `persona_json`；种子/运维配置）
- [ ] 用户时区本地日 = 角色生日，且用户消息像生日祝福时，注入场景 Prompt，角色给出符合人设的特殊反应
- [ ] Prompt 模板（如 T-xx「角色生日被祝福」）；写入《Prompt规范》

### 推送与设备（P1，M-07 之后）

- [ ] `POST /devices`（FCM token 注册）
- [ ] `push` 模块：FCM 发送
- [ ] 离线主动消息通知 payload（主动消息已落库后再补推送）

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
- [~] ~~节日地区选择（`holiday_region`）~~（公共节日方案暂缓）

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

### M-13 Burst Chat 体验（P1，当前）

- [x] AppBar 副标题「对方正在输入…」（`character_typing`）
- [x] 多气泡逐条展示（服务端分时推送）
- [x] `turn_cancelled` 移除未展示气泡
- [ ] 纯 emoji 收发

### M-17 延迟已读（P1，已完成）

- [x] 用户消息「已送达 → 已读」状态展示（`message_read`）

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
- [ ] 角色生日：用户当日祝福时的特殊气泡体验（随后端场景 Prompt）
- [ ] TTS、好感度、引用回复等（见需求 §4.18）

---

## 文档与规范

- [x] 需求 / 概要 / 详细 / 接口设计说明书
- [x] Prompt 规范
- [x] `CONSTITUTION.md` 实现约定（含 §8.1 时区）
- [ ] README 部署与开发指南（MVP 完成后）
