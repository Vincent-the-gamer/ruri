---
layout: doc
title: "ACP Server"
lastUpdated: true
---

# ACP Server

The **Agent Client Protocol (ACP)** Server allows Ruri to act as an agent server that can be integrated with IDEs and other tools that support the ACP standard.

## Overview

When running in ACP mode, Ruri communicates via stdio transport, making it compatible with:

- **Zed** — A high-performance code editor
- **JetBrains** — IDEs like IntelliJ IDEA, PyCharm, WebStorm, etc.
- **Other ACP-compatible clients** — Any tool that implements the Agent Client Protocol

This enables you to use Ruri's AI capabilities directly within your development environment, with full access to tools, skills, and model providers.

## Starting in ACP Mode

To start Ruri as an ACP server, use the `--acp` flag:

```bash
ruri --acp
```

When started with `--acp`, Ruri:

1. Communicates via stdio (standard input/output)
2. Processes ACP protocol messages
3. Does not start the Web UI server
4. Uses the active configuration profile

## Configuration

### Via Zed

To use Ruri as an agent server in Zed, add the following to your Zed settings:

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

Replace `/<path_to>/ruri` with the actual path to your Ruri binary.

### Via JetBrains

In JetBrains IDEs, configure Ruri as an external agent server through the AI assistant settings. Point the command to the Ruri binary with the `--acp` argument.

### ACP Configuration

You can manage ACP configuration through the API:

**Get ACP config:**

```bash
curl http://localhost:3000/api/acp/config \
  -H "Cookie: session=<your-session-cookie>"
```

**Update ACP config:**

```bash
curl -X PUT http://localhost:3000/api/acp/config \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{
    "enabled": true,
    "allowed_tools": ["read_file", "write_file", "edit_file", "bash"]
  }'
```

## How It Works

The ACP communication flow:

```
IDE/Client ←→ stdio ←→ Ruri ACP Server ←→ AI Model
```

1. The IDE sends a request via stdio using the ACP protocol
2. Ruri parses the request and creates a chat message
3. The message is processed through the active persona, skills, and tools
4. The AI model generates a response, potentially using tools
5. The response is sent back to the IDE via stdio

## Features Available in ACP Mode

When running as an ACP server, Ruri provides:

- **All model providers** — Use any configured provider
- **Built-in tools** — File operations, search, and optionally bash commands
- **Skills** — Active skills from the current config profile
- **Personas** — The active persona from the current profile
- **Knowledge base** — If configured and active in the profile

::: tip
Make sure to enable Computer Use in your [Config Profile](/config-profiles) if you want the agent to execute shell commands through the ACP connection.
:::

## Security Considerations

- The ACP server runs with the same permissions as the Ruri process
- Tool execution permissions follow the current [Config Profile](/config-profiles) settings
- Only enable the `bash` tool in trusted environments
- Use [Config Profiles](/config-profiles) to create a restricted profile for ACP use with limited tool access

## Troubleshooting

### ACP Server Not Responding

- Ensure Ruri is started with `--acp` flag
- Check that the binary path in your IDE configuration is correct
- Verify that the Ruri binary has execute permissions

### Tools Not Available

- Check that the tools are enabled in the active [Config Profile](/config-profiles)
- Ensure Computer Use is enabled for `bash` and shell tools
- Verify that skills aren't restricting available tools via `allowed_tools`

### Model Not Responding

- Verify that a model provider is configured and active
- Check API key validity
- Ensure the ACP server has network access to the model provider API
