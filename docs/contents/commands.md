---
layout: doc
title: "Command System"
lastUpdated: true
---

# Command System

Ruri includes a built-in command system that allows you to interact with the agent and manage sessions directly from the chat input. Commands are prefixed with `/` and provide quick access to common actions.

## Available Commands

| Command                | Description                                    |
| ---------------------- | ---------------------------------------------- |
| `/help`                | Show available commands and version info       |
| `/sid`                 | Show current session info                      |
| `/reset`               | Reset the current conversation's LLM context   |
| `/new`                 | Create and switch to a new conversation        |
| `/set <key> <value>`   | Set a session variable                         |
| `/unset <key>`         | Remove a session variable                      |
| `/stop`                | Stop the currently running agent task          |
| `/dashboard_update`    | Update the WebUI (requires admin)              |

## Command Details

### `/help`

Displays a list of all available commands along with the current Ruri version information.

```
/help
```

### `/sid`

Shows information about the current session, including the session ID and active configuration.

```
/sid
```

### `/reset`

Resets the LLM context for the current conversation. This clears the conversation history from the model's memory, effectively starting a fresh context while keeping the conversation container.

Use this when:
- The conversation context becomes too long and responses degrade
- You want to start a new topic within the same conversation
- The model seems confused by earlier messages

```
/reset
```

### `/new`

Creates a new conversation and switches to it. This is useful when you want to start a completely fresh conversation with a clean slate.

```
/new
```

### `/set <key> <value>`

Sets a session variable that persists for the duration of the current conversation. Session variables can be used to dynamically configure agent behavior.

```
/set persona "Code Expert"
/set model "gpt-4o"
/set effort high
```

### `/unset <key>`

Removes a previously set session variable.

```
/unset persona
```

### `/stop`

Stops the currently running agent task. This is useful when:
- The agent is taking too long to respond
- The agent is executing an undesired action
- You want to interrupt a long-running tool call chain

```
/stop
```

### `/dashboard_update`

Triggers an update of the Web UI. This command requires admin privileges.

```
/dashboard_update
```

## Using Commands

Commands can be entered directly in the chat input field. They are processed before the message is sent to the AI model, so they execute immediately without involving the model.

::: tip
Commands are not sent to the AI model. They are handled locally by the Ruri server. If you want the model to know about a configuration change you've made, mention it in a regular message.
:::

## Programmatic Access

Commands can also be triggered via the chat API by including the command as the message content:

```bash
curl -X POST http://localhost:3000/api/chat \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{"message": "/new"}'
```
