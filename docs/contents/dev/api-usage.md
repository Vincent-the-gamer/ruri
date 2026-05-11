---
layout: doc
title: "API Usage Guide"
lastUpdated: true
---

# API Usage Guide

This guide shows you how to use Ruri's REST API programmatically — from authenticating and making requests, to handling real-time logs over WebSocket. For the full endpoint reference, see the [API Reference](/api).

## Base URL

All API requests are made to:

```
http://localhost:3000/api
```

This can be changed with the `--port` flag when starting Ruri.

## Authentication

Ruri uses cookie-based session authentication. You must log in first to obtain a session cookie, then include it in all subsequent requests.

### Login Flow

1. **Send a login request** with your credentials
2. **Receive a session cookie** in the response
3. **Include the cookie** in all subsequent API requests

::: warning
The default credentials are `ruri` / `ruri`. Change the password after your first login for security.
:::

### Python Example

```python
import requests

BASE_URL = "http://localhost:3000/api"

# Create a session — it automatically stores and sends cookies
session = requests.Session()

# Step 1: Log in
response = session.post(f"{BASE_URL}/auth/login", json={
    "username": "ruri",
    "password": "ruri"
})

if response.status_code == 200:
    print("Logged in successfully!")
    # The session cookie is now stored in `session.cookies`
else:
    print(f"Login failed: {response.json()}")

# Step 2: Make authenticated requests — the cookie is sent automatically
providers = session.get(f"{BASE_URL}/providers")
print(providers.json())
```

### JavaScript Example (fetch)

```javascript
const BASE_URL = "http://localhost:3000/api";

// Step 1: Log in — the browser stores the session cookie automatically
const loginRes = await fetch(`${BASE_URL}/auth/login`, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ username: "ruri", password: "ruri" }),
  credentials: "include", // Important: include cookies
});

if (loginRes.ok) {
  console.log("Logged in successfully!");
}

// Step 2: Make authenticated requests
const providersRes = await fetch(`${BASE_URL}/providers`, {
  credentials: "include", // Include the session cookie
});
const providers = await providersRes.json();
console.log(providers);
```

### JavaScript Example (axios)

```javascript
import axios from "axios";

const client = axios.create({
  baseURL: "http://localhost:3000/api",
  withCredentials: true, // Send cookies with every request
});

// Step 1: Log in
await client.post("/auth/login", {
  username: "ruri",
  password: "ruri",
});

// Step 2: Make authenticated requests — cookies are sent automatically
const { data: providers } = await client.get("/providers");
console.log(providers);
```

### Using curl

```bash
# Log in and save the session cookie
curl -c cookies.txt -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"ruri","password":"ruri"}'

# Use the saved cookie for subsequent requests
curl -b cookies.txt http://localhost:3000/api/providers
```

::: tip
When using `requests.Session()` in Python or `axios` with `withCredentials: true` in JavaScript, cookies are handled automatically. You don't need to manually extract and set cookie headers.
:::

## Common API Patterns

### CRUD Operations

Most Ruri resources (providers, skills, platforms) follow the standard REST CRUD pattern:

| Action | Method | Endpoint | Body |
| --- | --- | --- | --- |
| List all | `GET` | `/api/{resource}` | — |
| Get one | `GET` | `/api/{resource}/:id` | — |
| Create | `POST` | `/api/{resource}` | JSON body |
| Update | `PUT` | `/api/{resource}/:id` | JSON body |
| Delete | `DELETE` | `/api/{resource}/:id` | — |

#### Example: Managing Providers

```python
# List all providers
providers = session.get(f"{BASE_URL}/providers").json()

# Create a new provider
new_provider = session.post(f"{BASE_URL}/providers", json={
    "name": "My OpenAI",
    "provider_type": "openai_compatible",
    "api_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "gpt-4o"
}).json()

# Activate the provider
session.post(f"{BASE_URL}/providers/{new_provider['id']}/activate")

# Update the provider
session.put(f"{BASE_URL}/providers/{new_provider['id']}", json={
    "name": "Updated Name",
    "model": "gpt-4o-mini"
})

# Delete the provider
session.delete(f"{BASE_URL}/providers/{new_provider['id']}")
```

#### Example: Managing Skills

```python
# List all skills
skills = session.get(f"{BASE_URL}/skills").json()

# Add a new skill
session.post(f"{BASE_URL}/skills", json={
    "name": "code-review",
    "content": "---\nname: code-review\ndescription: Review code\n---\nYou are a code reviewer."
})

# Toggle a skill on/off
session.patch(f"{BASE_URL}/skills/code-review", json={
    "enabled": True
})

# Delete a skill
session.delete(f"{BASE_URL}/skills/code-review")
```

#### Example: Managing Platforms

```python
# List platforms
platforms = session.get(f"{BASE_URL}/platforms").json()

# Create a Discord platform
session.post(f"{BASE_URL}/platforms", json={
    "platform_type": "discord",
    "token": "your-bot-token",
    "application_id": "your-app-id"
})

# Restart a platform adapter
session.post(f"{BASE_URL}/platforms/{platform_id}/restart")
```

### Chat API

The chat endpoint is the primary way to interact with the AI agent:

```python
# Send a message
response = session.post(f"{BASE_URL}/chat", json={
    "message": "Explain the Rust borrow checker",
    "conversation_id": None  # Optional: continue an existing conversation
}).json()

# Get chat history
history = session.get(f"{BASE_URL}/chat/history").json()

# Clear chat history
session.delete(f"{BASE_URL}/chat/history")
```

JavaScript equivalent:

