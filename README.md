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
| 数据库 | MySQL |
| 客户端 | Flutter（Android 优先） |
| 部署 | Docker Compose · Caddy |

## 项目结构

```
dokidoki/
├── docs/                    # 需求与设计文档
├── dokidoki-server/         # Rust 后端（开发中）
│   └── migrations/          # 数据库迁移
└── dokidoki-app/            # Flutter 客户端（待建）
```

## 文档

- [需求分析说明书](docs/需求分析说明书.md)
- [概要设计说明书](docs/概要设计说明书.md)
- [详细设计说明书](docs/详细设计说明书.md)
- [接口设计说明书](docs/接口设计说明书.md)
- [Prompt 规范](docs/Prompt规范.md)

## 状态

当前处于设计与文档阶段，代码实现尚未开始。部署说明、配置示例与开发指南将在 MVP 完成后补充。

## License

待定
