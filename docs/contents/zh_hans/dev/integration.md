---
layout: doc
title: "集成指南"
lastUpdated: true
---

# 集成指南

本指南介绍如何将 Ruri 集成到你自己的应用、工具和工作流中——从简单的 REST API 调用到构建完整的自定义客户端。

## 通过 REST API 集成

与 Ruri 集成最简单的方式是通过其 REST API。任何语言的任何 HTTP 客户端都可以与 Ruri 交互。

### 基本集成模式

1. **认证** — `POST /api/auth/login` 发送凭据
2. **存储会话 Cookie** — 在所有后续请求中包含该 Cookie
3. **调用 API 端点** — 聊天、管理提供商、读取工具等
4. **处理错误** — 解析 `{"error": "..."}` 错误格式

详细的代码示例请参见 [API 使用指南](/zh_hans/dev/api-usage)，完整的端点列表请参见 [API 参考](/api)。

### 快速集成清单

- [ ] 认证并存储会话 Cookie
- [ ] 使用 `GET /api/agent/status` 验证代理状态
- [ ] 使用 `GET /api/tools` 列出可用工具
- [ ] 通过 `POST /api/chat` 发送消息
- [ ] 处理 `401` 响应时的重新认证
- [ ] 可选：连接 `WS /api/ws/logs` 进行实时监控

### CORS 注意事项

Ruri 的服务器包含 CORS 中间件，允许跨域携带凭据的请求。如果你在不同源上构建基于浏览器的客户端，请确保：

- 在请求中设置 `credentials: "include"`（fetch）或 `withCredentials: true`（axios）
- 处理 `Set-Cookie` 头以管理会话

::: info
当使用 `--remote` 参数运行 Ruri 时，服务器绑定到 `0.0.0.0`，使其可以从网络中的其他机器访问。这对于在不同主机上运行的集成非常有用。
:::

## ACP 集成（用于 IDE）

**Agent Client Protocol（ACP）** 允许你在代码编辑器中使用 Ruri 作为 AI 助手。Ruri 的 ACP 服务器通过 stdio 使用 JSON-RPC 通信，兼容任何支持 ACP 的 IDE。

### ACP 工作原理

1. IDE 启动 Ruri 二进制文件，附带 `--acp` 参数
2. Ruri 以 stdio 模式启动——从 stdin 读取 JSON-RPC 请求，向 stdout 写入响应
3. 日志输出到 stderr，避免干扰协议通信
4. IDE 发送请求（聊天、文件操作、工具调用）并接收结构化响应

### 在 IDE 中配置 ACP

#### Zed

添加到你的 Zed `settings.json`：

```json
{
  "agent_servers": {
    "ruri": {
      "type": "custom",
      "command": "/path/to/ruri",
      "args": ["--acp"]
    }
  }
}
```

#### JetBrains

1. 打开 **设置** → **工具** → **AI Assistant**
2. 添加外部代理服务器
3. 将命令设置为 Ruri 二进制文件路径
4. 添加 `--acp` 作为参数

#### 自定义 ACP 客户端

如果你正在构建自己的 IDE 插件或编辑器集成，可以通过 stdio 与 ACP 服务器通信：

```python
import subprocess
import json

# 以 ACP 模式启动 Ruri
process = subprocess.Popen(
    ["./ruri", "--acp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

# 发送 JSON-RPC 请求
request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
}
process.stdin.write(json.dumps(request) + "\n")
process.stdin.flush()

# 读取响应
response_line = process.stdout.readline()
response = json.loads(response_line)
print(response)
```

::: tip
在 ACP 模式下运行时，Ruri 使用你保存的配置——包括当前活跃的模型提供商、人设、技能和工具。请确保先通过 Web UI 配置好这些设置。
:::

有关 ACP 模式下可用功能的更多详情，请参见 [ACP 服务器](/acp) 文档。

## MCP 服务器开发基础

Ruri 可以连接到外部 **MCP（Model Context Protocol）** 服务器来扩展其工具能力。如果你想构建 Ruri 可以使用的 MCP 服务器，以下是你需要了解的内容。

### 什么是 MCP 服务器？

MCP 服务器是一个通过标准化协议暴露**工具**和**资源**的程序。当 Ruri 连接到你的 MCP 服务器时，其工具将与内置工具一起供 AI 代理使用。

### 传输类型

Ruri 支持通过以下方式连接 MCP 服务器：

| 传输方式 | 描述 | 使用场景 |
| --- | --- | --- |
| **Stdio** | 运行本地进程，通过 stdin/stdout 通信 | 本地工具、文件系统访问、本地数据库 |
| **SSE** | 基于 HTTP 的服务器推送事件 | 远程服务器、云端托管的工具 |
| **WebSocket** | 持久 WebSocket 连接 | 实时工具、流式数据 |
| **HTTP** | 标准 HTTP 请求/响应 | 简单的远程 API |

### 构建基础 MCP 服务器

