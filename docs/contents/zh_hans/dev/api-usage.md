---
layout: doc
title: "API 使用指南"
lastUpdated: true
---

# API 使用指南

本指南将向你展示如何以编程方式使用 Ruri 的 REST API——从认证和发送请求，到通过 WebSocket 处理实时日志。完整的端点参考请参见 [API 参考](/api)。

## 基础 URL

所有 API 请求发送到：

```
http://localhost:3000/api
```

启动 Ruri 时可通过 `--port` 参数更改端口。

## 认证

Ruri 使用基于 Cookie 的会话认证。你必须先登录以获取会话 Cookie，然后在所有后续请求中包含该 Cookie。

### 登录流程

1. **发送登录请求**，附带你的凭据
2. **在响应中接收会话 Cookie**
3. **在所有后续 API 请求中包含该 Cookie**

::: warning
默认凭据为 `ruri` / `ruri`。首次登录后请务必修改密码以确保安全。
:::

### Python 示例

```python
import requests

BASE_URL = "http://localhost:3000/api"

# 创建会话 — 自动存储和发送 Cookie
session = requests.Session()

# 步骤 1：登录
response = session.post(f"{BASE_URL}/auth/login", json={
    "username": "ruri",
    "password": "ruri"
})

if response.status_code == 200:
    print("登录成功！")
    # 会话 Cookie 现在已存储在 `session.cookies` 中
else:
    print(f"登录失败：{response.json()}")

# 步骤 2：发送已认证的请求 — Cookie 会自动发送
providers = session.get(f"{BASE_URL}/providers")
print(providers.json())
```

### JavaScript 示例（fetch）

```javascript
const BASE_URL = "http://localhost:3000/api";

// 步骤 1：登录 — 浏览器会自动存储会话 Cookie
const loginRes = await fetch(`${BASE_URL}/auth/login`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ username: "ruri", password: "ruri" }),
  credentials: "include", // 重要：包含 Cookie
});

if (loginRes.ok) {
  console.log("登录成功！");
}

// 步骤 2：发送已认证的请求
const providersRes = await fetch(`${BASE_URL}/providers`, {
  credentials: "include", // 包含会话 Cookie
});
const providers = await providersRes.json();
console.log(providers);
```

### JavaScript 示例（axios）

```javascript
import axios from "axios";

const client = axios.create({
  baseURL: "http://localhost:3000/api",
  withCredentials: true, // 每次请求都发送 Cookie
});

// 步骤 1：登录
await client.post("/auth/login", {
  username: "ruri",
  password: "ruri",
});

// 步骤 2：发送已认证的请求 — Cookie 自动发送
const { data: providers } = await client.get("/providers");
console.log(providers);
```

### 使用 curl

```bash
# 登录并保存会话 Cookie
curl -c cookies.txt -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"ruri","password":"ruri"}'

# 使用保存的 Cookie 发送后续请求
curl -b cookies.txt http://localhost:3000/api/providers
```

::: tip
在 Python 中使用 `requests.Session()` 或在 JavaScript 中使用带 `withCredentials: true` 的 `axios` 时，Cookie 会自动处理，无需手动提取和设置 Cookie 头。
:::

## 常用 API 模式

### CRUD 操作

大多数 Ruri 资源（提供商、技能、平台）遵循标准的 REST CRUD 模式：

| 操作 | 方法 | 端点 | 请求体 |
| --- | --- | --- | --- |
| 列出所有 | `GET` | `/api/{resource}` | — |
| 获取单个 | `GET` | `/api/{resource}/:id` | — |
| 创建 | `POST` | `/api/{resource}` | JSON 请求体 |
| 更新 | `PUT` | `/api/{resource}/:id` | JSON 请求体 |
| 删除 | `DELETE` | `/api/{resource}/:id` | — |

#### 示例：管理提供商

```python
# 列出所有提供商
providers = session.get(f"{BASE_URL}/providers").json()

# 创建新的提供商
new_provider = session.post(f"{BASE_URL}/providers", json={
    "name": "My OpenAI",
    "provider_type": "openai_compatible",
    "api_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "gpt-4o"
}).json()

# 激活提供商
session.post(f"{BASE_URL}/providers/{new_provider['id']}/activate")

# 更新提供商
session.put(f"{BASE_URL}/providers/{new_provider['id']}", json={
    "name": "Updated Name",
    "model": "gpt-4o-mini"
})

# 删除提供商
session.delete(f"{BASE_URL}/providers/{new_provider['id']}")
```

