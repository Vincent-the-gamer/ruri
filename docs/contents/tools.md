---
layout: doc
title: "Built-in Tools"
lastUpdated: true
---

# Built-in Tools

Ruri comes with a set of built-in tools that the AI agent can use to interact with the filesystem, execute commands, and search the web. These tools are automatically available when a model supports tool calling.

## Overview

| Tool           | Description                                   |
| -------------- | --------------------------------------------- |
| `read_file`    | Read file contents, with optional line range  |
| `write_file`   | Write content to a file                       |
| `create_file`  | Create a new file                             |
| `edit_file`    | Make a targeted replacement in a file         |
| `list_directory` | List files and directories                  |
| `search_files` | Search by name pattern or content             |
| `bash`         | Execute shell commands (Computer Use mode)    |
| `web_search`   | Search the web for information                |

## File Operations

### `read_file`

Read the contents of a file, optionally specifying a line range.

**Parameters:**

| Parameter    | Type   | Required | Description                        |
| ------------ | ------ | -------- | ---------------------------------- |
| `path`       | string | Yes      | The file path to read              |
| `start_line` | number | No       | Starting line number (1-based)     |
| `end_line`   | number | No       | Ending line number (inclusive)     |

**Example usage by the agent:**

The agent might read a configuration file to understand the project structure:

```
read_file(path="config.toml")
read_file(path="src/main.rs", start_line=1, end_line=50)
```

### `write_file`

Write content to a file, overwriting any existing content.

**Parameters:**

| Parameter | Type   | Required | Description                  |
| --------- | ------ | -------- | ---------------------------- |
| `path`    | string | Yes      | The file path to write to    |
| `content` | string | Yes      | The content to write         |

### `create_file`

Create a new file with the specified content. This tool is used when creating files that do not yet exist.

**Parameters:**

| Parameter | Type   | Required | Description                    |
| --------- | ------ | -------- | ------------------------------ |
| `path`    | string | Yes      | The file path to create        |
| `content` | string | Yes      | The initial file content       |

### `edit_file`

Make a targeted replacement in an existing file. This is useful for making precise edits without rewriting the entire file.

**Parameters:**

| Parameter  | Type   | Required | Description                            |
| ---------- | ------ | -------- | -------------------------------------- |
| `path`     | string | Yes      | The file path to edit                  |
| `old_text` | string | Yes      | The text to find and replace           |
| `new_text` | string | Yes      | The replacement text                   |

### `list_directory`

List files and directories at the specified path.

**Parameters:**

| Parameter | Type   | Required | Description                      |
| --------- | ------ | -------- | -------------------------------- |
| `path`    | string | Yes      | The directory path to list        |

## Search Tools

### `search_files`

Search for files by name pattern (glob) or by content (regex).

**Parameters:**

| Parameter         | Type   | Required | Description                              |
| ----------------- | ------ | -------- | ---------------------------------------- |
| `pattern`         | string | Yes      | Glob pattern for filename matching       |
| `content_pattern` | string | No       | Regex pattern for content matching       |
| `path`            | string | No       | Base directory to search in              |

**Example usage:**

- Search for all Rust source files: `search_files(pattern="**/*.rs")`
- Search for files containing a specific function: `search_files(pattern="**/*.rs", content_pattern="fn main")`

## Shell Execution

### `bash`

Execute shell commands on the host system. This tool is only available when **Computer Use** mode is enabled.

**Parameters:**

| Parameter | Type   | Required | Description              |
| --------- | ------ | -------- | ------------------------ |
| `command` | string | Yes      | The shell command to run |

::: warning
The `bash` tool is powerful and can execute arbitrary commands. Only enable Computer Use mode when you trust the environment and the agent's instructions. Consider using Sandbox mode for additional safety.
:::

See the [Computer Use](/computer-use) page for more details on runtime modes and safety features.

## Web Search

### `web_search`

Search the web for information using a configurable search backend. Results are parsed from HTML using the `scraper` crate.

**Parameters:**

| Parameter | Type   | Required | Description                   |
| --------- | ------ | -------- | ----------------------------- |
| `query`   | string | Yes      | The search query              |

::: info
Web search must be enabled in your [Config Profile](/config-profiles) and requires a configured search provider.
:::

## Tool Availability

Tools are made available to the AI model based on the current configuration:

- **File tools** (`read_file`, `write_file`, `create_file`, `edit_file`, `list_directory`, `search_files`) are always available
- **`bash`** is available only when Computer Use is enabled
- **`web_search`** is available only when web search is enabled in the active config profile
- **Skills** can restrict available tools through the `allowed_tools` field

See [Skills](/skills) for information on how to control which tools are available to specific skills.
