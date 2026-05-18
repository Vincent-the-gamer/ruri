---
layout: doc
title: "API Reference"
lastUpdated: true
---

# API Reference

Ruri provides a comprehensive REST API for programmatic access to all features. All endpoints are relative to the base URL (default: `http://localhost:3000`).

## Authentication

Most API endpoints require authentication via a session cookie. Authenticate by logging in first.

### Login

```
POST /api/auth/login
```

**Request body:**

```json
{
  "username": "ruri",
  "password": "ruri",
  "remember_me": false
}
```

**Response:** A session cookie is set in the response headers, plus a JSON body:

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

### Logout

```
POST /api/auth/logout
```

### Get Current User

```
GET /api/auth/me
```

Returns the currently authenticated user's information including ID, username, avatar URL, and whether a password change is required.

### Change Password

```
POST /api/auth/change-password
```

**Request body:**

```json
{
  "old_password": "ruri",
  "new_password": "my-new-password"
}
```

### Update Username

```
PUT /api/auth/username
```

**Request body:**

```json
{
  "new_username": "my-new-username"
}
```

### Upload Avatar

```
POST /api/auth/avatar
```

**Request:** Multipart form data with an image file. Supported formats: PNG, JPEG, GIF, WebP. Max size: 2MB.

### Get Avatar

```
GET /api/auth/avatar/:user_id
```

Returns the avatar image for the specified user.

## Chat

### Send a Chat Message

```
POST /api/chat
```

**Request body:**

```json
{
  "message": "Hello, how are you?",
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

| Field                 | Type     | Description                                           |
| --------------------- | -------- | ----------------------------------------------------- |
| `message`             | string   | The chat message text                                 |
| `conversation_id`     | string?  | Continue an existing conversation                     |
| `images`              | array    | Base64-encoded images for multimodal models           |
| `files`               | array    | Attached files (PDF, DOCX, XLSX, TXT, etc.)           |
| `provider_id`         | string?  | Override the active provider for this request         |
| `session_id`          | string?  | Session identifier                                    |
| `temperature`         | number?  | Model temperature (0-2)                               |
| `max_tokens`          | number?  | Maximum tokens in the response                        |
| `knowledge_base_ids`  | array    | Knowledge bases to search                             |
| `tool_choice`         | string?  | `auto`, `none`, `required`, or a specific function    |
| `parallel_tool_calls` | boolean? | Whether the model can call multiple tools in parallel |

### Get Chat History

```
GET /api/chat/history
```

### Clear Chat History

```
DELETE /api/chat/history
```

### Stop Chat Generation

```
POST /api/chat/stop
```

**Request body:**

```json
{
  "session_id": "session-to-stop"
}
```

Stops an in-progress chat generation for the specified session.

## Conversations

See the [Chat History](/chat-history) page for a detailed overview of the conversation system.

### List Conversations

```
GET /api/conversations
```

**Query parameters (optional):**

| Parameter   | Description                                 |
| ----------- | ------------------------------------------- |
| `bot_name`  | Filter by bot name                          |
| `chat_type` | Filter by `group` or `private`              |
| `keyword`   | Search in title and chat ID (partial match) |

### Create Conversation

```
POST /api/conversations
```

**Request body:**

```json
{
  "bot_name": "my-bot",
  "chat_type": "private",
  "chat_id": "user-123",
  "title": "My Conversation"
}
```

### Get Conversation

```
GET /api/conversations/:id
```

### Delete Conversation

```
DELETE /api/conversations/:id
```

Deletes a conversation and **all its messages** (cascade delete). Returns `204 No Content` on success.

### Add Message to Conversation

```
POST /api/conversations/:id/messages
```

**Request body:**

```json
{
  "role": "user",
  "content": "Hello!"
}
```

The conversation's `updated_at` timestamp is automatically refreshed when a message is added.

### Get Conversation Messages

```
GET /api/conversations/:id/messages
```

Returns all messages in a conversation, ordered by `created_at` ascending.

## Providers

### List Providers

```
GET /api/providers
```

### Create Provider

```
POST /api/providers
```

**Request body:**

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

Provider types: `openai_compatible`, `anthropic`, `gemini`.

### Get Provider

```
GET /api/providers/:id
```

### Update Provider

```
PUT /api/providers/:id
```

**Request body:** Same as create, with fields to update.

### Delete Provider

```
DELETE /api/providers/:id
```

### Activate Provider

```
POST /api/providers/:id/activate
```

Sets the specified provider as the active provider for all chat interactions.

### Fetch Provider Models

```
POST /api/providers/fetch-models
```

Queries a provider's API to list all available models.

**Request body:**

```json
{
  "provider_type": "openai_compatible",
  "base_url": "https://api.openai.com/v1",
  "api_key": "sk-..."
}
```

**Response:**

```json
{
  "models": [
    { "id": "gpt-4o", "name": "GPT-4o" },
    { "id": "gpt-4o-mini", "name": "GPT-4o Mini" }
  ]
}
```

## Skills

### List Skills

```
GET /api/skills
```

### Add Skill

```
POST /api/skills
```

**Request body:**

```json
{
  "name": "my-skill",
  "content": "---\nname: my-skill\ndescription: My skill\n---\nSkill instructions here."
}
```

### Upload Skill Package

```
POST /api/skills/upload
```

**Request:** Multipart form data with a `file` field containing a ZIP archive.

```bash
curl -X POST http://localhost:3000/api/skills/upload \
  -H "Cookie: session=<your-session-cookie>" \
  -F "file=@skills.zip"