::: info
如果你使用国内模型提供商（如 DeepSeek），`api_url` 应设置为对应的 API 地址，例如 DeepSeek 为 `https://api.deepseek.com/v1`。
:::

#### 示例：管理技能

```python
# 列出所有技能
skills = session.get(f"{BASE_URL}/skills").json()

# 添加新技能
session.post(f"{BASE_URL}/skills", json={
    "name": "code-review",
    "content": "---\nname: code-review\ndescription: Review code\n---\nYou are a code reviewer."
})

# 开启/关闭技能
session.patch(f"{BASE_URL}/skills/code-review", json={
    "enabled": True
})

# 删除技能
session.delete(f"{BASE_URL}/skills/code-review")
```

#### 示例：管理平台

```python
# 列出平台
platforms = session.get(f"{BASE_URL}/platforms").json()

# 创建 Discord 平台
session.post(f"{BASE_URL}/platforms", json={
    "platform_type": "discord",
    "token": "your-bot-token",
    "application_id": "your-app-id"
})

# 重启平台适配器
session.post(f"{BASE_URL}/platforms/{platform_id}/restart")
```

### 聊天 API

聊天端点是与 AI 代理交互的主要方式：

```python
# 发送消息
response = session.post(f"{BASE_URL}/chat", json={
    "message": "解释一下 Rust 的借用检查器",
    "conversation_id": None  # 可选：继续已有的对话
}).json()

# 获取聊天历史
history = session.get(f"{BASE_URL}/chat/history").json()

# 清除聊天历史
session.delete(f"{BASE_URL}/chat/history")
```

JavaScript 等效代码：

```javascript
// 发送消息
const response = await client.post("/chat", {
  message: "解释一下 Rust 的借用检查器",
  conversation_id: null,
});

// 获取聊天历史
const history = await client.get("/chat/history");

// 清除聊天历史
await client.delete("/chat/history");
```

### 对话 API

用于更结构化的对话管理：

```python
# 创建新对话
conv = session.post(f"{BASE_URL}/conversations", json={
    "title": "Rust 学习"
}).json()

conversation_id = conv["id"]

# 向对话中添加消息
session.post(f"{BASE_URL}/conversations/{conversation_id}/messages", json={
    "role": "user",
    "content": "Rust 的所有权规则是什么？"
})

# 获取对话中的所有消息
messages = session.get(f"{BASE_URL}/conversations/{conversation_id}/messages").json()

# 列出所有对话
conversations = session.get(f"{BASE_URL}/conversations").json()

# 删除对话
session.delete(f"{BASE_URL}/conversations/{conversation_id}")
```

### 代理状态

检查代理是否正在处理请求：

```python
status = session.get(f"{BASE_URL}/agent/status").json()
print(status)
```

### 工具列表

获取所有可用工具的列表，包括内置工具和 MCP 提供的工具：

```python
tools = session.get(f"{BASE_URL}/tools").json()
for tool in tools:
    print(f"- {tool['name']}: {tool.get('description', '无描述')}")
```

## WebSocket：实时日志

Ruri 提供了一个 WebSocket 端点用于流式传输实时日志消息。这对于构建仪表板、监控工具或自定义 UI 非常有用。

### 连接

连接到 `ws://localhost:3000/api/ws/logs`：

#### Python（websockets）

```python
import asyncio
import json
import websockets

async def listen_to_logs():
    # 注意：需要传递会话 Cookie 进行认证
    uri = "ws://localhost:3000/api/ws/logs"
    async with websockets(uri, extra_headers=[
        ("Cookie", "session=your-session-cookie")
    ]) as ws:
        async for message in ws:
            log_entry = json.loads(message)
            level = log_entry.get("level", "info")
            text = log_entry.get("message", "")
            timestamp = log_entry.get("timestamp", "")
            print(f"[{timestamp}] [{level.upper()}] {text}")

asyncio.run(listen_to_logs())
```

#### JavaScript（原生 WebSocket）

