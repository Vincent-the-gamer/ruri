---
layout: doc
title: "MCP 客户端"
lastUpdated: true
---

# MCP 客户端

**MCP（模型上下文协议）** 让 Ruri 能连接外部工具服务器，极大地扩展 AI 的能力。简单来说，MCP 就像是给 AI 安装"插件" — 连接不同的 MCP 服务器，AI 就能获得新的技能，完成内置能力之外的任务，比如访问数据库、管理远程系统上的文件，或使用专业服务。

## 什么是 MCP 服务器？

MCP 服务器是为 Ruri 提供额外工具和数据的外部程序。当你连接到一个 MCP 服务器后，它的工具就会像内置工具一样对 AI 可用。

例如，连接到**文件系统 MCP 服务器**后，AI 可以安全地访问指定目录中的文件。连接到**网页搜索 MCP 服务器**后，AI 将获得增强的搜索能力。

### 为什么要用 MCP？

- **扩展 AI 能力** — 添加 Ruri 内置之外的工具
- **访问外部数据** — 连接数据库、API 和其他数据源
- **专业工具** — 使用特定领域的工具，如数据库查询、API 测试等
- **安全的文件访问** — 通过文件系统服务器让 AI 在受控范围内访问指定目录

## MCP 工具如何工作

当 Ruri 连接到 MCP 服务器后，流程如下：

1. 服务器告诉 Ruri 它能提供哪些工具
2. 这些工具会出现在 Ruri 内置工具旁边
3. AI 根据你的对话内容决定何时使用它们
4. 工具调用被发送到 MCP 服务器执行
5. 结果返回后，AI 继续对话

你会在聊天中看到 MCP 工具调用，就像内置工具一样 — AI 在做什么完全透明可见。

![MCP 配置页面](/ruri-pics/zh_hans/mcp-config-cn.png)

## 添加 MCP 服务器

### 步骤 1：打开 MCP 页面

在侧边栏中点击 **MCP**。

### 步骤 2：添加服务器

点击**添加服务器**并填写以下信息：

- **名称** — 此服务器的唯一名称
- **传输类型** — Ruri 连接服务器的方式：
  - **Stdio** — 在你的机器上运行本地程序
  - **SSE / WebSocket / HTTP** — 通过网络连接到远程服务器
- **配置信息** — 具体内容取决于传输类型（见下文）

### 步骤 3：启用并使用

打开服务器开关。一旦连接成功，该服务器的工具会立即可供 AI 使用！

::: info
连接状态会显示在 MCP 页面上。如果服务器连接失败，请检查配置并确保所需的程序已安装。
:::

## 常用 MCP 服务器示例

### 文件系统服务器

让 AI 受控访问你计算机上的指定目录：

- **传输类型：** Stdio
- **命令：** `npx`
- **参数：** `@modelcontextprotocol/server-filesystem /path/to/your/project`

这样 AI 就可以安全地仅在指定目录内读写文件。

### SQLite 数据库服务器

让 AI 查询和操作 SQLite 数据库：

- **传输类型：** Stdio
- **命令：** `npx`
- **参数：** `@modelcontextprotocol/server-sqlite /path/to/database.db`

### GitHub 服务器

让 AI 操作 GitHub 仓库（查看 Issue、PR 等）：

- **传输类型：** Stdio
- **命令：** `npx`
- **参数：** `@modelcontextprotocol/server-github`
- **环境变量：** 需要设置 `GITHUB_PERSONAL_ACCESS_TOKEN`

### Puppeteer 浏览器控制服务器

让 AI 控制浏览器进行网页操作：

- **传输类型：** Stdio
- **命令：** `npx`
- **参数：** `@modelcontextprotocol/server-puppeteer`

### Brave 搜索服务器

使用 Brave 搜索引擎替代默认搜索：

- **传输类型：** Stdio
- **命令：** `npx`
- **参数：** `@modelcontextprotocol/server-brave-search`
- **环境变量：** 需要设置 `BRAVE_API_KEY`

::: tip
浏览 [MCP 服务器仓库](https://github.com/modelcontextprotocol/servers) 查找越来越多可连接的服务器列表。
:::

## 管理 MCP 服务器

### 通过 Web UI

1. 在侧边栏中点击 **MCP**
2. 使用添加按钮**添加**新服务器
3. 根据需要**开关**服务器
4. **监控**每个服务器的连接状态
5. **编辑**服务器配置
6. **移除**不再需要的服务器

### 连接类型

添加服务器时，你需要选择 Ruri 的连接方式：

- **Stdio** — 适用于作为本地程序运行的服务器。你需要提供启动命令、参数以及环境变量。
- **SSE / WebSocket / HTTP** — 适用于远程运行的服务器。你需要提供 URL 以及可选的认证头信息。

::: warning
只连接你信任的 MCP 服务器。它们提供的工具可以代表 AI 代理访问文件、执行命令或发起网络请求。使用[技能系统](/zh_hans/skills)中的 `allowed_tools` 来限制 MCP 工具在特定上下文中的使用范围。

不受信任的 MCP 服务器可能提供危险的工具（如执行命令、访问文件），请谨慎添加。本地服务器以与 Ruri 相同的权限运行，请务必确认服务器来源可靠。远程服务器请尽量使用 HTTPS 连接。
:::
