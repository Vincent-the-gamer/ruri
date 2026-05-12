<div align="center">
    <img src=".github/ruri-avatar.png" style="width: 100px;"/>
    <h1>Ruri 琉璃</h1>
    <p><b>A customizable AI Agent, written in Rust + Vue.</b></p>
    <p><a href="README.md">English</a> | <a href="README.zh-CN.md">中文</a></p>
</div>

> [!IMPORTANT]
> This project is in the alpha test stage.

## Features

- [x] Model Provider - Anthropic Compatible, OpenAI Compatible, Custom
- [x] Tool Call
- [x] Skills
- [x] Web Search
- [x] ACP (Agent Client Protocol) Server
- [x] Persona
- [x] MCP (Model Context Protocol) Client
- [x] Command System
- [x] RAG-Based Knowledge Base (Embedding Model + Rerank Model) support
- [x] Chat - DingTalk
- [x] Chat - Discord
- [x] Chat - Personal WeChat(Wechat ClawBot)
- [x] Sandbox - AIO Sandbox (https://github.com/agent-infra/sandbox)
- [x] Chat History - Conversation management with filtering and search
- [ ] Chat - OneBot V12（A standardized bot application interface）

### Planned in the future

- Sub Agent
- Chat - Matrix
- Chat - VoceChat
- Chat - QQ
- Chat - WeCom
- Chat - Custom API(You can write your own program to talk to Ruri)

...

## Installation and Running

1. Download Alpha Version from release: [GitHub Releases](https://github.com/Vincent-the-gamer/ruri/releases)
2. Add it to PATH
3. Run `ruri`

```bash
# Show help
ruri -h

ruri
# Default port is 3000
ruri --port 8080
# Remote access, expose the server to the internet
ruri --remote
```

## Usage

Documentation: https://ruri.vince-g.xyz/

## Dev

```bash
cargo run
```

Open the web UI at `http://localhost:3000`.

### ACP (Agent Client Protocol) Server Config

Config example in Zed:

```json
{
  "agent_servers": {
    "ruri": {
      "type": "custom",
      "command": "/<path_to>/ruri",
      "args": ["--acp"]
    }
  }
}
```

## Preview

![preview](.github/preview.png)

## License

[GPLv3 License](./COPYING)

Copyright (C) 2026-PRESENT Vincent-the-gamer <https://github.com/Vincent-the-gamer>