以下是使用 `@modelcontextprotocol/sdk` 包的 MCP 服务器最小示例：

```javascript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const server = new McpServer({
  name: "my-ruri-tools",
  version: "1.0.0",
});

// 注册一个工具
server.tool(
  "get_weather",                        // 工具名称
  "获取指定城市的当前天气",             // 描述
  { city: z.string().describe("城市名称") }, // 参数
  async ({ city }) => {                 // 处理函数
    // 你的工具逻辑
    const weather = { city, temp: "22°C", condition: "晴天" };
    return {
      content: [{ type: "text", text: JSON.stringify(weather) }],
    };
  }
);

// 启动服务器
const transport = new StdioServerTransport();
await server.connect(transport);
```

### 将 MCP 服务器添加到 Ruri

1. 在 Ruri Web UI 中进入 **MCP** 页面
2. 点击 **添加服务器**
3. 填写详情：
   - **名称：** 唯一标识符（例如 `my-weather-tool`）
   - **传输类型：** 本地服务器选择 `Stdio`
   - **命令：** 启动服务器的命令（例如 `node`、`python`）
   - **参数：** 脚本路径和任何参数
4. 将服务器开关切换为**开启**
5. 验证工具出现在 `GET /api/tools` 中

```bash
# 示例：通过 API 添加
curl -b cookies.txt -X POST http://localhost:3000/api/mcp/servers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-weather-tool",
    "transport_type": "stdio",
    "command": "node",
    "args": ["./my-mcp-server.js"]
  }'
```

::: warning
仅连接你信任的 MCP 服务器。它们提供的工具可以代表 AI 代理访问文件、执行命令或发起网络请求。使用[技能](/skills)的 `allowed_tools` 来限制特定上下文中可用的 MCP 工具。
:::

有关在 Ruri 中使用 MCP 服务器的更多信息，请参见 [MCP 客户端](/mcp) 文档。

## Webhook 模式

虽然 Ruri 没有内置的 Webhook 支持来推送事件，但你可以使用以下模式构建类似 Webhook 的行为。

### 模式一：轮询代理状态

定期检查代理状态以检测处理何时完成：

```python
import time
import requests

session = requests.Session()
session.post("http://localhost:3000/api/auth/login", json={
    "username": "ruri", "password": "ruri"
})

# 发送消息
session.post("http://localhost:3000/api/chat", json={
    "message": "分析这个代码库"
})

# 轮询直到处理完成
while True:
    status = session.get("http://localhost:3000/api/agent/status").json()
    if not status.get("processing", False):
        print("代理处理完成")
        # 获取对话结果
        history = session.get("http://localhost:3000/api/chat/history").json()
        break
    time.sleep(2)
```

### 模式二：WebSocket 日志流

使用 WebSocket 日志端点实时响应事件：

```javascript
const ws = new WebSocket("ws://localhost:3000/api/ws/logs");

ws.onmessage = (event) => {
  const log = JSON.parse(event.data);

  // 响应特定事件
  if (log.message.includes("Agent completed")) {
    // 获取最新的聊天响应
    fetchChatResponse().then((response) => {
      // 发送到你的 Webhook 端点
      fetch("https://your-app.com/webhook", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          event: "agent_completed",
          response: response,
          timestamp: log.timestamp,
        }),
      });
    });
  }
};
```

### 模式三：平台适配器作为集成钩子

Ruri 的平台系统可以作为集成机制。从平台（钉钉、Discord、微信）接收的消息由 AI 处理，响应被发送回去。你可以通过以下方式利用这一点：

1. 构建一个轻量级的"平台"服务，接收来自外部系统的 Webhook
2. 将这些 Webhook 载荷作为消息转发到 Ruri 的聊天 API
3. 将 AI 的响应返回给原始系统

```python
from flask import Flask, request
import requests as http

app = Flask(__name__)
ruri_session = http.Session()

# 预认证
ruri_session.post("http://localhost:3000/api/auth/login", json={
    "username": "ruri", "password": "ruri"
})

@app.route("/webhook/github", methods=["POST"])
def github_webhook():
    """通过 Ruri 处理 GitHub Webhook，让 AI 总结变更。"""
    payload = request.json
    message = f"有新的 PR 被创建：{payload.get('pull_request', {}).get('title', '未知')}"

    # 转发到 Ruri
    response = ruri_session.post("http://localhost:3000/api/chat", json={
        "message": message
    })

    return {"status": "ok", "ruri_response": response.json()}
```

## 示例：构建自定义聊天客户端

以下是一个完整的示例，构建一个基于终端的聊天客户端，通过 REST API 与 Ruri 通信。

### Python 聊天客户端

