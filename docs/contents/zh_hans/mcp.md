---
layout: doc
title: "MCP 客户端"
lastUpdated: true
---

# MCP 客户端

**模型上下文协议（Model Context Protocol，MCP）** 客户端允许 Ruri 连接到外部 MCP 服务器，通过这些服务器提供的额外工具和数据源扩展智能体的能力。

## 概述

MCP 是一种协议，使 AI 智能体能够与外部工具和服务交互。通过连接 MCP 服务器，Ruri 可以：

- 访问外部数据库和 API
- 使用内置集合中不包含的专业工具
- 从外部来源获取上下文信息
- 与第三方服务交互

## 传输类型

Ruri 支持四种传输类型来连接 MCP 服务器：

### Stdio

MCP 服务器作为本地进程运行，通过标准输入/输出流进行通信。

**配置：**

| 字段      | 类型   | 描述                           |
| --------- | ------ | ------------------------------ |
| `command` | string | 启动 MCP 服务器的命令          |
| `args`    | array  | 服务器的命令行参数             |
| `env`     | object | 为进程设置的环境变量           |

**示例：** 连接到文件系统 MCP 服务器：

```yaml
transport: stdio
command: "npx"
args:
  - "@modelcontextprotocol/server-filesystem"
  - "/path/to/allowed/directory"
env:
  NODE_ENV: "production"
```

### SSE（Server-Sent Events）

使用 SSE 进行服务器到客户端的消息通信，使用 HTTP POST 进行客户端到服务器的消息通信，连接到远程 MCP 服务器。

**配置：**

| 字段      | 类型   | 描述                     |
| --------- | ------ | ------------------------ |
| `url`     | string | SSE 端点 URL             |
| `headers` | object | 请求中包含的 HTTP 头     |

**示例：**

```yaml
transport: sse
url: "https://mcp-server.example.com/sse"
headers:
  Authorization: "Bearer your-api-key"
```

### WebSocket

使用 WebSocket 进行双向通信，连接到 MCP 服务器。

**配置：**

| 字段      | 类型   | 描述                     |
| --------- | ------ | ------------------------ |
| `url`     | string | WebSocket 端点 URL       |
| `headers` | object | 连接的 HTTP 头           |

**示例：**

```yaml
transport: websocket
url: "wss://mcp-server.example.com/ws"
```

### HTTP

使用 HTTP 请求连接到 MCP 服务器。

**配置：**

| 字段      | 类型   | 描述                     |
| --------- | ------ | ------------------------ |
| `url`     | string | HTTP 端点 URL            |
| `headers` | object | 请求中包含的 HTTP 头     |

**示例：**

```yaml
transport: http
url: "https://mcp-server.example.com/mcp"
headers:
  Authorization: "Bearer your-api-key"
```

## 管理 MCP 服务器

### 通过 Web UI

1. 在侧边栏中导航到 **MCP** 页面
2. 添加新的 MCP 服务器，选择所需的传输类型和配置
3. 根据需要启用或禁用服务器
4. 监控每个服务器的连接状态

### 配置

MCP 服务器的配置字段如下：

| 字段        | 类型   | 描述                                                |
| ----------- | ------ | --------------------------------------------------- |
| `name`      | string | MCP 服务器的唯一标识符                              |
| `transport` | string | 传输类型：`stdio`、`sse`、`websocket`、`http`       |
| `command`   | string | （仅 Stdio）启动服务器的命令                        |
| `args`      | array  | （仅 Stdio）命令行参数                              |
| `env`       | object | （仅 Stdio）环境变量                                |
| `url`       | string | （SSE/WebSocket/HTTP）服务器端点 URL                |
| `headers`   | object | （SSE/WebSocket/HTTP）HTTP 头                       |

## MCP 工具的工作原理

当 Ruri 连接到 MCP 服务器时：

1. MCP 服务器广播可用的工具
2. Ruri 将这些工具与其内置工具一起注册
3. AI 模型可以在对话中调用这些工具
4. 工具调用被转发到 MCP 服务器执行
5. 结果返回给模型进行处理

这意味着 MCP 工具与内置工具一样可供 AI 智能体使用 — 模型根据对话上下文决定何时使用它们。

## 安全注意事项

- **Stdio 传输**以与 Ruri 服务器相同的权限运行本地进程。仅配置您信任的 MCP 服务器。
- **远程传输**（SSE、WebSocket、HTTP）通过网络通信。尽可能使用 HTTPS/WSS 并包含身份验证头。
- **工具权限** — MCP 工具受技能级别的 `allowed_tools` 限制。使用它来控制特定上下文中可用的 MCP 工具。

::: warning
连接不受信任的 MCP 服务器时请谨慎。它们提供的工具可以代表智能体访问文件、执行命令或发起网络请求。
:::
