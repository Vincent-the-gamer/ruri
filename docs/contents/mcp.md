---
layout: doc
title: "MCP Client"
lastUpdated: true
---

# MCP Client

The **Model Context Protocol (MCP)** Client allows Ruri to connect to external MCP servers, extending the agent's capabilities with additional tools and data sources provided by those servers.

## Overview

MCP is a protocol that enables AI agents to interact with external tools and services. By connecting to MCP servers, Ruri can:

- Access external databases and APIs
- Use specialized tools not included in the built-in set
- Retrieve contextual information from external sources
- Interact with third-party services

## Transport Types

Ruri supports four transport types for connecting to MCP servers:

### Stdio

The MCP server runs as a local process, communicating via standard input/output streams.

**Configuration:**

| Field     | Type   | Description                                    |
| --------- | ------ | ---------------------------------------------- |
| `command` | string | The command to start the MCP server            |
| `args`    | array  | Command-line arguments for the server          |
| `env`     | object | Environment variables to set for the process   |

**Example:** Connect to a filesystem MCP server:

```yaml
transport: stdio
command: "npx"
args:
  - "@modelcontextprotocol/server-filesystem"
  - "/path/to/allowed/directory"
env:
  NODE_ENV: "production"
```

### SSE (Server-Sent Events)

Connect to a remote MCP server using SSE for server-to-client messages and HTTP POST for client-to-server messages.

**Configuration:**

| Field    | Type   | Description                         |
| -------- | ------ | ----------------------------------- |
| `url`    | string | The SSE endpoint URL                |
| `headers`| object | HTTP headers to include in requests |

**Example:**

```yaml
transport: sse
url: "https://mcp-server.example.com/sse"
headers:
  Authorization: "Bearer your-api-key"
```

### WebSocket

Connect to an MCP server using WebSocket for bidirectional communication.

**Configuration:**

| Field    | Type   | Description                         |
| -------- | ------ | ----------------------------------- |
| `url`    | string | The WebSocket endpoint URL          |
| `headers`| object | HTTP headers for the connection     |

**Example:**

```yaml
transport: websocket
url: "wss://mcp-server.example.com/ws"
```

### HTTP

Connect to an MCP server using HTTP requests.

**Configuration:**

| Field    | Type   | Description                         |
| -------- | ------ | ----------------------------------- |
| `url`    | string | The HTTP endpoint URL               |
| `headers`| object | HTTP headers to include in requests |

**Example:**

```yaml
transport: http
url: "https://mcp-server.example.com/mcp"
headers:
  Authorization: "Bearer your-api-key"
```

## Managing MCP Servers

### Via Web UI

1. Navigate to the **MCP** page in the sidebar
2. Add a new MCP server with the desired transport type and configuration
3. Enable or disable servers as needed
4. Monitor the connection status of each server

### Configuration

MCP servers are configured with the following fields:

| Field       | Type   | Description                                     |
| ----------- | ------ | ----------------------------------------------- |
| `name`      | string | A unique identifier for the MCP server          |
| `transport` | string | Transport type: `stdio`, `sse`, `websocket`, `http` |
| `command`   | string | (Stdio only) Command to start the server        |
| `args`      | array  | (Stdio only) Command-line arguments             |
| `env`       | object | (Stdio only) Environment variables              |
| `url`       | string | (SSE/WebSocket/HTTP) Server endpoint URL        |
| `headers`   | object | (SSE/WebSocket/HTTP) HTTP headers               |

## How MCP Tools Work

When Ruri connects to an MCP server:

1. The MCP server advertises available tools
2. Ruri registers these tools alongside its built-in tools
3. The AI model can invoke these tools during conversations
4. Tool calls are forwarded to the MCP server for execution
5. Results are returned to the model for processing

This means MCP tools are available to the AI agent just like built-in tools — the model decides when to use them based on the conversation context.

## Security Considerations

- **Stdio transport** runs a local process with the same permissions as the Ruri server. Only configure MCP servers you trust.
- **Remote transports** (SSE, WebSocket, HTTP) communicate over the network. Use HTTPS/WSS and include authentication headers where possible.
- **Tool permissions** — MCP tools are subject to the same skill-level restrictions via `allowed_tools`. Use this to control which MCP tools are available in specific contexts.

::: warning
Be cautious when connecting to untrusted MCP servers. The tools they provide can access files, execute commands, or make network requests on behalf of the agent.
:::
