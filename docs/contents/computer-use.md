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
- 📦 The AI can use 8 sandbox-specific tools (see below)
- 🌐 Sandbox endpoint is configurable, supporting remote sandbox instances
- 🛡️ Host filesystem and system commands are not directly accessible
- 🔄 Automatic retry with exponential backoff for transient server errors
- 🔗 Shell commands maintain a session ID for continuity across invocations

### AIO Sandbox Tools

When AIO Sandbox mode is active, the AI has access to the following tools for interacting with the sandbox container:

| Tool               | ID               | Description                                                                               |
| ------------------ | ---------------- | ----------------------------------------------------------------------------------------- |
| **Shell**          | `shell`          | Execute shell commands in the sandbox container, with an optional timeout                 |
| **Read File**      | `read_file`      | Read file contents from the sandbox                                                       |
| **Write File**     | `write_file`     | Write content to a file in the sandbox                                                    |
| **Create File**    | `create_file`    | Create a new file (fails if the file already exists; auto-creates parent directories)     |
| **Edit File**      | `edit_file`      | Find and replace exact text in a file — surgical edits using `old_text`/`new_text` pairs  |
| **List Directory** | `list_directory` | List directory contents with file info (size, type, permissions), defaults to `/home/gem` |
| **Find Files**     | `find_files`     | Find files by name pattern using glob syntax (e.g., `**/*.py`, `src/**/*.rs`)             |
| **Search in File** | `search_in_file` | Search within a file using a regex pattern, returns matches with line numbers             |

#### Tool Details

- **Shell** — Runs commands in a persistent shell session. The sandbox maintains a `session_id` so environment variables, `cd` changes, and other side effects persist across multiple shell calls within the same conversation. You can optionally specify a timeout (in seconds) for long-running commands.

- **Read File** — Reads the full content of a file at a given path inside the sandbox.

- **Write File** — Writes content to a file, overwriting any existing content. The parent directory must already exist.

- **Create File** — Creates a new file only if it does not already exist. Unlike Write File, it automatically creates any missing parent directories, making it safe for building out new project structures.

- **Edit File** — Performs precise, surgical edits by replacing an exact `old_text` snippet with `new_text`. This avoids the risk of overwriting unrelated content. Returns the number of replacements made.

- **List Directory** — Lists the contents of a directory with rich metadata: file name, whether it's a file or directory, size, modification time, and permissions. Results include a summary of total, directory, and file counts.

- **Find Files** — Searches for files matching a glob pattern. Supports standard glob syntax like `**/*.py` (all Python files recursively) or `src/**/*.rs` (Rust files under `src/`). Returns a list of matching file paths.

- **Search in File** — Searches within a specific file using a regular expression pattern. Returns the matched text along with the corresponding line numbers, making it easy to locate specific code or content.

### Retry & Error Handling

The AIO Sandbox client includes built-in resilience to handle transient server issues:

- **Automatic retry** — When the sandbox API returns a transient server error (HTTP 502 Bad Gateway, 503 Service Unavailable, or 504 Gateway Timeout), the client automatically retries the request.
- **Exponential backoff** — Retries use exponential backoff delays: 1s, 2s, 4s between attempts.
- **Up to 3 retries** — The client will retry up to 3 times (4 total attempts) before giving up.
- **Descriptive error messages** — Transient errors include helpful context, such as reminding you to check whether the sandbox container is running and whether the configured endpoint is reachable.
- **No retry on client errors** — HTTP 4xx errors and parse errors are returned immediately without retry, since they indicate a problem with the request itself rather than a transient server issue.

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