```

### Toggle Skill

```
PATCH /api/skills/:name
```

**Request body:**

```json
{
  "enabled": true
}
```

### Delete Skill

```
DELETE /api/skills/:name
```

## Tools

### List Tools

```
GET /api/tools
```

Returns a list of all available tools, including built-in tools and MCP-provided tools.

## Agent

### Get Agent Status

```
GET /api/agent/status
```

Returns the current status of the agent, including:

- Whether it's processing a request
- Active provider and model
- Skills and tools count
- Uptime in seconds
- Message count

## ACP

### Get ACP Configuration

```
GET /api/acp/config
```

### Update ACP Configuration

```
PUT /api/acp/config
```

**Request body:**

```json
{
  "active_provider_id": "provider-id",
  "active_skill_names": ["code-review", "summarize"],
  "active_knowledge_base_ids": ["kb-id"],
  "proxy_config": {}
}
```

## Computer Use

### Get Computer Use Configuration

```
GET /api/computer-use/config
```

### Update Computer Use Configuration

```
PUT /api/computer-use/config
```

**Request body:**

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

### Get Shell Command Blacklist

```
GET /api/computer-use/shell-blacklist
```

### Update Shell Command Blacklist

```
PUT /api/computer-use/shell-blacklist
```

**Request body:**

```json
{
  "blacklist": ["sudo ", "rm -rf", "format "]
}
```

## Web Search

### Get Web Search Configuration

```
GET /api/web-search/config
```

### Update Web Search Configuration

```
PUT /api/web-search/config
```

**Request body:**

```json
{
  "search_engine": "duckduckgo",
  "api_key": null,
  "max_results": 10,
  "enabled": true
}
```

## Config Profiles

### List Config Profiles

```
GET /api/profiles
```

### Get Config Profile

```
GET /api/profiles/:id
```

### Create Config Profile

```
POST /api/profiles
```

**Request body:**

```json
{
  "name": "Coding",
  "description": "For development work",
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

### Update Config Profile

```
PUT /api/profiles/:id
```

### Delete Config Profile

```
DELETE /api/profiles/:id
```

### Activate Config Profile

```
POST /api/profiles/:id/activate
```

### Deactivate Config Profile

```
POST /api/profiles/:id/deactivate
```

### Get Config Profile's Resolved Provider

```
GET /api/profiles/:id/provider
```

Returns the resolved provider for a config profile, including any embedded providers.

## Personas

```
GET    /api/personas           — List personas
GET    /api/personas/:id       — Get persona
POST   /api/personas           — Create persona
PUT    /api/personas/:id       — Update persona
DELETE /api/personas/:id       — Delete persona
```

**Create/Update request body:**

```json
{
  "name": "Code Expert",
  "description": "A senior software engineer",
  "prompt": "You are a senior software engineer..."
}
```

## Platforms

Platform CRUD endpoints follow the standard pattern:

```
GET    /api/platforms           — List platforms
POST   /api/platforms           — Create platform
GET    /api/platforms/:id       — Get platform
PUT    /api/platforms/:id       — Update platform
DELETE /api/platforms/:id       — Delete platform
```

### Restart Platform

```
POST /api/platforms/:id/restart
```

Restarts a specific platform adapter without restarting the entire Ruri server.

### WeChat QR Login

```
POST /api/platforms/weixin-qr/start    — Start QR code login flow
GET  /api/platforms/weixin-qr/status   — Check QR login status
```

## MCP Servers

```
GET    /api/mcp/servers         — List MCP servers
GET    /api/mcp/servers/:id     — Get MCP server
POST   /api/mcp/servers         — Create MCP server
PUT    /api/mcp/servers/:id     — Update MCP server
DELETE /api/mcp/servers/:id     — Delete MCP server
PATCH  /api/mcp/servers/:id     — Toggle MCP server enabled/disabled
```

**Create/Update request body:**

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

## Knowledge Base

```
GET    /api/knowledge-bases              — List knowledge bases
GET    /api/knowledge-bases/:id          — Get knowledge base
POST   /api/knowledge-bases              — Create knowledge base
PUT    /api/knowledge-bases/:id          — Update knowledge base
DELETE /api/knowledge-bases/:id          — Delete knowledge base
GET    /api/knowledge-bases/:id/documents         — List documents
POST   /api/knowledge-bases/:id/documents/upload  — Upload document
DELETE /api/knowledge-bases/:id/documents/:doc_id — Delete document
POST   /api/knowledge-bases/search        — Search knowledge bases
```

**Create knowledge base request body:**

```json
{
  "name": "My Knowledge Base",
  "description": "Project documentation",
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

**Search request body:**

```json
{
  "query": "What is the return policy?",
  "top_k": 5
}
```

## Commands

### List Built-in Commands

```
GET /api/commands
```

Returns all available slash commands with their metadata (name, description, usage, admin requirement, enabled status).

### Toggle Command Admin Requirement

```
PATCH /api/commands/:name/admin
```

**Request body:**

```json
{
  "require_admin": true
}
```

## System

### Restart System

```
POST /api/system/restart
```

Restarts the Ruri server. Requires admin privileges.

## Debug Session

### Get Debug Session

```
GET /api/debug-session
```

Returns the current debug session configuration, including providers, skills, persona, and other settings. Useful for development and testing.

### Update Debug Session

```
PUT /api/debug-session
```

Updates the debug session configuration. Accepts a partial debug session config object.

## WebSocket

### Real-time Logs

```
WS /api/ws/logs
```

Connect to this WebSocket endpoint to receive real-time log messages from the LogManager. This is the same data displayed in the Web UI's log viewer.

**Message format:**

```json
{
  "level": "info",
  "message": "Agent started processing",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## Error Responses

All endpoints return errors in the following format:

```json
{
  "error": "Error message describing what went wrong"
}
```

Common HTTP status codes:

| Status | Description                            |
| ------ | -------------------------------------- |
| 200    | Success                                |
| 201    | Created                                |
| 204    | No Content (successful deletion)       |
| 400    | Bad request — Invalid input            |
| 401    | Unauthorized — Authentication required |
| 403    | Forbidden — Insufficient permissions   |
| 404    | Not found                              |
| 500    | Internal server error                  |

## Authentication Details

Ruri uses session-based authentication with cookies:

1. **Login** — `POST /api/auth/login` with credentials
2. **Session cookie** — Returned in the response, automatically included in subsequent requests
3. **Default credentials** — `ruri` / `ruri` (change on first login)
4. **Password changes** — `POST /api/auth/change-password`
5. **Username updates** — `PUT /api/auth/username`
6. **Avatar upload** — `POST /api/auth/avatar` (multipart, max 2MB)

::: tip
When using `curl`, include the session cookie with `-H "Cookie: session=<value>"`. When using a browser or HTTP client library, cookies are typically handled automatically.
:::