```javascript
// Send a message
const response = await client.post("/chat", {
  message: "Explain the Rust borrow checker",
  conversation_id: null,
});

// Get chat history
const history = await client.get("/chat/history");

// Clear chat history
await client.delete("/chat/history");
```

### Conversations API

For more structured conversation management:

```python
# Create a new conversation
conv = session.post(f"{BASE_URL}/conversations", json={
    "title": "Rust Learning"
}).json()

conversation_id = conv["id"]

# Add a message to the conversation
session.post(f"{BASE_URL}/conversations/{conversation_id}/messages", json={
    "role": "user",
    "content": "What are Rust's ownership rules?"
})

# Get all messages in the conversation
messages = session.get(f"{BASE_URL}/conversations/{conversation_id}/messages").json()

# List all conversations
conversations = session.get(f"{BASE_URL}/conversations").json()

# Delete a conversation
session.delete(f"{BASE_URL}/conversations/{conversation_id}")
```

### Agent Status

Check whether the agent is currently processing a request:

```python
status = session.get(f"{BASE_URL}/agent/status").json()
print(status)
```

### Tools Listing

Get a list of all available tools, including built-in tools and MCP-provided tools:

```python
tools = session.get(f"{BASE_URL}/tools").json()
for tool in tools:
    print(f"- {tool['name']}: {tool.get('description', 'No description')}")
```

## WebSocket: Real-time Logs

Ruri provides a WebSocket endpoint for streaming real-time log messages. This is useful for building dashboards, monitoring tools, or custom UIs.

### Connecting

Connect to `ws://localhost:3000/api/ws/logs`:

#### Python (websockets)

```python
import asyncio
import json
import websockets

async def listen_to_logs():
    # Note: You need to pass the session cookie for authentication
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

#### JavaScript (native WebSocket)

```javascript
const ws = new WebSocket("ws://localhost:3000/api/ws/logs");

ws.onopen = () => {
  console.log("Connected to Ruri log stream");
};

ws.onmessage = (event) => {
  const logEntry = JSON.parse(event.data);
  const { level, message, timestamp } = logEntry;
  console.log(`[${timestamp}] [${level.toUpperCase()}] ${message}`);
};

ws.onerror = (error) => {
  console.error("WebSocket error:", error);
};

ws.onclose = () => {
  console.log("Disconnected from log stream");
};
```

### Log Message Format

Each message is a JSON object:

```json
{
  "level": "info",
  "message": "Agent started processing",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

The `level` field can be one of: `trace`, `debug`, `info`, `warn`, `error`.

## Error Handling

All API errors follow a consistent format:

```json
{
  "error": "Error message describing what went wrong"
}
```

### Common Status Codes

| Status | Meaning | Action |
| --- | --- | --- |
| `200` | Success | Parse the response body |
| `201` | Created | Resource was created successfully |
| `400` | Bad Request | Check your request body for missing or invalid fields |
| `401` | Unauthorized | Log in again — your session may have expired |
| `403` | Forbidden | You don't have permission for this action |
| `404` | Not Found | Check the resource ID or endpoint path |
| `500` | Server Error | Check Ruri's logs for details (`RUST_LOG=debug`) |

### Python Error Handling Pattern

```python
def safe_api_call(method, url, **kwargs):
    """Make an API call with proper error handling."""
    try:
        response = session.request(method, url, **kwargs)

        if response.status_code == 401:
            # Session expired — re-authenticate
            session.post(f"{BASE_URL}/auth/login", json={
                "username": "ruri",
                "password": "ruri"
            })
            # Retry the original request
            response = session.request(method, url, **kwargs)

        response.raise_for_status()
        return response.json()
    except requests.exceptions.HTTPError as e:
        error_data = e.response.json() if e.response.headers.get("content-type", "").startswith("application/json") else {}
        print(f"API Error ({e.response.status_code}): {error_data.get('error', str(e))}")
        raise
    except requests.exceptions.ConnectionError:
        print("Cannot connect to Ruri — is the server running?")
        raise

# Usage
providers = safe_api_call("GET", f"{BASE_URL}/providers")
```

### JavaScript Error Handling Pattern

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
      // Session expired — re-authenticate
      await client.post("/auth/login", {
        username: "ruri",
        password: "ruri",
      });
      // Retry the original request
      const config = { method, url: path };
      if (data) config.data = data;
      const response = await client(config);
      return response.data;
    }

    const status = error.response?.status;
    const message = error.response?.data?.error || error.message;
    console.error(`API Error (${status}): ${message}`);
    throw error;
  }
}

// Usage
const providers = await safeApiCall("get", "/providers");
```

## Full Example: Chat Client Script

Here's a complete Python script that logs in and sends a chat message:

```python
#!/usr/bin/env python3
"""Simple Ruri chat client."""
import requests
import sys

BASE_URL = "http://localhost:3000/api"

def main():
    session = requests.Session()

    # Log in
    res = session.post(f"{BASE_URL}/auth/login", json={
        "username": "ruri",
        "password": "ruri"
    })
    res.raise_for_status()
    print("✓ Logged in")

    # Send a message
    message = " ".join(sys.argv[1:]) or "Hello, Ruri!"
    res = session.post(f"{BASE_URL}/chat", json={
        "message": message
    })
    res.raise_for_status()
    data = res.json()

    # Print the response
    if "choices" in data and data["choices"]:
        content = data["choices"][0].get("message", {}).get("content", "")
        print(f"\nRuri: {content}")
    else:
        print(f"\nResponse: {data}")

if __name__ == "__main__":
    main()
```

## Full API Reference

For the complete list of endpoints, request/response formats, and details, see the [API Reference](/api).
