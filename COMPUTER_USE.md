# Computer Use 功能

Computer Use（电脑能力）功能让 Agent 可以在 Ruri 运行环境中执行代码、访问文件、调用 Shell。

## 功能概述

Computer Use 提供：

- **Shell 工具**：执行 shell 命令
- **Python 工具**：执行 Python 代码
- **文件系统工具**：读写文件、目录操作
- **权限管理**：管理员和普通用户的权限分离
- **Workspace 隔离**：每个会话独立的 workspace

## 配置

### 配置项

在 `data/config.json` 中配置：

```json
{
  "computer_use_config": {
    "runtime": "local",
    "require_admin": true,
    "admin_ids": ["user1", "user2"],
    "allowed_paths": ["/path/to/allowed/dir"],
    "sandbox_config": null
  }
}
```

### 运行时模式

- `none`：不启用电脑能力
- `local`：在本地环境执行（推荐）
- `sandbox`：在隔离沙盒中执行（未来支持）

### 权限配置

- `require_admin`：是否需要管理员权限才能使用 Shell/Python 工具（默认：`true`）
- `admin_ids`：管理员用户 ID 列表
- `allowed_paths`：非管理员用户可访问的额外路径

## API 端点

### 获取配置

```bash
GET /api/computer-use/config
```

响应示例：

```json
{
  "runtime": "local",
  "require_admin": true,
  "admin_ids": ["user1"],
  "allowed_paths": [],
  "sandbox_config": null
}
```

### 更新配置

```bash
PUT /api/computer-use/config
```

请求体：

```json
{
  "runtime": "local",
  "require_admin": false,
  "admin_ids": ["user1", "user2"]
}
```

## Workspace 管理

每个会话都有独立的 workspace，路径为：

```
data/workspaces/{session_id}
```

其中 `session_id` 会被规范化（将不适合作为文件名的字符替换为 `_`）。

例如：
- 会话 ID `user/session:123` → workspace 路径 `data/workspaces/user_session_123`

## 权限模型

### 管理员权限

管理员可以：
- 使用 Shell 工具执行任意命令
- 使用 Python 工具执行任意代码
- 访问任意路径的文件
- 使用绝对路径

### 普通用户权限

普通用户可以：
- 使用文件系统工具（受限目录）
- 仅使用相对路径（相对于当前会话的 workspace）
- 访问以下目录：
  - `data/skills` - 技能目录
  - `data/workspaces/{session_id}` - 当前会话的 workspace
  - 系统临时目录中的 `.ruri` 文件夹
  - 配置的 `allowed_paths`

普通用户**不能**：
- 使用 Shell 工具
- 使用 Python 工具
- 使用绝对路径

## 工具

### Shell 工具

执行 shell 命令，命令会在 workspace 目录下执行。

```json
{
  "name": "shell",
  "arguments": {
    "command": "ls -la",
    "timeout": 30
  }
}
```

### Python 工具

执行 Python 代码，代码会在 workspace 目录下执行。

```json
{
  "name": "python",
  "arguments": {
    "code": "print('Hello, World!')",
    "timeout": 60
  }
}
```

## 使用示例

### 1. 启用电脑能力（管理员）

```bash
curl -X PUT http://localhost:3000/api/computer-use/config \
  -H "Content-Type: application/json" \
  -d '{
    "runtime": "local",
    "require_admin": true,
    "admin_ids": ["my_user_id"]
  }'
```

### 2. 在聊天中使用

发送消息时，Agent 会根据配置自动使用相应的工具：

```
用户: 帮我列出当前目录的文件
Agent: [使用 list_directory 工具列出 data/workspaces/{session_id} 目录]
```

管理员用户可以执行更强大的操作：

```
用户: 运行 npm install
Agent: [使用 shell 工具在 workspace 目录执行 npm install]
```

## 安全建议

1. **默认配置是安全的**：默认情况下，电脑能力是禁用的（`runtime: "none"`）
2. **限制管理员权限**：只在需要时将用户添加到 `admin_ids`
3. **审查 allowed_paths**：谨慎配置额外允许访问的路径
4. **监控日志**：查看工具调用的日志，了解 Agent 执行了哪些操作

## 未来功能

- [ ] 沙盒模式（Sandbox runtime）
- [ ] 浏览器自动化工具
- [ ] 更细粒度的权限控制
- [ ] 危险命令拦截
- [ ] 资源使用限制

## 参考资料

设计参考了 [AstrBot 的电脑能力](https://docs.astrbot.app/use/computer.html) 功能。
