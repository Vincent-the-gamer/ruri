<div align="center">
    <img src=".github/ruri-avatar.png" style="width: 120px;" alt="Ruri Logo"/>
    <h1>Ruri 琉璃</h1>
    <p>
        <b>A customizable AI Agent, written in Rust + Vue.</b>
    </p>
    <p>
        <a href="https://github.com/Vincent-the-gamer/ruri/releases"><img src="https://img.shields.io/github/v/release/Vincent-the-gamer/ruri?style=flat-square" alt="Release"/></a>
        <a href="./COPYING"><img src="https://img.shields.io/badge/license-GPLv3-blue.svg?style=flat-square" alt="License"/></a>
        <a href="https://ruri.vince-g.xyz/"><img src="https://img.shields.io/badge/docs-online-green.svg?style=flat-square" alt="Docs"/></a>
    </p>
    <p><a href="README.md">English</a> | <a href="README.zh-CN.md">中文</a></p>
</div>

> [!IMPORTANT]
> This project is in beta.

---

## 📖 Documentation

Full documentation is available at **[ruri.vince-g.xyz](https://ruri.vince-g.xyz/)**.

| Section                                                     | Description                                 |
| ----------------------------------------------------------- | ------------------------------------------- |
| [Getting Started](https://ruri.vince-g.xyz/getting-started) | Installation & first launch guide           |
| [Model Providers](https://ruri.vince-g.xyz/providers)       | Configure OpenAI, Anthropic, Gemini, Ollama |
| [Built-in Tools](https://ruri.vince-g.xyz/tools)            | File ops, shell, web search, image analysis |
| [Skills](https://ruri.vince-g.xyz/skills)                   | Create custom AI behaviors                  |
| [Personas](https://ruri.vince-g.xyz/personas)               | Customize AI personality                    |
| [MCP Client](https://ruri.vince-g.xyz/mcp)                  | Connect external tool servers               |
| [Knowledge Base](https://ruri.vince-g.xyz/knowledge-base)   | RAG document search                         |
| [Chat Platforms](https://ruri.vince-g.xyz/platforms)        | DingTalk, Discord, WeChat, OneBot12         |
| [Computer Use](https://ruri.vince-g.xyz/computer-use)       | Sandboxed command execution                 |
| [API Reference](https://ruri.vince-g.xyz/api)               | Complete REST API documentation             |
| [Developer Guide](https://ruri.vince-g.xyz/dev/)            | Build, integrate, and contribute            |

---

## ✨ Features

### 🤖 Multi-Provider AI Chat

Connect to multiple AI model providers simultaneously — **OpenAI**, **Anthropic (Claude)**, **Google Gemini**, **DeepSeek**, **Ollama** (local), and any OpenAI-compatible API. Switch between models instantly with [Config Profiles](https://ruri.vince-g.xyz/config-profiles).

### 🔧 Built-in Tools

Give the AI real capabilities beyond chat:

- **File Operations** — Read, write, create, edit, and delete files
- **Shell Execution** — Run commands in isolated Docker sandboxes or directly on your system
- **Web Search** — DuckDuckGo, Tavily, Brave, Baidu, and more
- **Directory Browsing** — List and search files with regex and glob patterns
- **Image Analysis** — Multimodal support for vision-capable models

### 🎯 Skills System

Teach the AI new tricks with Markdown-based skills. Each skill defines custom behavior with YAML frontmatter, tool restrictions, model overrides, shell hooks, and auto-triggers. Package and share skills as ZIP files.

### 🧠 Personas

Customize the AI's personality — switch between a Code Expert, Creative Writer, Learning Tutor, or any persona you create. Define system prompts, swap personalities with a single click, and bind them to config profiles.

### 🔌 MCP Client

Connect to external **Model Context Protocol** servers via Stdio, SSE, WebSocket, or HTTP. Extend the AI with filesystem servers, database tools, browser automation, and more.

### 📚 Knowledge Base (RAG)

Upload PDFs, Excel spreadsheets, Word documents, and text files. The AI searches your documents using embedding models with optional reranking for better accuracy.

### 💬 Chat Platforms

Use Ruri from your favorite messaging apps:

- **DingTalk** (钉钉)
- **Discord**
- **WeChat** (微信) via Wechat ClawBot
- **OneBot12** (QQ, Lark, and more)

Hot-reload platform configs, restart individual adapters, and persist credentials automatically.

### 🛡️ AIO Sandbox

Execute AI commands safely inside isolated Docker containers via [AIO Sandbox](https://github.com/agent-infra/sandbox). Automatic retry with exponential backoff for transient errors.

### ⌨️ Command System

Handy slash commands for session control: `/help`, `/new`, `/reset`, `/sid`, `/whoami`, `/set`, `/stop`, and more.

### 🧩 ACP Server

Use Ruri as an AI coding assistant inside **Zed** and **JetBrains IDEs** via the Agent Client Protocol (JSON-RPC over stdio).

### ⚙️ Config Profiles

Create scene-based configuration profiles (Coding, Writing, Research, Casual) that bundle providers, personas, skills, platforms, knowledge bases, and proxy settings — switch everything with one click.

### 🌐 Proxy Support

HTTP, HTTPS, SOCKS4, and SOCKS5 proxy support with Clash-style rule-based routing. Per-profile proxy configuration with localhost bypass.

---

## 🚀 Quick Start

### Prerequisites

- An API key from an AI provider (OpenAI, Anthropic, DeepSeek, etc.) **or**
- [Ollama](https://ollama.com) installed for free local models

### Installation

1. Download the latest release from [GitHub Releases](https://github.com/Vincent-the-gamer/ruri/releases)
2. Extract the archive and add the binary to your `PATH`
3. Run the server:

```bash
# Show available options
ruri -h

# Start with default settings (port 3000)
ruri

# Use a custom port
ruri --port 8080

# Expose to the local network
ruri --remote
```

4. Open `http://localhost:3000` in your browser
5. Log in with default credentials (`ruri` / `ruri`) — you'll be prompted to change your password
6. Add a model provider on the **Providers** page and activate it
7. Start chatting!

> 📖 For detailed setup guides, see the [Getting Started](https://ruri.vince-g.xyz/getting-started) documentation.

---

## 🏗️ Tech Stack

| Layer    | Technology                                            |
| -------- | ----------------------------------------------------- |
| Backend  | Rust + Axum + Tokio + SQLx (SQLite)                   |
| Frontend | Vue 3 + TypeScript + Vite + Pinia + UnoCSS + vue-i18n |
| Docs     | VitePress + Teek Theme + Twoslash                     |

---

## 🔧 Development

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable, 2024 edition)
- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/installation) 9+

### Build from Source

```bash
# Clone the repository
git clone https://github.com/Vincent-the-gamer/ruri.git
cd ruri

# Build the frontend (required for embedded assets)
pnpm -C webui run build

# Build and run the backend
cargo run
```

### Development Mode

For frontend development with hot reload:

```bash
# Terminal 1: Start the Rust backend
cargo run

# Terminal 2: Start the Vite dev server
cd webui && pnpm dev
```

The Vite dev server proxies API requests to the backend and provides instant HMR for Vue components.

### Useful Commands

```bash
# Build for production
cargo build --release

# Run with verbose logging
RUST_LOG=debug cargo run

# Run all tests
cargo test

# Build and serve docs locally
cd docs && pnpm dev
```

### IDE Integration (ACP Mode)

```bash
cargo run -- --acp
```

Add to your Zed/JetBrains/... config:

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

---

## 📁 Project Structure

```
ruri/
├── src/                    # Rust backend
│   ├── main.rs             # Entry point, CLI args, server setup
│   ├── api/                # REST API handlers, routes, models
│   ├── agent/              # AI agent chat loop & tool dispatch
│   ├── acp/                # Agent Client Protocol (IDE integration)
│   ├── auth/               # Session auth + Argon2 password hashing
│   ├── command/            # Slash command system
│   ├── computer_use/       # Shell execution & sandbox
│   ├── conversation/       # Conversation persistence
│   ├── db/                 # SQLite initialization & pooling
│   ├── knowledge/          # RAG knowledge base
│   ├── logging/            # Logging infrastructure
│   ├── mcp/                # MCP client for external tool servers
│   ├── platform/           # Chat platform adapters
│   ├── provider/           # AI model provider abstractions
│   ├── transport/          # Transport layer (stdio, SSE, WS, HTTP)
│   ├── types/              # Shared types
│   └── web_dist/           # Embedded frontend build output
├── webui/                  # Vue 3 frontend
│   └── src/
│       ├── api/            # API client modules
│       ├── components/     # Reusable Vue components
│       ├── composables/    # Vue composables
│       ├── locales/        # i18n translations
│       ├── router/         # Vue Router config
│       ├── stores/         # Pinia state stores
│       ├── views/          # Page-level components
│       └── App.vue         # Root component
├── docs/                   # VitePress documentation
│   └── contents/           # English + Chinese docs
├── Cargo.toml              # Rust dependencies
└── pnpm-workspace.yaml     # pnpm workspace config
```

---

## 🤝 Contributing

Contributions are welcome! Here's how you can help:

1. **Report bugs** — Open an issue on [GitHub Issues](https://github.com/Vincent-the-gamer/ruri/issues)
2. **Suggest features** — Share your ideas for new capabilities
3. **Submit PRs** — Fix bugs, add features, or improve documentation
4. **Write skills** — Share useful AI skills with the community
5. **Improve docs** — Help translate or fix documentation errors

Please read the [Developer Guide](https://ruri.vince-g.xyz/dev/getting-started) before submitting code changes.

---

## 📄 License

[GPLv3 License](./COPYING)

Copyright (C) 2026-PRESENT Vincent-the-gamer <https://github.com/Vincent-the-gamer>
