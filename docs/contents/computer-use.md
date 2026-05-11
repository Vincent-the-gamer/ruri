---
layout: doc
title: "Computer Use"
lastUpdated: true
---

# Computer Use

Computer Use lets the AI take real actions on your computer — running commands, installing packages, executing scripts, and more. It's like giving the AI hands to work with your system.

## What Computer Use Enables

When enabled, the AI can:

- **Run shell commands** — Execute any command you could type in a terminal
- **Build and test projects** — Run `cargo build`, `npm test`, `python manage.py test`, etc.
- **Install dependencies** — Run `npm install`, `pip install`, and similar commands
- **Manage files** — Beyond reading and writing, the AI can move, rename, and organize files via commands
- **Interact with the OS** — Check system info, manage processes, network diagnostics

### When to Use Computer Use

- **"Run my test suite and tell me if anything fails"**
- **"Install the dependencies for this project"**
- **"Start the development server"**
- **"Check what processes are using port 3000"**
- **"Build the project and show me any errors"**

### When You Don't Need It

For simple chat, file reading, or web search, you don't need Computer Use. Keep it off unless you want the AI to actually execute commands on your system.

## Runtime Modes

Computer Use offers three runtime modes with different safety levels:

| Mode               | Description                                                  | Best For                          |
| ------------------ | ------------------------------------------------------------ | --------------------------------- |
| **None**           | Computer Use is disabled                                     | Simple chat sessions              |
| **AIO Sandbox** ⭐ | Commands run in an isolated Docker container via AIO Sandbox | Daily use (recommended)           |
| **Local**          | Commands run directly on your system                         | When full system access is needed |

### AIO Sandbox Mode (Recommended)

AIO Sandbox runs commands inside an isolated Docker container, communicating with Ruri via HTTP API. This provides the strongest security boundary:

- 🔒 Commands execute in an isolated container, isolated from the host system
- 📦 The AI can use sandbox-specific tools: Shell, Read File, Write File, List Directory
- 🌐 Sandbox endpoint is configurable, supporting remote sandbox instances
- 🛡️ Host filesystem and system commands are not directly accessible

To use AIO Sandbox, you need to deploy the [AIO Sandbox](https://github.com/agent-infra/sandbox) service first, then configure the sandbox endpoint in Ruri.

### Local Mode

Commands run directly on your system with full access:

- The AI can do anything your user account can do
- Full access to all files, network, and system commands
- Maximum flexibility, but requires trust

::: warning
In Local mode, the AI can execute any command your system allows. Only use this mode if you trust the environment and carefully review the AI's actions.
:::

## Enabling Computer Use

### Via Web UI

1. Go to **Settings** in the sidebar
2. Find the **Computer Use** section
3. Choose your runtime mode:
   - **None** — Disable Computer Use
   - **AIO Sandbox** — Isolated Docker container (recommended)
   - **Local** — Full system access
4. If AIO Sandbox is selected, configure the **Sandbox Endpoint** (e.g. `http://localhost:8080`)
5. Set the **workspace directory** — This is where commands will run by default
6. Save the configuration

### AIO Sandbox Configuration

AIO Sandbox connects to a remote sandbox service via HTTP API. The configuration is straightforward:

| Setting      | Description             | Default                 |
| ------------ | ----------------------- | ----------------------- |
| **Endpoint** | AIO Sandbox service URL | `http://localhost:8080` |

When AIO Sandbox mode is selected but no endpoint is configured, Ruri will fall back to basic local file tools and log a warning.

::: tip
You can deploy the AIO Sandbox service on the same machine or a remote server. For team environments, a shared sandbox instance allows multiple users to safely run commands without affecting the host system.
:::

### Via Config Profile

You can include Computer Use settings in your [Config Profile](/config-profiles). This lets you have:

- A "Development" profile with Computer Use enabled in Sandbox mode
- A "Casual Chat" profile with Computer Use disabled
- Quick switching between them without changing settings manually

## What You'll See

When the AI uses Computer Use, you'll see each command it runs right in the conversation:

1. The AI decides it needs to run a command
2. A tool call appears in the chat showing the exact command
3. The output is displayed
4. The AI uses the output to continue helping you

This gives you full visibility into what the AI is doing on your system.

## Safety Tips

- **Start with Sandbox mode** — It's the safest option and handles most development tasks
- **Set a specific workspace** — Only give the AI access to the directory it needs
- **Review commands** — Check what the AI runs, especially in Local mode
- **Disable when not needed** — Turn off Computer Use for simple chat sessions
- **Use profiles** — Keep Computer Use off by default and only enable it in specific [Config Profiles](/config-profiles)

::: warning
All command executions are logged. You can review logs in the Web UI to see what the AI has done on your system.
:::
