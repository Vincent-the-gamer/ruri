---
layout: doc
title: "Computer Use"
lastUpdated: true
---

# Computer Use

Computer Use is a powerful feature that allows the AI agent to execute commands and interact with the system. It includes safety controls through runtime modes, admin privileges, and workspace management.

## Overview

When Computer Use is enabled, the AI agent gains access to additional tools:

- **`bash`** — Execute shell commands on the host system
- **Python tool** — Run Python scripts
- **Shell tool** — Enhanced shell execution with workspace awareness

These tools are designed for scenarios where the agent needs to:

- Run build commands and tests
- Install dependencies
- Execute scripts and programs
- Interact with the operating system

## Runtime Modes

Computer Use has three runtime modes that control the level of access the agent has:

| Mode       | Description                                              |
| ---------- | -------------------------------------------------------- |
| `None`     | Computer Use is disabled; no shell/Python tools available |
| `Local`    | Commands execute directly on the host system              |
| `Sandbox`  | Commands execute in an isolated sandboxed environment     |

### None

The default mode. The `bash`, Python, and Shell tools are not available to the agent. This is the safest mode and is recommended when you don't need the agent to execute system commands.

### Local

Commands execute directly on the host system with the same permissions as the Ruri server process. This provides full flexibility but comes with security considerations.

::: warning
In Local mode, the AI agent can execute any command that the Ruri process has permission to run. Only use this mode in trusted environments.
:::

### Sandbox

Commands execute in an isolated environment that restricts what the agent can do. This is the recommended mode for most use cases, providing a balance between capability and security.

In Sandbox mode:

- File system access is restricted to the workspace directory
- Network access may be limited
- System-level commands are restricted

## Admin Privilege System

The admin privilege system controls who can enable or configure Computer Use features:

- **Admin users** can change Computer Use settings, including runtime mode
- **Regular users** can use Computer Use tools when enabled but cannot change settings
- Computer Use mode changes require admin authentication

## Workspace Management

Computer Use operates within a workspace context. The workspace defines:

- **Working directory** — Where commands are executed by default
- **Allowed paths** — Which directories the agent can access
- **Environment variables** — Variables available to executed commands

### Configuring the Workspace

Workspace settings can be configured through:

- The Web UI settings page
- [Config Profiles](/config-profiles) — each profile can specify a different workspace
- Session variables via the `/set` command

## Enabling Computer Use

### Via Web UI

1. Navigate to the **Settings** page
2. Find the Computer Use section
3. Select the desired runtime mode (Local or Sandbox)
4. Set the workspace directory
5. Save the configuration

### Via Config Profile

Include Computer Use settings in your [Config Profile](/config-profiles):

```yaml
computer_use:
  enabled: true
  mode: "sandbox"
  workspace: "/path/to/workspace"
```

## Safety Best Practices

1. **Use Sandbox mode** when possible — It provides the best balance of capability and security
2. **Restrict workspace access** — Only allow access to directories the agent needs
3. **Monitor agent actions** — Use the [Logging](#logging) feature to track what commands the agent executes
4. **Set up admin controls** — Ensure only trusted users can enable or modify Computer Use settings
5. **Review tool calls** — The agent's tool calls are visible in the conversation, allowing you to review what was executed

## Logging

All Computer Use tool invocations are logged through Ruri's LogManager. You can view real-time logs through:

- **Web UI** — The log viewer shows command executions in real-time
- **WebSocket** — Connect to `/api/ws/logs` for programmatic log monitoring
- **Log files** — Commands are recorded in the server log files

This provides full auditability of what the agent has done on your system.