```python
#!/usr/bin/env python3
"""
Ruri 终端聊天客户端 — 一个简单的 Ruri 聊天界面。
"""
import requests
import sys

BASE_URL = "http://localhost:3000/api"

class RuriClient:
    def __init__(self, base_url=BASE_URL, username="ruri", password="ruri"):
        self.base_url = base_url
        self.session = requests.Session()

        # 认证
        res = self.session.post(f"{base_url}/auth/login", json={
            "username": username,
            "password": password,
        })
        res.raise_for_status()
        print("✓ 已连接到 Ruri")

        # 检查代理状态
        status = self.session.get(f"{base_url}/agent/status").json()
        print(f"  代理状态：{'处理中' if status.get('processing') else '空闲'}")

        # 创建对话
        conv = self.session.post(f"{base_url}/conversations", json={
            "title": "终端聊天"
        }).json()
        self.conversation_id = conv.get("id")
        print(f"  对话：{self.conversation_id}\n")

    def chat(self, message):
        """发送消息并返回 AI 响应。"""
        res = self.session.post(f"{self.base_url}/chat", json={
            "message": message,
            "conversation_id": self.conversation_id,
        })
        res.raise_for_status()
        data = res.json()

        # 从响应中提取文本内容
        if "choices" in data and data["choices"]:
            content = data["choices"][0].get("message", {}).get("content", "")
            return content
        return str(data)

    def get_available_tools(self):
        """列出所有可用工具。"""
        tools = self.session.get(f"{self.base_url}/tools").json()
        return [t.get("name", "unknown") for t in tools]

    def get_providers(self):
        """列出所有已配置的提供商。"""
        return self.session.get(f"{self.base_url}/providers").json()


def main():
    client = RuriClient()

    print("Ruri 终端聊天 — 输入 '/quit' 退出，'/tools' 列出工具，'/providers' 列出提供商\n")

    while True:
        try:
            user_input = input("你: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n再见！")
            break

        if not user_input:
            continue

        if user_input == "/quit":
            print("再见！")
            break
        elif user_input == "/tools":
            tools = client.get_available_tools()
            print(f"\n可用工具：{', '.join(tools)}\n")
            continue
        elif user_input == "/providers":
            providers = client.get_providers()
            for p in providers:
                active = " (活跃)" if p.get("active") else ""
                print(f"  - {p.get('name', '未知')}: {p.get('model', 'N/A')}{active}")
            print()
            continue

        response = client.chat(user_input)
        print(f"\nRuri: {response}\n")


if __name__ == "__main__":
    main()
```

### JavaScript 聊天客户端（Node.js）

```javascript
import axios from "axios";
import readline from "readline";

const BASE_URL = "http://localhost:3000/api";

class RuriClient {
  constructor(baseURL = BASE_URL) {
    this.client = axios.create({
      baseURL,
      withCredentials: true,
    });
  }

  async login(username = "ruri", password = "ruri") {
    await this.client.post("/auth/login", { username, password });
    console.log("✓ 已连接到 Ruri");
  }

  async createConversation(title = "Node 聊天") {
    const { data } = await this.client.post("/conversations", { title });
    this.conversationId = data.id;
    console.log(`  对话：${this.conversationId}\n`);
  }

  async chat(message) {
    const { data } = await this.client.post("/chat", {
      message,
      conversation_id: this.conversationId,
    });

    if (data.choices?.[0]?.message?.content) {
      return data.choices[0].message.content;
    }
    return JSON.stringify(data);
  }

  async getTools() {
    const { data } = await this.client.get("/tools");
    return data.map((t) => t.name);
  }
}

async function main() {
  const client = new RuriClient();
  await client.login();
  await client.createConversation();

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  console.log("Ruri 终端聊天 — 输入 '/quit' 退出，'/tools' 列出工具\n");

  const prompt = () => {
    rl.question("你: ", async (input) => {
      input = input.trim();
      if (!input) return prompt();
      if (input === "/quit") {
        console.log("再见！");
        rl.close();
        return;
      }
      if (input === "/tools") {
        const tools = await client.getTools();
        console.log(`\n可用工具：${tools.join(", ")}\n`);
        return prompt();
      }

      const response = await client.chat(input);
      console.log(`\nRuri: ${response}\n`);
      prompt();
    });
  };

  prompt();
}

main().catch(console.error);
```

## 总结

| 集成方式 | 最适合 | 协议 |
| --- | --- | --- |
| REST API | 自定义应用、脚本、自动化 | HTTP + JSON |
| ACP | IDE 集成、编辑器插件 | JSON-RPC over stdio |
| MCP | 扩展 Ruri 的工具能力 | Stdio / SSE / WebSocket / HTTP |
| WebSocket | 实时日志监控、仪表板 | WebSocket |

## 下一步

- [API 使用指南](/zh_hans/dev/api-usage) — 详细的 API 模式和代码示例
- [API 参考](/api) — 完整的端点文档
- [ACP 服务器](/acp) — IDE 集成设置
- [MCP 客户端](/mcp) — 连接外部工具服务器
- [技能](/skills) — 自定义 AI 行为
- [平台](/platforms) — 聊天平台集成
