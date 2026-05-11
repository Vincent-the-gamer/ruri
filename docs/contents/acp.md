---
layout: doc
title: "ACP Server"
lastUpdated: true
---

# ACP Server — Ruri in Your IDE

Want Ruri's AI power right inside your code editor? The **Agent Client Protocol (ACP)** lets you use Ruri as an AI assistant in your IDE — with full access to your configured tools, skills, and model providers.

## Supported IDEs

ACP works with editors that support the Agent Client Protocol:

- **Zed** — High-performance code editor
- **JetBrains** — IntelliJ IDEA, PyCharm, WebStorm, and more
- **Any ACP-compatible client**

## Quick Setup

### Step 1: Enable ACP in Ruri

1. Open the Ruri Web UI and go to **Settings**
2. Find the ACP section and enable it
3. Choose which tools the ACP server should have access to
4. Save the configuration

::: tip
You can also enable ACP through your [Config Profile](/config-profiles) settings.
:::

### Step 2: Configure Your IDE

#### Zed

Add the following to your Zed settings:

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

Replace `/path/to/ruri` with the actual path to your Ruri binary.

#### JetBrains

1. Open **Settings** → **Tools** → **AI Assistant**
2. Add an external agent server
3. Set the command to the path of your Ruri binary
4. Add `--acp` as an argument
5. Save and restart the IDE if needed

### Step 3: Start Using It

Once configured, Ruri appears as an AI assistant in your editor. Ask it to:

- Explain code
- Refactor functions
- Write tests
- Debug issues
- Generate documentation

The AI has access to all your configured tools, skills, and knowledge bases!

## What Works in ACP Mode

When connected via ACP, Ruri brings your full configuration into the IDE:

- **Your model provider** — Use whichever AI model you've configured
- **Your persona** — The AI's personality follows your settings
- **Your skills** — Active skills from your config profile are available
- **File tools** — Read, write, edit, and search files
- **Knowledge base** — If active in your profile, the AI can search your documents
- **Shell commands** — If Computer Use is enabled, the AI can run commands

::: tip
Make sure Computer Use is enabled in your [Config Profile](/config-profiles) if you want the AI to execute shell commands through the IDE.
:::

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

### ACP Server Not Responding

- Make sure Ruri is installed and the binary path in your IDE settings is correct
- Verify the binary has execute permissions (`chmod +x ruri` on macOS/Linux)
- Check that your model provider is configured and the API key is valid

### Tools Not Available

- Check which tools are enabled in the active [Config Profile](/config-profiles)
- Make sure Computer Use is enabled if you need shell commands
- Verify that skills aren't restricting tool access via `allowed_tools`

### AI Not Responding

- Confirm the model provider is active and the API key has credits
- Check your network connection to the model provider
- Try chatting via the Web UI first to verify the provider works

## Security Notes

- The ACP server runs with the same permissions as the Ruri process
- Tool permissions follow your [Config Profile](/config-profiles) settings
- Consider creating a restricted profile with limited tool access for ACP use
- Only enable shell command tools in trusted environments
