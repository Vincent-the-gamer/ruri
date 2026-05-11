---
layout: doc
title: "Integration Guide"
lastUpdated: true
---

# Integration Guide

This guide covers how to integrate Ruri into your own applications, tools, and workflows — from simple REST API calls to building full custom clients.

## Integrating via the REST API

The simplest way to integrate with Ruri is through its REST API. Any HTTP client in any language can interact with Ruri.

### Basic Integration Pattern

1. **Authenticate** — `POST /api/auth/login` with credentials
2. **Store the session cookie** — Include it in all subsequent requests
3. **Call API endpoints** — Chat, manage providers, read tools, etc.
4. **Handle errors** — Parse the `{"error": "..."}` error format

See the [API Usage Guide](/dev/api-usage) for detailed code examples and the [API Reference](/api) for the full endpoint list.

### Quick Integration Checklist

- [ ] Authenticate and store the session cookie
- [ ] Verify the agent status with `GET /api/agent/status`
- [ ] List available tools with `GET /api/tools`
- [ ] Send messages via `POST /api/chat`
- [ ] Handle re-authentication on `401` responses
- [ ] Optionally connect to `WS /api/ws/logs` for real-time monitoring

### CORS Considerations

Ruri's server includes CORS middleware that allows cross-origin requests with credentials. If you're building a browser-based client on a different origin, make sure to:

- Set `credentials: "include"` (fetch) or `withCredentials: true` (axios) in your requests
- Handle the `Set-Cookie` header for session management

::: info
When running Ruri with `--remote`, the server binds to `0.0.0.0`, making it accessible from other machines on your network. This is useful for integrations running on different hosts.
:::

## ACP Integration for IDEs

The **Agent Client Protocol (ACP)** lets you use Ruri as an AI assistant inside code editors. Ruri's ACP server communicates over stdio using JSON-RPC, making it compatible with any ACP-capable IDE.

### How ACP Works

1. The IDE launches the Ruri binary with the `--acp` flag
2. Ruri starts in stdio mode — it reads JSON-RPC requests from stdin and writes responses to stdout
3. Logging output goes to stderr to avoid interfering with the protocol
4. The IDE sends requests (chat, file operations, tool calls) and receives structured responses

### Configuring ACP in an IDE

#### Zed

Add to your Zed `settings.json`:

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

1. Open **Settings** → **Tools** → **AI Assistant**
2. Add an external agent server
3. Set the command to the Ruri binary path
4. Add `--acp` as an argument

#### Custom ACP Client

If you're building your own IDE plugin or editor integration, you can communicate with the ACP server over stdio:

```python
import subprocess
import json

# Launch Ruri in ACP mode
process = subprocess.Popen(
    ["./ruri", "--acp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

# Send a JSON-RPC request
request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
}
process.stdin.write(json.dumps(request) + "\n")
process.stdin.flush()

# Read the response
response_line = process.stdout.readline()
response = json.loads(response_line)
print(response)
```

::: tip
When running in ACP mode, Ruri uses your saved configuration — including the active model provider, persona, skills, and tools. Make sure these are configured through the Web UI first.
:::

See the [ACP Server](/acp) documentation for more details on what features are available in ACP mode.

## MCP Server Development Basics

Ruri can connect to external **MCP (Model Context Protocol)** servers to extend its tool capabilities. If you want to build your own MCP server that Ruri can use, here's what you need to know.

### What Is an MCP Server?

An MCP server is a program that exposes **tools** and **resources** over a standardized protocol. When Ruri connects to your MCP server, its tools become available to the AI agent alongside the built-in ones.

### Transport Types

Ruri supports connecting to MCP servers via:

| Transport | Description | Use Case |
| --- | --- | --- |
| **Stdio** | Runs a local process, communicates over stdin/stdout | Local tools, filesystem access, local databases |
| **SSE** | Server-Sent Events over HTTP | Remote servers, cloud-hosted tools |
| **WebSocket** | Persistent WebSocket connection | Real-time tools, streaming data |
| **HTTP** | Standard HTTP request/response | Simple remote APIs |

