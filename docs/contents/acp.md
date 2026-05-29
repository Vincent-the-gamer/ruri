---
layout: doc
title: "ACP Server"
lastUpdated: true
---

# ACP Server — Ruri in Your IDE

Want Ruri's AI power right inside your code editor? The **Agent Client Protocol (ACP)** lets you use Ruri as an AI assistant in your IDE — with full access to your configured tools, skills, and model providers.

## Supported IDEs

| IDE                   | Description                                          |
| --------------------- | ---------------------------------------------------- |
| **Zed**               | High-performance code editor with native ACP support |
| **JetBrains**         | IntelliJ IDEA, PyCharm, WebStorm, and more           |
| **Other ACP clients** | Any tool that supports the Agent Client Protocol     |

![ACP Configuration](/ruri-pics/en/acp-config.png)

## Setup Guide

### Using Ruri in Zed

1. Make sure Ruri is installed and available (the `ruri --acp` command should run successfully)
2. Open Zed's settings file
3. Add the following to the `agent_servers` section:

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

4. Replace `/path/to/ruri` with the actual path to your Ruri binary
5. After saving, Ruri will be available as an AI agent in Zed

### Using Ruri in JetBrains

1. Make sure Ruri is installed and available
2. Open the AI Assistant settings in your JetBrains IDE
3. Configure Ruri as an external agent server
4. Set the command to the path of your Ruri binary, and add `--acp` as an argument
5. Save and you're ready to use it

## What You Can Do in Your IDE

When using Ruri in your IDE, the AI has the same capabilities as in the Web UI:

- 📖 **Read code** — The AI can read project files and understand code structure
- ✏️ **Edit code** — The AI can directly modify and create files
- 🔍 **Search code** — The AI can search for code and files in your project
- 💻 **Execute commands** — If Computer Use is enabled, the AI can run build and test commands
- 🛠️ **Use skills** — All active skills from your current config profile are available
- 🔄 **Streaming responses** — The AI's output is streamed in real-time to your IDE
- 🌐 **Independent proxy** — ACP has its own proxy configuration for routing LLM requests

::: tip
When using Ruri in your IDE, make sure your [Config Profile](/config-profiles) settings match your needs. You can create a dedicated profile for IDE use.
:::

## How It Works

In IDE mode, Ruri runs as a background service:

1. Your IDE sends requests to Ruri
2. Ruri processes them using the current persona, skills, and tools
3. The AI generates a response (potentially using tools like file operations)
4. The result is returned to your IDE

From the AI's perspective, it doesn't matter whether the request comes from the Web UI or the IDE — the processing is the same.

## Recommended Setup

For the best IDE experience, create a dedicated [Config Profile](/config-profiles) for ACP:

1. **Name it** "IDE" or "Coding"
2. **Choose your best coding model** as the provider
3. **Set a Code Expert persona** for technical accuracy
4. **Enable relevant skills** like code review or testing
5. **Enable Computer Use** in Sandbox mode for build/test commands
6. **Limit tools** to only what's needed for coding tasks

This way, the AI in your IDE is specialized for development work, while your regular chat profile stays separate.

## Troubleshooting

### AI Not Responding in IDE

- Make sure the Ruri binary path is correct
- Try running `ruri --acp` manually in a terminal to check for errors
- Verify that Ruri has execute permissions

### Tools Not Available

- Check that the required tools are enabled in the current [Config Profile](/config-profiles)
- Computer Use (shell commands, etc.) needs to be enabled separately in the profile
- Check if skills' `allowed_tools` restrictions are limiting access to certain tools

### Model Not Responding

- Confirm that the provider is correctly configured and active
- Check that your API key is valid
- Verify your network connection is working

## Security Notes

- The ACP server runs with the same permissions as the Ruri process
- Tool permissions follow your [Config Profile](/config-profiles) settings
- Consider creating a restricted profile with limited tool access for ACP use
- Only enable shell command tools in trusted environments
