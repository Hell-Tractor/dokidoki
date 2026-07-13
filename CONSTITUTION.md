# Dokidoki Constitution

本文件汇总 **Dokidoki 项目的实现约定与开发规范**，供人类与 AI 协作者共同遵守。详细需求与接口契约见 `docs/`；本文档聚焦「怎么写代码」。

---

## 1. 产品与技术栈

| 项 | 约定 |
|----|------|
| 产品名 | **Dokidoki**（无中文名） |
| 定位 | 基于 LLM 的角色扮演聊天 App；面向技术用户自部署 |
| 后端 | Rust · Axum · Tokio · sqlx |
| 数据库 | MySQL |
| 客户端 | Flutter（Android 优先） |
| 部署 | Docker Compose · Caddy |
| MVP 形态 | 单 crate 单二进制，不拆 workspace |

---

## 2. 仓库结构

```
dokidoki/
├── CONSTITUTION.md      # 本文件
├── docs/                # 需求与设计文档
├── dokidoki-server/     # Rust 后端
│   ├── migrations/
│   └── src/
│       ├── api/           # 传输层：路由、鉴权、Request/Response
│       ├── domain/        # 领域服务：auth, conversations, messages, …
│       └── db/            # 仓储：sqlx 查询、models
└── dokidoki-app/        # Flutter 客户端
```

---

## 3. 模块文件约定

- 顶层模块：`module.rs`（如 `chat.rs`、`api.rs`）
- 子模块：`module/sub.rs`（如 `chat/burst.rs`、`api/rest/auth.rs`）
- **不使用** `module/mod.rs`

---

## 4. 后端分层

采用 **传输层 → 领域服务 → 仓储 / 外部适配器** 三层心智模型，**不采用** Java 式 Controller → Service → DAO → Mapper 四层。

| 分层 | 目录 | 职责 | 禁止 |
|------|------|------|------|
| 传输层 | `api` | 路由、鉴权、序列化、调用领域服务 | 不写 burst、Prompt、状态机等业务逻辑；**不直接**调 `db::queries` |
| 领域服务 | `domain/`（`auth`、`conversations`、`messages`、`chat`、`proactive` 等） | 业务编排、领域规则 | 不直接拼 HTTP 响应；不散落 SQL |
| 仓储 | `db` | sqlx 查询、`models`、事务 | 不 import 上层模块 |
| 外部适配器 | `llm`、`push` | 第三方 API 封装 | 不含领域规则 |

### 4.1 Handler 约定

- Handler 保持薄（通常 < 30 行）：`ValidatedJson` 反序列化 + 校验 → 调领域服务 → `From` 映射 Response → 包进 `ApiResponse`
- Handler **不得**直接调用 `db::queries`；须经对应领域模块（如 `conversations::get_or_create`）
- SQL 统一放 `db/queries/`，由领域模块调用
- **不必**：每表单独 `trait Repository` + mock；为简单 CRUD 再包 `BaseDao`；建中央 `api/dto/` 目录

### 4.2 模块依赖（硬性规则）

```
api → domain::{auth, conversations, messages, …}
domain → db
chat → persona, schedule, llm, memory, db, ws_hub
proactive → persona, schedule, llm, chat::delivery, push, db
db → 无上层依赖
```

- `db` 不得 import `chat`、`api` 等上层模块
- `api` 不得直接操作 `burst_buffers`、`reply_queues`（须经 `ChatService`）
- `api` 不直接调用 `llm`（须经 `chat` / `proactive`）
- `api` 不直接调用 `db::queries`（须经领域模块）

### 4.3 高内聚低耦合

- **按领域分模块**：会话逻辑在 `domain/conversations.rs`，消息逻辑在 `domain/messages.rs`，不散落在 handler 或 `From` 里
- **领域模块**聚合业务规则（幂等创建、归属校验、展示过滤、trim 语义等）；**仓储**只做 SQL
- **API Response 的 `From`** 只做协议映射（字段投影、URL 拼接、类型适配），**不做**业务决策
- 业务决策已在领域层完成时，`From` 输入应为领域 ViewModel（如 `ConversationListItem`），而非 raw SQL row
- 避免 `api` ↔ `db` 双向依赖；共享类型放领域层或 `db/models`，不为了省事跨层 import

### 4.4 可见性（避免过度 `pub`）

- **默认私有**；只在需要时放宽
- `lib.rs`：`pub mod domain`；`db` 用 `pub(crate)`，不对外暴露仓储细节
- Response struct：**字段私有**即可（`Serialize` 不要求字段 `pub`）；仅跨 handler 复用的 Response **类型**才 `pub`（如 `UserResponse`）
- 领域输入 struct（如 `RegisterInput`）为**纯数据**，不含 `Deserialize` / `Validate`；由 `api` 校验后经 `From` 传入
- DB model 的敏感/内部字段用 `pub(crate)`（如 `Message.metadata`）；读取经类型上的 helper（如 `media_url()`）
- `AppError` 等同理：优先私有字段 + getter，而非公开字段

