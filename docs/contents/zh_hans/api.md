---
layout: doc
title: "API 参考"
lastUpdated: true
---

# API 参考

Ruri 提供了全面的 REST API，用于对所有功能进行编程访问。所有端点相对于基础 URL（默认：`http://localhost:3000`）。

## 身份验证

大多数 API 端点需要通过会话 Cookie 进行身份验证。请先登录以获取身份验证。

### 登录

```
POST /api/auth/login
```

**请求体：**

```json
{
  "username": "ruri",
  "password": "ruri",
  "remember_me": false
}
```

**响应：** 响应头中会设置会话 Cookie，同时返回 JSON 响应体：

```json
{
  "token": "session-token",
  "user": {
    "id": "user-id",
    "username": "ruri",
    "must_change_password": true,
    "avatar_url": null
  }
}
```

### 登出

```
POST /api/auth/logout
```

### 获取当前用户

```
GET /api/auth/me
```

返回当前已认证用户的信息，包括用户 ID、用户名、头像 URL 以及是否需要修改密码。

### 修改密码

```
POST /api/auth/change-password
```

**请求体：**

```json
{
  "old_password": "ruri",
  "new_password": "my-new-password"
}
```

### 更新用户名

```
PUT /api/auth/username
```

**请求体：**

```json
{
  "new_username": "my-new-username"
}
```

### 上传头像

```
POST /api/auth/avatar
```

**请求：** 包含图像文件的 multipart 表单数据。支持的格式：PNG、JPEG、GIF、WebP。最大大小：2MB。

### 获取头像

```
GET /api/auth/avatar/:user_id
```

返回指定用户的头像图片。

## 聊天

### 发送聊天消息

```
POST /api/chat
```

**请求体：**

```json
{
  "message": "你好，最近怎么样？",
  "conversation_id": "optional-conversation-id",
  "images": [],
  "files": [],
  "provider_id": "optional-provider-id",
  "session_id": "optional-session-id",
  "temperature": 0.7,
  "max_tokens": 4096,
  "knowledge_base_ids": [],
  "tool_choice": "auto",
  "parallel_tool_calls": true
}
```

| 字段                  | 类型     | 说明                                    |
| --------------------- | -------- | --------------------------------------- |
| `message`             | string   | 聊天消息文本                            |
| `conversation_id`     | string?  | 继续已有对话                            |
| `images`              | array    | Base64 编码的图片，用于多模态模型       |
| `files`               | array    | 附件文件（PDF、DOCX、XLSX、TXT 等）     |
| `provider_id`         | string?  | 覆盖本次请求的活跃提供商                |
| `session_id`          | string?  | 会话标识符                              |
| `temperature`         | number?  | 模型温度（0-2）                         |
| `max_tokens`          | number?  | 响应的最大 token 数                     |
| `knowledge_base_ids`  | array    | 要搜索的知识库                          |
| `tool_choice`         | string?  | `auto`、`none`、`required` 或指定函数名 |
| `parallel_tool_calls` | boolean? | 模型是否可以并行调用多个工具            |

### 获取聊天历史

```
GET /api/chat/history
```

### 清除聊天历史

```
DELETE /api/chat/history
```

### 停止聊天生成

```
POST /api/chat/stop
```

**请求体：**

```json
{
  "session_id": "session-to-stop"
}
```

停止指定会话中正在进行的聊天生成。

## 对话

详见[聊天记录](/zh_hans/chat-history)页面，了解对话系统的完整说明。

### 列出对话

```
GET /api/conversations
```

**查询参数（可选）：**

| 参数        | 说明                                             |
| ----------- | ------------------------------------------------ |
| `bot_name`  | 按机器人名称筛选                                 |
| `chat_type` | 按聊天类型筛选：`group` 或 `private`             |
| `keyword`   | 搜索对话标题和聊天 ID 中包含的关键词（模糊匹配） |

### 创建对话

```
POST /api/conversations
```

**请求体：**

```json
{
  "bot_name": "my-bot",
  "chat_type": "private",
  "chat_id": "user-123",
  "title": "我的对话"
}
```

### 获取对话

```
GET /api/conversations/:id
```

### 删除对话

```
DELETE /api/conversations/:id
```

删除对话会**级联删除**该对话下的所有消息。成功时返回 `204 No Content`。

### 向对话添加消息

```
POST /api/conversations/:id/messages
```

**请求体：**

```json
{
  "role": "user",
  "content": "你好！"
}
```

添加消息时，对话的 `updated_at` 时间戳会自动更新。

### 获取对话消息

```
GET /api/conversations/:id/messages
```

返回对话中的所有消息，按 `created_at` 升序排列。

## 提供商

### 列出提供商

```
GET /api/providers
```

### 创建提供商

```
POST /api/providers
```

**请求体：**

```json
{
  "name": "My Provider",
  "provider_type": "openai_compatible",
  "config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "default_model": "gpt-4o",
    "supports_multimodal": true
  }
}
```

提供商类型：`openai_compatible`、`anthropic`、`gemini`。

