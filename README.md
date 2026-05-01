# ruri

A customizable AI Agent, written in Rust + Vue.

## Features

- 🤖 Multi-provider support (OpenAI, Anthropic, Custom)
- 🛠 Tool execution framework with built-in tools
- 🎯 Modular skills system
- 🌐 Web UI with Vue frontend
- 📡 REST API
- 🔌 **ACP (Agent Client Protocol)** support — connect from Zed, JetBrains, and other ACP-compatible IDEs

## Quick Start

### Web UI + API Server

```bash
cargo run
```

Opens the web UI at `http://localhost:3000`.

### ACP Mode (Zed / JetBrains)

Run ruri in ACP mode to connect from ACP-compatible editors:

```bash
cargo run -- --acp
```

#### Provider Configuration

ACP mode reads provider settings from environment variables:

| Variable            | Description                                        |
| ------------------- | -------------------------------------------------- |
| `OPENAI_API_KEY`    | OpenAI API key (or any OpenAI-compatible endpoint) |
| `OPENAI_BASE_URL`   | Custom OpenAI-compatible base URL                  |
| `OPENAI_MODEL`      | Model name (default: `gpt-4o`)                     |
| `ANTHROPIC_API_KEY` | Anthropic API key                                  |
| `ANTHROPIC_MODEL`   | Model name (default: `claude-sonnet-4-20250514`)   |
| `CUSTOM_API_URL`    | Custom provider URL                                |
| `CUSTOM_API_KEY`    | Custom provider API key                            |
| `CUSTOM_MODEL`      | Custom provider model name                         |

#### Configure in Zed

Add to your Zed `settings.json`:

```json
{
  "agent": {
    "profiles": {
      "ruri": {
        "name": "Ruri",
        "agent_client_protocol": {
          "command": "path/to/ruri",
          "args": ["--acp"]
        }
      }
    }
  }
}
```

#### ACP Protocol Support

Ruri implements the following ACP methods:

- **Core**: `initialize`, `authenticate`
- **Session**: `session/new`, `session/prompt`, `session/cancel`, `session/load`, `session/close`, `session/resume`, `session/list`, `session/set_mode`, `session/set_config_option`
- **File System**: `fs/read_text_file`, `fs/write_text_file`
- **Terminal**: `terminal/create`, `terminal/output`, `terminal/release`, `terminal/wait_for_exit`, `terminal/kill`
- **Notifications**: `session/update` (agent messages, tool calls)