### 4.5 所有权与 Clone

- 优先 **move** 或 **借用**（`&` / `&mut`），避免 `.clone()`
- 若确实需要 owned 值，由**最终需要所有权**的调用方负责 clone；不在中间 helper 里 preemptively clone
- `Arc::clone`、`pool.clone()` 等 cheap clone 除外

### 4.6 文件变长时的拆分

| 触发条件 | 做法 |
|----------|------|
| 单 REST 文件超 ~200 行 | Request/Response 抽到 `api/rest/<domain>/types.rs` |
| 单模块超 ~500 行 | 考虑抽 `crates/core` |

---

## 5. REST API 约定

### 5.1 统一响应格式

**所有 REST 端点**（含 `GET /health`）均使用 envelope：

| 场景 | HTTP | Body |
|------|------|------|
| 成功，有数据 | 200 / 201 | `{ "data": <object \| array \| string> }` |
| 成功，无 body | 200 | `{ "data": "ok" }` |
| 失败 | 4xx / 5xx | `{ "error": { "code": "...", "message": "..." } }` |

### 5.2 代码位置

| 类型 | 文件 | 职责 |
|------|------|------|
| `ApiResponse<T>` | `api/response.rs` | 成功响应；`ok()` / `created()` / `ok_empty()` |
| `ErrorBody` | `api/response.rs` | 错误 JSON 结构 |
| `error_code_status()` | `api/response.rs` | `ErrorCode` → HTTP 状态码映射 |
| `AppError` | `error.rs` | 错误语义；`IntoResponse` 在 `api/response.rs` |
| `ApiResult<T>` | `api/response.rs` | `Result<ApiResponse<T>, AppError>` 类型别名 |
| `ValidatedJson<T>` | `api/extractors.rs` | JSON 反序列化 + `validator` 校验 |

- `error.rs` 负责**错误语义**，不依赖 axum
- `api/response.rs` 负责**JSON 外形与 HTTP 映射**
- 成功路径显式返回 `ApiResponse::ok(...)` / `created(...)`；失败路径 `?` 传播 `AppError`
- 不为 `ApiResult` 自定义 `impl IntoResponse`（orphan rules）；依赖 Axum 内置 `Result` 实现

### 5.3 Base URL 与鉴权

- Base URL：`/api/v1`
- 除 `GET /health`、`POST /auth/register`、`POST /auth/login` 外，须 `Authorization: Bearer <token>`
- Token 查 `user_sessions` 解析 `user_id`；未授权返回 `401`

---

## 6. Request / Response 组织

### 6.1 校验与领域输入

**所有 API 输入参数均须校验**，无例外（含 JSON body、query、path、multipart、WebSocket 客户端消息等）。未校验的输入不得进入 handler 业务逻辑。

| 输入来源 | 传输层做法 |
|----------|------------|
| JSON body | `ValidatedJson<T>`，`T: Deserialize + Validate` |
| Query | `ValidatedQuery<T>`，`T: Deserialize + Validate` |
| Path / 其他 | 在 Request struct 或 handler 入口用 `validator` / 专用函数校验格式与范围 |
| WebSocket | 定义 typed payload，`Deserialize + Validate` 后再处理（非法帧返回错误或不执行） |

- **格式校验**（长度、范围、枚举、正则等）属于传输层：Request / Query struct 放在 `api/rest/`，derive `Deserialize` + `Validate`，经上述 extractor 在 handler 入口完成
- 校验通过后，用 `From<Request>` 转为领域 **Input**（纯数据 struct，无 serde/validator），再调领域函数
- 字段一致时 Input 与 Request 仍分属两层（协议 vs 领域），用 `From` 衔接，**不**维护 Params 式第三份镜像
- **业务校验**（trim 后为空、资源归属、幂等等）在领域层，不在 `validator` 里
- 仅当 API 与领域字段语义不一致时（如 API `u32`、DB `i32`），在 `From` 或领域入口做一次转换

### 6.2 Response 与 handler 位置

- 仅本 handler 使用的 Response：与 handler 同文件，**类型与字段默认私有**
- 命名：`XxxResponse`（Rust 惯例）；**不使用** `Dto` / `Vo` 后缀
- 跨 handler 复用（如 `UserResponse` 用于 auth 与 `/me`）：放 `api/rest/users.rs`，**类型** `pub` 导出
- **MVP 不建**中央 `api/dto/` 目录

### 6.3 与 DB model 的关系

- **禁止**把 `db/models` 直接 `Json()` 作为 API 响应
- 字段不一致时（隐藏 `password_hash`、拼接 `avatar_url` 等）：**必须**经 Response struct 映射

### 6.4 类型转换（`From` / `Into`）

- 在能用 `From` / `Into` 且没有较大负面影响的情况下，**优先使用** `From` / `Into`，而非手写 `into_xxx()` / `to_xxx()` 或逐字段 struct literal
- `impl From` 放在**转换目标类型**附近（如 `impl From<User> for UserResponse` 与 `UserResponse` 同文件）
- **`From` 仅做映射**，不做业务逻辑（不做 `filter`、幂等、trim 判定等）；这些在领域层完成后再 `into()`
- 仓储 `insert` / `update` 后应用 SELECT 读回 row，再 `From` 为 model；避免手写拼默认值