### 获取提供商

```
GET /api/providers/:id
```

### 更新提供商

```
PUT /api/providers/:id
```

**请求体：** 与创建相同，包含要更新的字段。

### 删除提供商

```
DELETE /api/providers/:id
```

### 激活提供商

```
POST /api/providers/:id/activate
```

将指定提供商设置为所有聊天交互的活跃提供商。

### 获取提供商模型列表

```
POST /api/providers/fetch-models
```

查询提供商的 API 以列出所有可用模型。

**请求体：**

```json
{
  "provider_type": "openai_compatible",
  "base_url": "https://api.openai.com/v1",
  "api_key": "sk-..."
}
```

**响应：**

```json
{
  "models": [
    { "id": "gpt-4o", "name": "GPT-4o" },
    { "id": "gpt-4o-mini", "name": "GPT-4o Mini" }
  ]
}
```

## 技能

### 列出技能

```
GET /api/skills
```

### 添加技能

```
POST /api/skills
```

**请求体：**

```json
{
  "name": "my-skill",
  "content": "---\nname: my-skill\ndescription: 我的技能\n---\n技能指令在这里。"
}
```

### 上传技能包

```
POST /api/skills/upload
```

**请求：** 包含 `file` 字段的 multipart 表单数据，字段包含 ZIP 压缩包。

```bash
curl -X POST http://localhost:3000/api/skills/upload \
  -H "Cookie: session=<your-session-cookie>" \
  -F "file=@skills.zip"
```

### 切换技能

```
PATCH /api/skills/:name
```

**请求体：**

```json
{
  "enabled": true
}
```

### 删除技能

```
DELETE /api/skills/:name
```

## 工具

### 列出工具

```
GET /api/tools
```

返回所有可用工具的列表，包括内置工具和 MCP 提供的工具。

## 智能体

### 获取智能体状态

```
GET /api/agent/status
```

返回智能体的当前状态，包括：

- 是否正在处理请求
- 活跃提供商和模型
- 技能和工具数量
- 运行时长（秒）
- 消息数量

## ACP

### 获取 ACP 配置

```
GET /api/acp/config
```

### 更新 ACP 配置

```
PUT /api/acp/config
```

**请求体：**

```json
{
  "active_provider_id": "provider-id",
  "active_skill_names": ["code-review", "summarize"],
  "active_knowledge_base_ids": ["kb-id"],
  "proxy_config": {}
}
```

## 计算机使用

### 获取计算机使用配置

```
GET /api/computer-use/config
```

### 更新计算机使用配置

```
PUT /api/computer-use/config
```

**请求体：**

```json
{
  "runtime": "aio_sandbox",
  "require_admin": true,
  "admin_ids": ["user-1"],
  "allowed_paths": ["/safe/path"],
  "command_admin_required": { "reset": false },
  "shell_command_blacklist": ["sudo ", "rm -rf"],
  "aio_sandbox_config": {
    "endpoint": "http://localhost:8080"
  }
}
```

### 获取 Shell 命令黑名单

```
GET /api/computer-use/shell-blacklist
```

### 更新 Shell 命令黑名单

```
PUT /api/computer-use/shell-blacklist
```

**请求体：**

```json
{
  "blacklist": ["sudo ", "rm -rf", "format "]
}
```

## 网页搜索

### 获取网页搜索配置

```
GET /api/web-search/config
```

### 更新网页搜索配置

```
PUT /api/web-search/config
```

**请求体：**

```json
{
  "search_engine": "duckduckgo",
  "api_key": null,
  "max_results": 10,
  "enabled": true
}
```

## 配置方案

### 列出配置方案

```
GET /api/profiles
```

### 获取配置方案

```
GET /api/profiles/:id
```

### 创建配置方案

```
POST /api/profiles
```

**请求体：**

```json
{
  "name": "编程",
  "description": "用于开发工作",
  "enable": true,
  "provider_id": "provider-id",
  "persona_id": "persona-id",
  "web_search_enabled": true,
  "computer_use_enabled": true,
  "active_skill_names": ["code-review"],
  "active_knowledge_base_ids": [],
  "proxy_config": {},
  "command_prefix": "/",
  "enabled_commands": ["help", "new", "reset"],
  "command_admin_required": {},
  "custom_error_message": null,
  "platform_ids": []
}
```

### 更新配置方案

```
PUT /api/profiles/:id
```

### 删除配置方案

```
DELETE /api/profiles/:id
```

### 激活配置方案

```
POST /api/profiles/:id/activate
```

### 停用配置方案

```
POST /api/profiles/:id/deactivate
```

### 获取配置方案的已解析提供商

```
GET /api/profiles/:id/provider
```

返回配置方案解析后的提供商，包括内嵌的提供商。

## 人格

```
GET    /api/personas           — 列出人格
GET    /api/personas/:id       — 获取人格
POST   /api/personas           — 创建人格
PUT    /api/personas/:id       — 更新人格
DELETE /api/personas/:id       — 删除人格
```

