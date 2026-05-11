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
  "password": "ruri"
}
```

**响应：** 在响应头中设置会话 Cookie。

### 登出

```
POST /api/auth/logout
```

## 聊天

### 发送聊天消息

```
POST /api/chat
```

**请求体：**

```json
{
  "message": "你好，最近怎么样？",
  "conversation_id": "optional-conversation-id"
}
```

### 获取聊天历史

```
GET /api/chat/history
```

### 清除聊天历史

```
DELETE /api/chat/history
```

## 对话

### 列出对话

```
GET /api/conversations
```

### 创建对话

```
POST /api/conversations
```

**请求体：**

```json
{
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

### 获取对话消息

```
GET /api/conversations/:id/messages
```

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
  "api_url": "https://api.openai.com/v1",
  "api_key": "sk-...",
  "model": "gpt-4o"
}
```

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

**请求：** 包含 `file` 字段的多部分表单数据，字段包含 ZIP 压缩包。

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

返回智能体的当前状态，包括是否正在处理请求。

## ACP

### 获取 ACP 配置

```
GET /api/acp/config
```

### 更新 ACP 配置

```
PUT /api/acp/config
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

## 身份验证端点

```
POST /api/auth/login    — 登录
POST /api/auth/logout   — 登出
```

可能还有用于密码修改和用户名更新的额外身份验证端点。

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

| 状态码 | 描述                             |
| ------ | -------------------------------- |
| 200    | 成功                             |
| 201    | 已创建                           |
| 400    | 请求错误 — 输入无效              |
| 401    | 未授权 — 需要身份验证            |
| 403    | 禁止访问 — 权限不足              |
| 404    | 未找到                           |
| 500    | 内部服务器错误                   |

## 身份验证详情

Ruri 使用基于 Cookie 的会话身份验证：

1. **登录** — 使用凭据调用 `POST /api/auth/login`
2. **会话 Cookie** — 在响应中返回，自动包含在后续请求中
3. **默认凭据** — `ruri` / `ruri`（首次登录后修改）
4. **密码修改** — 初始登录后支持
5. **用户名更新** — 通过身份验证端点支持

::: tip
使用 `curl` 时，使用 `-H "Cookie: session=<value>"` 包含会话 Cookie。使用浏览器或 HTTP 客户端库时，Cookie 通常会自动处理。
:::