### Building a Basic MCP Server

Here's a minimal example of an MCP server using the `@modelcontextprotocol/sdk` package:

```javascript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const server = new McpServer({
  name: "my-ruri-tools",
  version: "1.0.0",
});

// Register a tool
server.tool(
  "get_weather",                        // Tool name
  "Get the current weather for a city", // Description
  { city: z.string().describe("City name") }, // Parameters
  async ({ city }) => {                 // Handler
    // Your tool logic here
    const weather = { city, temp: "22°C", condition: "Sunny" };
    return {
      content: [{ type: "text", text: JSON.stringify(weather) }],
    };
  }
);

// Start the server
const transport = new StdioServerTransport();
await server.connect(transport);
```

### Adding Your MCP Server to Ruri

1. Go to the **MCP** page in the Ruri Web UI
2. Click **Add Server**
3. Fill in the details:
   - **Name:** A unique identifier (e.g., `my-weather-tool`)
   - **Transport Type:** `Stdio` for local servers
   - **Command:** The command to start your server (e.g., `node`, `python`)
   - **Arguments:** The script path and any arguments
4. Toggle the server **on**
5. Verify the tools appear in `GET /api/tools`

```bash
# Example: Adding via the API
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
Only connect to MCP servers you trust. The tools they provide can access files, execute commands, or make network requests on behalf of the AI agent. Use [Skills](/skills) with `allowed_tools` to restrict which MCP tools are available in specific contexts.
:::

For more about using MCP servers with Ruri, see the [MCP Client](/mcp) documentation.

## Webhook Patterns

While Ruri doesn't have built-in webhook support for outgoing events, you can build webhook-like behavior using the following patterns.

### Pattern 1: Polling the Agent Status

Periodically check the agent status to detect when processing completes:

```python
import time
import requests

session = requests.Session()
session.post("http://localhost:3000/api/auth/login", json={
    "username": "ruri", "password": "ruri"
})

# Send a message
session.post("http://localhost:3000/api/chat", json={
    "message": "Analyze this codebase"
})

# Poll until processing completes
while True:
    status = session.get("http://localhost:3000/api/agent/status").json()
    if not status.get("processing", False):
        print("Agent finished processing")
        # Fetch the conversation result
        history = session.get("http://localhost:3000/api/chat/history").json()
        break
    time.sleep(2)
```

### Pattern 2: WebSocket Log Streaming

Use the WebSocket log endpoint to react to events in real time:

```javascript
const ws = new WebSocket("ws://localhost:3000/api/ws/logs");

