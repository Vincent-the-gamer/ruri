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
  "password": "ruri"
}
```

**Response:** A session cookie is set in the response headers.

### Logout

```
POST /api/auth/logout
```

## Chat

### Send a Chat Message

```
POST /api/chat
```

**Request body:**

```json
{
  "message": "Hello, how are you?",
  "conversation_id": "optional-conversation-id"
}
```

### Get Chat History

```
GET /api/chat/history
```

### Clear Chat History

```
DELETE /api/chat/history
```

## Conversations

### List Conversations

```
GET /api/conversations
```

### Create Conversation

```
POST /api/conversations
```

**Request body:**

```json
{
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

### Get Conversation Messages

```
GET /api/conversations/:id/messages
```

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
  "api_url": "https://api.openai.com/v1",
  "api_key": "sk-...",
  "model": "gpt-4o"
}
```

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

Returns the current status of the agent, including whether it's processing a request.

## ACP

### Get ACP Configuration

```
GET /api/acp/config
```

### Update ACP Configuration

```
PUT /api/acp/config
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

## Authentication Endpoints

```
POST /api/auth/login    — Login
POST /api/auth/logout   — Logout
```

Additional auth endpoints may be available for password changes and username updates.

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

| Status | Description                           |
| ------ | ------------------------------------- |
| 200    | Success                               |
| 201    | Created                               |
| 400    | Bad request — Invalid input           |
| 401    | Unauthorized — Authentication required |
| 403    | Forbidden — Insufficient permissions  |
| 404    | Not found                             |
| 500    | Internal server error                 |

## Authentication Details

Ruri uses session-based authentication with cookies:

1. **Login** — `POST /api/auth/login` with credentials
2. **Session cookie** — Returned in the response, automatically included in subsequent requests
3. **Default credentials** — `ruri` / `ruri` (change on first login)
4. **Password changes** — Supported after initial login
5. **Username updates** — Supported through the auth endpoints

::: tip
When using `curl`, include the session cookie with `-H "Cookie: session=<value>"`. When using a browser or HTTP client library, cookies are typically handled automatically.
:::
