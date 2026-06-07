# AGENTS.md - Ruri

Agent entrypoint for this repository; Use this file for repo-wide rules only.

## Project Knowledge

Ruri is a customizable AI agent with a web-based chat interface.

- **Tech Stack**:
  - Backend: Rust (edition 2024), async runtime with Tokio + Axum
  - Frontend: Vue 3 + TypeScript + Vite, UnoCSS for styling
  - Docs: VitePress
  - Package manager: pnpm (workspace monorepo)

- **File Structure**:
  - `docs/`: VitePress documentation of this project.
  - `src/`: Backend source code, written in Rust.
  - `webui/`: Frontend source code, written in Vue 3 + TypeScript.
  - `vendor/`: Vendored crate patches (e.g., `agent-client-protocol-schema`).

## Architecture Overview

### Backend (`src/`)

Entry point: `src/main.rs`. Starts an Axum HTTP server serving both the API and the embedded frontend (via `rust-embed`).

| Module          | Purpose                                                                                 |
| --------------- | --------------------------------------------------------------------------------------- |
| `agent/`        | Agent loop runner, built-in tools, tool executor, skill system, sub-agent orchestration |
| `api/`          | REST API handlers, request/response models, shared state                                |
| `auth/`         | Authentication (password hashing with Argon2)                                           |
| `command/`      | Built-in slash commands for the chat interface                                          |
| `computer_use/` | Computer-use sandbox: workspace isolation, permissions, runtime config                  |
| `conversation/` | Conversation management and history                                                     |
| `db/`           | SQLite database layer (via SQLx)                                                        |
| `knowledge/`    | Knowledge base: document parsing (PDF, DOCX, Excel), text chunking                      |
| `logging/`      | Logging infrastructure                                                                  |
| `mcp/`          | Model Context Protocol (MCP) client support                                             |
| `metrics/`      | Metrics collection and reporting                                                        |
| `platform/`     | Multi-platform support (e.g., Discord via Serenity, WeChat)                             |
| `provider/`     | LLM provider adapters (OpenAI, Anthropic, Gemini)                                       |
| `transport/`    | Transport layer (ACP stdio transport)                                                   |
| `acp/`          | Agent Client Protocol implementation                                                    |
| `types/`        | Shared type definitions                                                                 |

### Frontend (`webui/src/`)

- **Framework**: Vue 3 with Composition API (`<script setup lang="ts">`)
- **State management**: Pinia stores (one store per domain: `agent`, `chat`, `auth`, `provider`, `mcp`, `skill`, etc.)
- **Routing**: Vue Router
- **i18n**: vue-i18n
- **Styling**: UnoCSS (utility-first CSS)
- **HTTP client**: Axios
- **Markdown rendering**: marked
- **Charts**: Chart.js

Key views: `Chat.vue`, `Providers.vue`, `Configs.vue`, `Dashboard.vue`, `Skills.vue`, `SubAgents.vue`, `KnowledgeBase.vue`, `McpConfig.vue`, etc.

## CLI Usage

```
ruri [OPTIONS]

Options:
  -a, --acp                Start in ACP mode (stdio transport)
  -c, --acp-config <PATH>  Override config file path (used in ACP mode)
  -r, --remote             Bind WebUI and API to 0.0.0.0 (accessible from network)
  -p, --port <PORT>        Port to listen on (default: 3000)
```

## Code Style

### Backend (Rust)

- No `#[allow(dead_code)]`
- No unused variables
- No dead code
- Prefer `thiserror` for library errors, `anyhow` for application-level errors
- Use `tracing` for logging (not `println!`)
- Async code: use Tokio runtime, `async-trait` where needed

### Frontend (Vue 3 + TypeScript)

- No unused variables
- No dead code
- No `any` type
- Always use Composition API with `<script setup lang="ts">`
- Use Pinia stores for shared state
- Use UnoCSS utility classes for styling (avoid inline styles)

### General

- Run `cargo clippy` and fix warnings before committing backend changes
- Run `vue-tsc --noEmit` to type-check frontend changes

## Forbidden Operations

- **CRITICAL**: Do NOT run dangerous shell commands: e.g. `rm -rf`, `sudo`, `git push --force`, etc.
- Do NOT commit secrets, API keys, or credentials
- Do NOT modify `Cargo.lock` or `pnpm-lock.yaml` manually

## Development Setup

### Prerequisites

- Rust (stable, edition 2024)
- Node.js + pnpm (package manager version `pnpm@11.5.0`)
- SQLite (for the database layer)

### Install Dependencies

```bash
# Install frontend dependencies (workspace-wide)
pnpm install
```

### Run Development Servers

```bash
# Frontend dev server (hot reload)
pnpm -C webui run dev

# Backend (build and run)
cargo run
```

## Build Ruri

Ruri uses `rust-embed` to embed static assets into the binary. You must build the frontend first.

```bash
# Build Frontend
pnpm -C webui run build
```

Then build the backend:

```bash
# Build Backend for development
cargo build

# Build Backend for production
cargo build --release
```

## Running Tests

```bash
# Run all Rust tests
cargo test

# Run tests for a specific module
cargo test -p ruri -- agent
cargo test -p ruri -- computer_use

# Frontend type-check (no dedicated test suite yet)
pnpm -C webui exec vue-tsc --noEmit
```

## Key Design Patterns

### Provider Pattern

LLM providers (OpenAI, Anthropic, Gemini) implement a common trait in `src/provider/mod.rs`. Each provider lives in its own file and handles:

- API request formatting
- Response parsing
- Streaming support
- Provider-specific configuration

### Tool System

Tools are defined in `src/agent/builtin_tools.rs` and executed via `src/agent/tool_executor.rs`. Each tool has a name, description, parameter schema, and an async execute function.

### Agent Loop

The agent runner (`src/agent/runner.rs`) orchestrates the conversation loop: send prompt → receive response → execute tool calls → send results → repeat until completion.

### Sub-Agent System

Sub-agents (`src/agent/subagent.rs`) allow spawning child agents for parallel or specialized tasks. Sub-agents share the same tool set but can have different system prompts and models.
