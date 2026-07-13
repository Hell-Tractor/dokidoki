# 开发种子数据

**不要**放进 `migrations/`；生产环境按需手动执行。

## 角色

| 文件 | 说明 |
|------|------|
| `dev_characters.sql` | 默认角色「小爱」（原型：《常轨脱离Creative》和泉妃爱） |

固定 ID：`00000000-0000-4000-8000-000000000001`

头像文件在 `assets/avatars/`；服务端启动时会同步到 `{upload.dir}/avatars/`（已存在则跳过）。种子 SQL 中 `avatar_path` 为相对路径，如 `avatars/{id}.png`。

## 执行

```bash
mysql -h 127.0.0.1 -u dokidoki -p dokidoki < seeds/dev_characters.sql
```

或使用 `TEST_DATABASE_URL` 中的库：

```bash
mysql "$TEST_DATABASE_URL" < seeds/dev_characters.sql
```

可重复执行（`ON DUPLICATE KEY UPDATE`）。
