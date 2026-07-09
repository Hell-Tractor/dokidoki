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
└── dokidoki-app/        # Flutter 客户端（待建）
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
| 传输层 | `api` | 路由、鉴权、序列化、调用领域服务 | 不写 burst、Prompt、状态机等业务逻辑 |
| 领域服务 | `auth`、`chat`、`proactive` 等 | 业务编排、领域规则 | 不直接拼 HTTP 响应；不散落 SQL |
| 仓储 | `db` | sqlx 查询、`models`、事务 | 不 import 上层模块 |
| 外部适配器 | `llm`、`push` | 第三方 API 封装 | 不含领域规则 |

### 4.1 Handler 约定

- Handler 保持薄（通常 < 30 行）：反序列化 Request → 调领域服务 → 映射 Response → 包进 `ApiResponse`
- SQL 统一放 `db/queries/`，按表或领域分文件
- **不必**：每表单独 `trait Repository` + mock；为简单 CRUD 再包 `BaseDao`；建中央 `api/dto/` 目录

### 4.2 模块依赖（硬性规则）

```
api → auth, chat, proactive, db, push, schedule, persona  （不直接调 llm）
auth → db
chat → persona, schedule, llm, memory, db, ws_hub
proactive → persona, schedule, llm, chat::delivery, push, db
db → 无上层依赖
```

- `db` 不得 import `chat`、`api` 等上层模块
- `api` 不得直接操作 `burst_buffers`、`reply_queues`（须经 `ChatService`）
- `api` 不直接调用 `llm`（须经 `chat` / `proactive`）

### 4.3 文件变长时的拆分

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

### 6.1 默认：与 handler 同文件

- 放在 `api/rest/<domain>.rs`，如 `RegisterRequest`、`AuthResponse` 定义在 `api/rest/auth.rs`
- 命名：`XxxRequest` / `XxxResponse`（Rust 惯例）
- 代码中**不使用** `Dto` / `Vo` 后缀

### 6.2 跨 handler 复用

- 如 `UserResponse` 同时用于 auth 与 `/me`：放到 `api/rest/users.rs` 并 `pub` 导出
- **MVP 不建**中央 `api/dto/` 目录

### 6.3 与 DB model 的关系

- API 字段与表结构一致时：用 `From` / `Into` 转换
- 字段不一致时（隐藏 `password_hash`、拼接 `avatar_url` 等）：**必须**经 Response struct 映射
- **禁止**把 `db/models` 直接 `Json()` 作为 API 响应

### 6.4 类型转换

- 在能用 `From` / `Into` 且没有较大负面影响的情况下，**优先使用** `From` / `Into`，而非手写 `into_xxx()` / `to_xxx()`
- `impl From` 放在被转换目标类型附近（如 `impl From<User> for UserResponse` 与 `UserResponse` 同文件）

### 6.5 待办（已知例外）

- `UserResponse` 暂留 `api/rest/auth.rs`；实现 `GET /me` 时再迁移到 `api/rest/users.rs`

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
| `display_name` | 注册时 `Option<String>`；空则默认 `username` |
| `max_proactive_per_day` | DB `i32`；API Response `u32` |

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
