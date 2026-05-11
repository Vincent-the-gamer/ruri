---
layout: doc
title: "Developer Getting Started"
lastUpdated: true
---

# Developer Getting Started

This guide walks you through setting up a development environment for Ruri, building from source, and understanding the project architecture.

## Prerequisites

Before you begin, make sure you have the following installed:

| Tool | Version | Purpose |
| --- | --- | --- |
| [Rust](https://www.rust-lang.org/tools/install) | Stable (2024 edition) | Backend language |
| [Node.js](https://nodejs.org/) | 18+ | Frontend toolchain |
| [pnpm](https://pnpm.io/installation) | 9+ | Package manager (required — Ruri uses pnpm workspaces) |

::: warning
Ruri uses the **Rust 2024 edition**. Make sure your Rust toolchain is up to date:

```bash
rustup update stable
```
:::

### Optional Tools

- [Ollama](https://ollama.com) — For local AI model testing without an API key
- An API key from a model provider (OpenAI, Anthropic, DeepSeek, etc.) for end-to-end testing

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/Vincent-the-gamer/ruri.git
cd ruri
```

### 2. Build the Backend

```bash
cargo build
```

This compiles the Rust backend. The first build will take a while as it downloads and compiles all dependencies.

### 3. Install Frontend Dependencies

```bash
pnpm install
```

This installs dependencies for both the root workspace and the `webui/` package.

### 4. Build the Frontend

```bash
cd webui
pnpm build
```

The Vite build output is placed in `webui/dist/`. During a full `cargo build` or `cargo run`, the Rust build script copies these assets into `src/web_dist/` so they get embedded into the binary via `rust-embed`.

### 5. Run the Server

```bash
cargo run
```

Open `http://localhost:3000` in your browser. Log in with the default credentials (`ruri` / `ruri`).

::: tip
If you only changed frontend code and want a faster iteration cycle, you can skip `cargo run` and use Vite's dev server instead — see [Development Mode](#development-mode) below.
:::

## Development Mode

### Backend (Rust)

Run the backend with hot-rebuild:

```bash
cargo run
```

There is no built-in hot-reload for the Rust backend. After changing backend code, stop the server (`Ctrl+C`) and re-run `cargo run`. For faster recompilation during development, you can use:

```bash
# Install cargo-watch for auto-rebuild on file changes
cargo install cargo-watch
cargo watch -x run
```

### Frontend (Vue 3 + Vite)

Run the Vite dev server for the frontend with hot module replacement:

```bash
cd webui
pnpm dev
```

This starts a Vite dev server (default: `http://localhost:5173`) that proxies API requests to the Rust backend at `http://localhost:3000`. Any changes to Vue components, styles, or TypeScript files are reflected instantly in the browser.

::: info
You need both the Rust backend **and** the Vite dev server running simultaneously during frontend development. Start `cargo run` in one terminal, then `cd webui && pnpm dev` in another.
:::

### CLI Flags

Ruri supports several command-line flags for development:

| Flag | Short | Description |
| --- | --- | --- |
| `--port <PORT>` | `-p` | Set the server port (default: `3000`) |
| `--remote` | `-r` | Bind to `0.0.0.0` (accessible from the network) |
| `--acp` | `-a` | Start in ACP mode (stdio transport for IDE integration) |
| `--acp-config <PATH>` | `-c` | Override ACP config file path |

```bash
# Run on a custom port
cargo run -- --port 8080

# Make accessible from other machines on the network
cargo run -- --remote

# Run in ACP mode (for IDE integration testing)
cargo run -- --acp
```

## Project Structure

Understanding the codebase layout helps you navigate and contribute effectively.

```
ruri/
├── src/                    # Rust backend
│   ├── main.rs             # Entry point, server setup, CLI arg parsing
│   ├── api/                # REST API handlers, routes, models, state
│   │   ├── handlers.rs     # Route handler implementations
│   │   ├── mod.rs          # Router creation
│   │   ├── models.rs       # API request/response types
│   │   └── state.rs        # Shared application state (AppState)
│   ├── agent/              # AI agent logic (chat loop, tool dispatch)
│   ├── acp/                # Agent Client Protocol server (IDE integration)
│   ├── auth/               # Authentication (session, password hashing)
│   ├── command/            # Command system (/help, /status, etc.)
│   ├── computer_use/       # Shell command execution & sandbox
│   ├── conversation/       # Conversation database & history
│   ├── db/                 # SQLite database initialization & pooling
│   ├── knowledge/          # RAG knowledge base (embedding, search, file parsing)
│   ├── logging/            # Logging infrastructure & log manager
│   ├── mcp/                # MCP client (connect to external tool servers)
│   ├── platform/           # Chat platform adapters (DingTalk, Discord, WeChat)
│   ├── provider/           # AI model provider abstractions
│   ├── tools/              # Built-in tools (read_file, write_file, web_search, etc.)
│   ├── transport/          # Transport layer for MCP (stdio, SSE, WebSocket)
│   ├── types/              # Shared types (ChatMessage, ContentPart, etc.)
│   └── web_dist/           # Embedded frontend build output (auto-generated)
├── webui/                  # Vue 3 frontend
│   ├── src/
│   │   ├── api/            # API client modules
│   │   ├── components/     # Vue components
│   │   ├── composables/    # Vue composables (reusable logic)
│   │   ├── locales/        # i18n translation files
│   │   ├── router/         # Vue Router configuration
│   │   ├── stores/         # Pinia state stores
│   │   ├── types/          # TypeScript type definitions
│   │   ├── views/          # Page-level Vue components
│   │   ├── App.vue         # Root component
│   │   ├── main.ts         # Frontend entry point
│   │   └── style.css       # Global styles
│   ├── public/             # Static assets
│   ├── index.html          # HTML entry point
│   ├── vite.config.ts      # Vite configuration
│   └── uno.config.ts       # UnoCSS configuration
├── docs/                   # Documentation (VitePress)
│   └── contents/           # Doc pages & VitePress config
├── Cargo.toml              # Rust dependencies
├── Cargo.lock              # Rust dependency lockfile
├── package.json            # Workspace root package.json
├── pnpm-workspace.yaml     # pnpm workspace configuration
└── pnpm-lock.yaml          # Frontend dependency lockfile
```

### Key Architectural Concepts

**Backend (Rust + Axum)**

- The backend uses [Axum](https://github.com/tokio-rs/axum) as the HTTP framework with [Tokio](https://tokio.rs) as the async runtime
- `AppState` is the central shared state, held in an `Arc` and passed to all handlers
- SQLite (via [SQLx](https://github.com/launchbadge/sqlx)) is used for persistent storage — conversations, MCP configs, knowledge base
- The frontend is embedded into the binary at compile time via `rust-embed`, so the final binary is self-contained
- Session-based authentication with [Argon2](https://en.wikipedia.org/wiki/Argon2) password hashing

**Frontend (Vue 3 + Vite + UnoCSS)**

- [Vue 3](https://vuejs.org/) with the Composition API
- [Vite](https://vitejs.dev/) for build tooling and dev server
- [UnoCSS](https://unocss.dev/) for utility-first CSS
- [Pinia](https://pinia.vuejs.org/) for state management
- [Vue Router](https://router.vuejs.dev/) for routing
- [vue-i18n](https://kazupon.github.io/vue-i18n/) for internationalization
- [Axios](https://axios-http.com/) for HTTP API calls

## Environment Variables

Ruri uses the standard `RUST_LOG` environment variable to control log verbosity (via `tracing-subscriber`):

```bash
# Show only warnings and errors
RUST_LOG=warn cargo run

# Show info-level logs (default)
RUST_LOG=info cargo run

# Show debug logs for the entire application
RUST_LOG=debug cargo run

# Show debug logs only for a specific module
RUST_LOG=ruri::agent=debug cargo run

# Show trace-level logs for everything
RUST_LOG=trace cargo run
```

::: warning
`RUST_LOG=trace` produces a large amount of output. Use module-specific filters like `RUST_LOG=ruri::api=debug` to focus on what you need.
:::

## Common Development Tasks

### Adding a New API Endpoint

1. Define the route in `src/api/mod.rs`
2. Add the handler function in `src/api/handlers.rs`
3. Add request/response types in `src/api/models.rs` if needed
4. Rebuild and test with `cargo run`

### Adding a New Frontend Page

1. Create a new `.vue` file in `webui/src/views/`
2. Add the route in `webui/src/router/`
3. Add any API client functions in `webui/src/api/`
4. Add a Pinia store in `webui/src/stores/` if the page manages state
5. Run `cd webui && pnpm dev` to test with hot reload

### Running Tests

```bash
# Run all Rust tests
cargo test

# Run tests for a specific module
cargo test -p ruri --lib agent
```

::: info
Frontend tests are not yet set up in the current project structure. Contributions to add Vitest or Cypress are welcome!
:::

## Next Steps

- [API Usage Guide](/dev/api-usage) — Learn how to use Ruri's REST API programmatically
- [Integration Guide](/dev/integration) — Integrate Ruri into your own applications
- [API Reference](/api) — Complete endpoint documentation