### 6.5 鉴权实现

- 公开路由（`/health`、`/auth/*`）与受保护路由分组；受保护组挂 `require_auth` 中间件（`api/middleware.rs`）
- 中间件校验 Bearer Token、查 `user_sessions`，将 `User` 注入 request extensions
- Handler 通过 `AuthUser` extractor（`api/extractors.rs`）获取当前用户

---

## 7. 认证与安全

| 项 | 约定 |
|----|------|
| 密码传输 | HTTPS 传明文；**不在前端预哈希** |
| 密码确认 | 前端二次确认即可；后端只收单个 `password` |
| 密码存储 | argon2id 哈希，存 `password_hash` |
| Token 格式 | `{prefix}{uuid}`（如 `doki_` + UUID） |
| Token 存储 | SHA-256 哈希后存 `user_sessions` |
| 随机数 | 使用 `rand_core` + `getrandom`（`rand 0.10` 无 `OsRng`） |

---

## 8. 数据类型约定

| 字段 | 类型 |
|------|------|
| `birthday` | `chrono::NaiveDate`，API 与 DB 均为 `Option<NaiveDate>` |
| `timezone` | IANA 字符串（如 `Asia/Shanghai`）；注册必填，可 `PATCH /me` 更新 |
| `display_name` | 注册时 `Option<String>`；空则默认 `username` |
| `max_proactive_per_day` | DB `i32`；API Response `u32` |
| `Message.metadata` | 字段 `pub(crate)`，仅 `db/queries` 可读写（sqlx）；读取须通过 `Message` 上的 helper（如 `media_url()`） |
| 绝对时刻（`created_at`、`read_at`、`dnd_*` 等） | `chrono::DateTime<Utc>`；DB `DATETIME(6)` 存 UTC 墙钟；API 为 RFC 3339（带 `Z`） |

### 8.1 时区约定

| 语义 | 约定 |
|------|------|
| **存储** | 所有绝对时刻按 UTC；MySQL 连接池 `after_connect` 执行 `SET time_zone = '+00:00'` |
| **传输** | REST / WebSocket 时间字段一律 UTC ISO 8601；展示由客户端转本地 |
| **日历** | `birthday` 等用 `DATE` / `NaiveDate`，不含时区 |
| **角色日程** | `characters.schedule_json.timezone`（IANA）；Schedule Engine 在该时区解析 `HH:MM` |
| **用户本地语义** | `users.timezone`（IANA）；勿扰、日上限、生日等按用户时区计算（`time` 模块） |
| **用户勿扰** | 存用户本地墙钟 `TIME`；服务端结合 `users.timezone` 判断 |
| **日上限 / 特殊日期** | 按 **用户时区自然日**（`time::user_day_bounds`） |

---

## 9. AppState

- 当前字段：`config` + `db: MySqlPool`
- `AppState::new()` 为 async：加载配置 → 连接 MySQL → 执行 `sqlx::migrate!`
- 其余字段（`ws_hub`、各 Service 等）**按需再补**，不提前堆砌

---

## 10. 开发流程

### 10.1 依赖管理

- 添加依赖使用 **`cargo add`**，不手写版本号

### 10.2 Git Commit

- 使用 **Conventional Commits**：`feat:`、`fix:`、`docs:`、`refactor:` 等
- 示例：`feat: add POST /auth/login`、`fix: resolve OsRng import with rand_core`

### 10.3 测试

- **每个 REST API 须有对应集成测试**（`tests/<domain>_api.rs`）
- 集成测试使用真实 MySQL，通过环境变量 `TEST_DATABASE_URL` 连接（如 `mysql://user:pass@127.0.0.1:3306/dokidoki_test`）
- 运行：`TEST_DATABASE_URL=... cargo test --test auth_api`
- 测试辅助代码放 `src/test_support.rs` 及子模块（如 `test_support/http.rs`）；**不使用** `mod.rs`
- 目标：新增 API 对应模块行覆盖率 **> 90%**（`cargo llvm-cov` 度量）
- MVP 阶段 auth 等 DB 密集型接口用集成测试；纯函数（hash/verify）可补单元测试

### 10.4 文档

- 需求、接口、架构细节以 `docs/` 为准
- 新增或变更**实现约定**时，同步更新本文件

---

## 11. 参考文档

| 文档 | 内容 |
|------|------|
| [需求分析说明书](docs/需求分析说明书.md) | 产品需求 |
| [概要设计说明书](docs/概要设计说明书.md) | 系统架构概览 |
| [详细设计说明书](docs/详细设计说明书.md) | 实现级设计（§2.3 分层与 API） |
| [接口设计说明书](docs/接口设计说明书.md) | REST/WS 契约 |
| [Prompt 规范](docs/Prompt规范.md) | LLM Prompt 模板 |
