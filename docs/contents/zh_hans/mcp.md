---
layout: doc
title: "MCP 客户端"
lastUpdated: true
---

# MCP 客户端

**MCP（模型上下文协议）** 让 Ruri 能连接外部工具服务器，极大地扩展 AI 的能力。简单来说，MCP 就像是给 AI 安装"插件" — 连接不同的 MCP 服务器，AI 就能获得新的技能。

## 为什么要用 MCP？

Ruri 内置了文件操作、搜索、网页搜索等工具，但 MCP 能让 AI 做到更多：

- 🗄️ **访问数据库** — 查询和操作数据库
- 📂 **访问更多文件系统** — 扩展 AI 可访问的目录
- 🔗 **对接第三方服务** — 如 GitHub、Notion、Slack 等
- 🧮 **专业计算工具** — 数学运算、数据分析等
- 🔧 **自定义工具** — 你自己开发的专有工具

## 添加 MCP 服务器

### 通过 Web UI

1. 在侧边栏点击 **MCP**
2. 点击 **添加服务器**
3. 填写服务器名称和连接信息（具体字段取决于服务器类型）
4. 保存后，服务器会自动连接

### 连接信息说明

添加 MCP 服务器时，你需要提供连接方式。主要有两种类型：

**本地服务器**（运行在你电脑上的服务）：

- 需要提供启动命令和相关参数
- 例如使用 `npx` 或 `node` 启动一个 MCP 服务

**远程服务器**（运行在网络上的服务）：

- 需要提供服务器 URL
- 可能需要设置认证头（如 API Key）

添加后，你可以在 MCP 页面查看每个服务器的连接状态。

## 常用 MCP 服务器

### 📂 文件系统访问

让 AI 访问指定目录的文件：

- 启动命令：`npx @modelcontextprotocol/server-filesystem /path/to/allowed/directory`
- 类型：本地服务器

### 🗄️ SQLite 数据库

让 AI 查询和操作 SQLite 数据库：

- 启动命令：`npx @modelcontextprotocol/server-sqlite /path/to/database.db`
- 类型：本地服务器

### 🔗 GitHub

让 AI 操作 GitHub 仓库（查看 Issue、PR 等）：

- 启动命令：`npx @modelcontextprotocol/server-github`
- 需要设置环境变量 `GITHUB_PERSONAL_ACCESS_TOKEN`
- 类型：本地服务器

### 🌐 Puppeteer 浏览器控制

让 AI 控制浏览器进行网页操作：

- 启动命令：`npx @modelcontextprotocol/server-puppeteer`
- 类型：本地服务器

### 🔍 Brave 搜索

使用 Brave 搜索引擎替代默认搜索：

- 启动命令：`npx @modelcontextprotocol/server-brave-search`
- 需要设置环境变量 `BRAVE_API_KEY`
- 类型：本地服务器

::: tip
你可以在 [MCP 服务器仓库](https://github.com/modelcontextprotocol/servers) 中找到更多可用的 MCP 服务器。
:::

## MCP 工具如何工作

当你连接一个 MCP 服务器后：

1. 服务器会告诉 Ruri 它能提供哪些工具
2. 这些工具会自动出现在 AI 可用的工具列表中
3. AI 根据你的需求自动决定是否调用这些工具
4. 工具调用在服务器上执行，结果返回给 AI
5. AI 根据结果给你回答

对你来说，MCP 工具和内置工具的体验完全一样 — AI 会在需要时自动使用它们。

## 安全提示

- ⚠️ 只连接你信任的 MCP 服务器
- 🔒 本地服务器以 Ruri 相同的权限运行，请确认服务器来源可靠
- 🌐 远程服务器请尽量使用 HTTPS 连接
- 🛡️ 可以在技能中通过 `allowed_tools` 限制 MCP 工具的使用范围

::: warning
不受信任的 MCP 服务器可能提供危险的工具（如执行命令、访问文件），请谨慎添加。
:::