```javascript
const ws = new WebSocket("ws://localhost:3000/api/ws/logs");

ws.onopen = () => {
  console.log("已连接到 Ruri 日志流");
};

ws.onmessage = (event) => {
  const logEntry = JSON.parse(event.data);
  const { level, message, timestamp } = logEntry;
  console.log(`[${timestamp}] [${level.toUpperCase()}] ${message}`);
};

ws.onerror = (error) => {
  console.error("WebSocket 错误:", error);
};

ws.onclose = () => {
  console.log("已断开日志流连接");
};
```

### 日志消息格式

每条消息是一个 JSON 对象：

```json
{
  "level": "info",
  "message": "Agent started processing",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

`level` 字段可以是以下值之一：`trace`、`debug`、`info`、`warn`、`error`。

## 错误处理

所有 API 错误遵循一致的格式：

```json
{
  "error": "描述错误信息的字符串"
}
```

### 常见状态码

| 状态码 | 含义 | 处理方式 |
| --- | --- | --- |
| `200` | 成功 | 解析响应体 |
| `201` | 已创建 | 资源创建成功 |
| `400` | 请求错误 | 检查请求体中是否有缺失或无效的字段 |
| `401` | 未授权 | 重新登录——你的会话可能已过期 |
| `403` | 禁止访问 | 你没有执行此操作的权限 |
| `404` | 未找到 | 检查资源 ID 或端点路径 |
| `500` | 服务器错误 | 查看 Ruri 日志获取详情（`RUST_LOG=debug`） |

### Python 错误处理模式

```python
def safe_api_call(method, url, **kwargs):
    """带有正确错误处理的 API 调用。"""
    try:
        response = session.request(method, url, **kwargs)

        if response.status_code == 401:
            # 会话过期 — 重新认证
            session.post(f"{BASE_URL}/auth/login", json={
                "username": "ruri",
                "password": "ruri"
            })
            # 重试原始请求
            response = session.request(method, url, **kwargs)

        response.raise_for_status()
        return response.json()
    except requests.exceptions.HTTPError as e:
        error_data = e.response.json() if e.response.headers.get("content-type", "").startswith("application/json") else {}
        print(f"API 错误 ({e.response.status_code}): {error_data.get('error', str(e))}")
        raise
    except requests.exceptions.ConnectionError:
        print("无法连接到 Ruri —— 服务器是否正在运行？")
        raise

# 使用示例
providers = safe_api_call("GET", f"{BASE_URL}/providers")
```

### JavaScript 错误处理模式

```javascript
async function safeApiCall(method, path, data = null) {
  try {
    const config = { method, url: path };
    if (data) {
      config.data = data;
    }

    const response = await client(config);
    return response.data;
  } catch (error) {
    if (error.response?.status === 401) {
      // 会话过期 — 重新认证
      await client.post("/auth/login", {
        username: "ruri",
        password: "ruri",
      });
      // 重试原始请求
      const config = { method, url: path };
      if (data) config.data = data;
      const response = await client(config);
      return response.data;
    }

    const status = error.response?.status;
    const message = error.response?.data?.error || error.message;
    console.error(`API 错误 (${status}): ${message}`);
    throw error;
  }
}

// 使用示例
const providers = await safeApiCall("get", "/providers");
```

## 完整示例：聊天客户端脚本

以下是一个完整的 Python 脚本，实现登录并发送聊天消息：

```python
#!/usr/bin/env python3
"""简单的 Ruri 聊天客户端。"""
import requests
import sys

BASE_URL = "http://localhost:3000/api"

def main():
    session = requests.Session()

    # 登录
    res = session.post(f"{BASE_URL}/auth/login", json={
        "username": "ruri",
        "password": "ruri"
    })
    res.raise_for_status()
    print("✓ 登录成功")

    # 发送消息
    message = " ".join(sys.argv[1:]) or "你好，琉璃！"
    res = session.post(f"{BASE_URL}/chat", json={
        "message": message
    })
    res.raise_for_status()
    data = res.json()

    # 打印响应
    if "choices" in data and data["choices"]:
        content = data["choices"][0].get("message", {}).get("content", "")
        print(f"\nRuri: {content}")
    else:
        print(f"\n响应: {data}")

if __name__ == "__main__":
    main()
```

## 完整 API 参考

有关完整的端点列表、请求/响应格式和详情，请参见 [API 参考](/api)。