ws.onmessage = (event) => {
  const log = JSON.parse(event.data);

  // React to specific events
  if (log.message.includes("Agent completed")) {
    // Fetch the latest chat response
    fetchChatResponse().then((response) => {
      // Send to your webhook endpoint
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

### Pattern 3: Platform Adapter as Integration Hook

Ruri's platform system can serve as an integration mechanism. Messages received from a platform (DingTalk, Discord, WeChat) are processed by the AI and responses are sent back. You can leverage this by:

1. Building a lightweight "platform" service that receives webhooks from external systems
2. Forwarding those webhook payloads as messages to Ruri's chat API
3. Returning the AI's response to the originating system

```python
from flask import Flask, request
import requests as http

app = Flask(__name__)
ruri_session = http.Session()

# Pre-authenticate
ruri_session.post("http://localhost:3000/api/auth/login", json={
    "username": "ruri", "password": "ruri"
})

@app.route("/webhook/github", methods=["POST"])
def github_webhook():
    """Handle GitHub webhooks by asking Ruri to summarize changes."""
    payload = request.json
    message = f"A new PR was opened: {payload.get('pull_request', {}).get('title', 'Unknown')}"

    # Forward to Ruri
    response = ruri_session.post("http://localhost:3000/api/chat", json={
        "message": message
    })

    return {"status": "ok", "ruri_response": response.json()}
```

## Example: Building a Custom Chat Client

Here's a complete example of building a terminal-based chat client that communicates with Ruri through the REST API.

### Python Chat Client

```python
#!/usr/bin/env python3
"""
Ruri Terminal Chat Client — A simple chat interface for Ruri.
"""
import requests
import sys

BASE_URL = "http://localhost:3000/api"

class RuriClient:
    def __init__(self, base_url=BASE_URL, username="ruri", password="ruri"):
        self.base_url = base_url
        self.session = requests.Session()

        # Authenticate
        res = self.session.post(f"{base_url}/auth/login", json={
            "username": username,
            "password": password,
        })
        res.raise_for_status()
        print("✓ Connected to Ruri")

        # Check agent status
        status = self.session.get(f"{base_url}/agent/status").json()
        print(f"  Agent status: {'processing' if status.get('processing') else 'idle'}")

        # Create a conversation
        conv = self.session.post(f"{base_url}/conversations", json={
            "title": "Terminal Chat"
        }).json()
        self.conversation_id = conv.get("id")
        print(f"  Conversation: {self.conversation_id}\n")

    def chat(self, message):
        """Send a message and return the AI response."""
        res = self.session.post(f"{self.base_url}/chat", json={
            "message": message,
            "conversation_id": self.conversation_id,
        })
        res.raise_for_status()
        data = res.json()

        # Extract the text content from the response
        if "choices" in data and data["choices"]:
            content = data["choices"][0].get("message", {}).get("content", "")
            return content
        return str(data)

    def get_available_tools(self):
        """List all available tools."""
        tools = self.session.get(f"{self.base_url}/tools").json()
        return [t.get("name", "unknown") for t in tools]

    def get_providers(self):
        """List all configured providers."""
        return self.session.get(f"{self.base_url}/providers").json()


def main():
    client = RuriClient()

    print("Ruri Terminal Chat — type '/quit' to exit, '/tools' to list tools, '/providers' to list providers\n")

    while True:
        try:
            user_input = input("You: ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\nGoodbye!")
            break

        if not user_input:
            continue

        if user_input == "/quit":
            print("Goodbye!")
            break
        elif user_input == "/tools":
            tools = client.get_available_tools()
            print(f"\nAvailable tools: {', '.join(tools)}\n")
            continue
        elif user_input == "/providers":
            providers = client.get_providers()
            for p in providers:
                active = " (active)" if p.get("active") else ""
                print(f"  - {p.get('name', 'Unknown')}: {p.get('model', 'N/A')}{active}")
            print()
            continue

        response = client.chat(user_input)
        print(f"\nRuri: {response}\n")


if __name__ == "__main__":
    main()
```

### JavaScript Chat Client (Node.js)

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
    console.log("✓ Connected to Ruri");
  }

  async createConversation(title = "Node Chat") {
    const { data } = await this.client.post("/conversations", { title });
    this.conversationId = data.id;
    console.log(`  Conversation: ${this.conversationId}\n`);
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

  console.log("Ruri Terminal Chat — type '/quit' to exit, '/tools' to list tools\n");

  const prompt = () => {
    rl.question("You: ", async (input) => {
      input = input.trim();
      if (!input) return prompt();
      if (input === "/quit") {
        console.log("Goodbye!");
        rl.close();
        return;
      }
      if (input === "/tools") {
        const tools = await client.getTools();
        console.log(`\nAvailable tools: ${tools.join(", ")}\n`);
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

## Summary

| Integration Method | Best For | Protocol |
| --- | --- | --- |
| REST API | Custom apps, scripts, automation | HTTP + JSON |
| ACP | IDE integrations, editor plugins | JSON-RPC over stdio |
| MCP | Extending Ruri's tool capabilities | Stdio / SSE / WebSocket / HTTP |
| WebSocket | Real-time log monitoring, dashboards | WebSocket |

## Next Steps

- [API Usage Guide](/dev/api-usage) — Detailed API patterns and code examples
- [API Reference](/api) — Complete endpoint documentation
- [ACP Server](/acp) — IDE integration setup
- [MCP Client](/mcp) — Connecting external tool servers
- [Skills](/skills) — Custom AI behaviors
- [Platforms](/platforms) — Chat platform integrations