**创建/更新请求体：**

```json
{
  "name": "代码专家",
  "description": "一位资深软件工程师",
  "prompt": "你是一位资深软件工程师……"
}
```

## 平台

平台 CRUD 端点遵循标准模式：

```
GET    /api/platforms           — 列出平台
POST   /api/platforms           — 创建平台
GET    /api/platforms/:id       — 获取平台
PUT    /api/platforms/:id       — 更新平台
DELETE /api/platforms/:id       — 删除平台
```

### 重启平台

```
POST /api/platforms/:id/restart
```

重启指定的平台适配器，无需重启整个 Ruri 服务器。

### 微信扫码登录

```
POST /api/platforms/weixin-qr/start    — 启动扫码登录流程
GET  /api/platforms/weixin-qr/status   — 查询扫码登录状态
```

## MCP 服务器

```
GET    /api/mcp/servers         — 列出 MCP 服务器
GET    /api/mcp/servers/:id     — 获取 MCP 服务器
POST   /api/mcp/servers         — 创建 MCP 服务器
PUT    /api/mcp/servers/:id     — 更新 MCP 服务器
DELETE /api/mcp/servers/:id     — 删除 MCP 服务器
PATCH  /api/mcp/servers/:id     — 切换 MCP 服务器的启用/禁用状态
```

**创建/更新请求体：**

```json
{
  "name": "my-mcp-server",
  "transport_type": "stdio",
  "transport_config": {
    "type": "stdio",
    "command": "node",
    "args": ["./my-server.js"],
    "env": {}
  },
  "enabled": true
}
```

## 知识库

```
GET    /api/knowledge-bases                      — 列出知识库
GET    /api/knowledge-bases/:id                  — 获取知识库
POST   /api/knowledge-bases                      — 创建知识库
PUT    /api/knowledge-bases/:id                  — 更新知识库
DELETE /api/knowledge-bases/:id                  — 删除知识库
GET    /api/knowledge-bases/:id/documents         — 列出文档
POST   /api/knowledge-bases/:id/documents/upload  — 上传文档
DELETE /api/knowledge-bases/:id/documents/:doc_id — 删除文档
POST   /api/knowledge-bases/search               — 搜索知识库
```

**创建知识库请求体：**

```json
{
  "name": "我的知识库",
  "description": "项目文档",
  "embedding_provider_config": {
    "base_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "text-embedding-3-small",
    "dimension": 1536
  },
  "rerank_provider_config": {
    "base_url": "https://api.example.com",
    "api_key": "key",
    "model": "rerank-model"
  },
  "chunk_size": 500,
  "chunk_overlap": 50
}
```

**搜索请求体：**

```json
{
  "query": "退货政策是什么？",
  "top_k": 5
}
```

## 命令

### 列出内置命令

```
GET /api/commands
```

返回所有可用的斜杠命令及其元数据（名称、描述、用法、是否需要管理员、启用状态）。

### 切换命令的管理员要求

```
PATCH /api/commands/:name/admin
```

**请求体：**

```json
{
  "require_admin": true
}
```

## 系统

### 重启系统

```
POST /api/system/restart
```

重启 Ruri 服务器。需要管理员权限。

## 调试会话

### 获取调试会话

```
GET /api/debug-session
```

返回当前的调试会话配置，包括提供商、技能、人格和其他设置。用于开发和测试。

### 更新调试会话

```
PUT /api/debug-session
```

更新调试会话配置。接受部分调试会话配置对象。

## WebSocket

### 实时日志

```
WS /api/ws/logs
```

连接到此 WebSocket 端点以从 LogManager 接收实时日志消息。这与 Web UI 日志查看器中显示的数据相同。

**消息格式：**

```json
{
  "level": "info",
  "message": "智能体开始处理",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## 错误响应

所有端点以以下格式返回错误：

```json
{
  "error": "描述错误的消息"
}
```

常见 HTTP 状态码：

| 状态码 | 描述                  |
| ------ | --------------------- |
| 200    | 成功                  |
| 201    | 已创建                |
| 204    | 无内容（删除成功）    |
| 400    | 请求错误 — 输入无效   |
| 401    | 未授权 — 需要身份验证 |
| 403    | 禁止访问 — 权限不足   |
| 404    | 未找到                |
| 500    | 内部服务器错误        |

## 身份验证详情

Ruri 使用基于 Cookie 的会话身份验证：

1. **登录** — 使用凭据调用 `POST /api/auth/login`
2. **会话 Cookie** — 在响应中返回，自动包含在后续请求中
3. **默认凭据** — `ruri` / `ruri`（首次登录后修改）
4. **密码修改** — `POST /api/auth/change-password`
5. **用户名更新** — `PUT /api/auth/username`
6. **头像上传** — `POST /api/auth/avatar`（multipart，最大 2MB）

::: tip
使用 `curl` 时，使用 `-H "Cookie: session=<value>"` 包含会话 Cookie。使用浏览器或 HTTP 客户端库时，Cookie 通常会自动处理。
:::
