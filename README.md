# Dokidoki

基于 LLM 的角色扮演聊天应用。不是 AI 助手，而是「有性格、有日程、会主动找你的虚拟角色」。

面向**技术用户自部署**：在 VPS 上运行后端，通过 Flutter 客户端连接使用。

## 特性（规划）

- 性格化对话与情绪陪伴，维持角色一致性
- 角色日程与活动状态，支持主动发消息
- 贴近真实 IM 的聊天节奏（连发合并、多气泡回复、正在输入）
- 文本与图片对话；架构预留 TTS

## 技术栈

| 组件 | 技术 |
|------|------|
| 后端 | Rust · Axum · Tokio · sqlx |
| 数据库 | MySQL 8.0+ |
| 客户端 | Flutter（Android / Web / 桌面均可） |
| 部署 | 当前：直接运行；规划：Docker Compose · Caddy |

## 项目结构

```
dokidoki/
├── docs/                    # 需求与设计文档
├── dokidoki-server/         # Rust 后端
│   ├── config.toml.example  # 配置模板
│   ├── migrations/          # 数据库迁移（启动时自动执行）
│   └── seeds/               # 可选开发种子数据
└── dokidoki-app/            # Flutter 客户端
```

## 本地部署

### 依赖

| 组件 | 版本建议 | 安装 |
|------|----------|------|
| Rust | stable（edition 2021） | https://rustup.rs |
| Flutter | 稳定版 SDK | https://docs.flutter.dev/get-started/install |
| MySQL | 8.0+ | 本机安装或 Docker |

确认工具可用：

```bash
rustc --version && cargo --version
flutter doctor
mysql --version
```

### 1. 准备数据库

```sql
CREATE DATABASE dokidoki CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER 'dokidoki'@'%' IDENTIFIED BY 'your_password';
GRANT ALL PRIVILEGES ON dokidoki.* TO 'dokidoki'@'%';
FLUSH PRIVILEGES;
```

### 2. 配置并启动后端

```bash
cd dokidoki-server
cp config.toml.example config.toml
# 编辑 config.toml（见下方必改项）
mkdir -p data/uploads          # 与 upload.dir 对应；示例默认是 /data/uploads
cargo run                      # 开发；生产可用 cargo run --release
```

服务默认监听 `0.0.0.0:8080`。启动时会自动跑 `migrations/`，无需单独 migrate 命令。

可选：导入开发用角色种子：

```bash
mysql -u dokidoki -p dokidoki < seeds/dev_characters.sql
```

#### `config.toml` 必须修改的字段

从 `config.toml.example` 复制后，至少改这些：

| 字段 | 说明 |
|------|------|
| `[database].url` | MySQL 连接串，如 `mysql://dokidoki:your_password@127.0.0.1:3306/dokidoki` |
| `[llm].mode` | `fake`（本地联调）或 `http`（真实模型） |
| `[llm].base_url` / `api_key` / `model` | `mode = "http"` 时填写；兼容 OpenAI 风格 Chat Completions |
| `[llm].vision_model` | 带图对话用的模型名（可与 `model` 相同） |
| `[upload].dir` | 上传目录，需可写；本机可改为 `./data/uploads` |
| `[server].host` / `port` | 按需改绑定地址与端口 |

其余（`auth`、`chat`、`summary`、`proactive`、`push` 等）可先沿用示例值。`push.fcm_credentials_path` 当前可保留占位路径。

日志目录为运行目录下的 `logs/`；可用 `RUST_LOG=dokidoki_server=debug` 提高详细度。

### 3. 启动 Flutter 客户端

```bash
cd dokidoki-app
flutter pub get
flutter run                 # 交互选择设备
# 或：flutter run -d chrome / -d <android-device-id>
```

首次进入会要求填写**服务端地址**（写入本地，不是编译期配置），例如：

- 本机 / 模拟器：`http://127.0.0.1:8080`
- 真机同局域网：`http://192.168.x.x:8080`
- Chrome 调试：同上；跨域已由服务端 CORS 处理

然后注册或登录即可。API 前缀为 `/api/v1`，WebSocket 为 `/api/v1/ws`。

## 文档

- [需求分析说明书](docs/需求分析说明书.md)
- [概要设计说明书](docs/概要设计说明书.md)
- [详细设计说明书](docs/详细设计说明书.md)
- [接口设计说明书](docs/接口设计说明书.md)
- [Prompt 规范](docs/Prompt规范.md)

## License

本项目采用 [GNU Affero General Public License v3.0](LICENSE)（AGPL-3.0）。

若你修改本软件并通过网络提供服务，必须向该服务的用户提供对应完整源码。详见仓库根目录 [`LICENSE`](LICENSE)。
