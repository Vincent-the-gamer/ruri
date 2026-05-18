---
layout: doc
title: "快速开始"
lastUpdated: true
---

# 快速开始

欢迎使用 **Ruri 琉璃** — 你的个人 AI 助手，运行在浏览器中。强大、私密、高度可定制。

## Ruri 能做什么？

Ruri 不仅仅是一个聊天机器人。以下是你可以用它做的事情：

- **与 AI 模型对话** — 连接 OpenAI、Anthropic、DeepSeek，或通过 Ollama 运行本地模型
- **读写文件** — AI 可以浏览你的项目文件、编辑代码、创建新文件
- **联网搜索** — 让 AI 在线查找信息，给你更准确的回答
- **执行命令** — 启用 Computer Use 后，AI 可以执行 Shell 命令、运行构建任务等
- **构建知识库** — 上传你的文档，让 AI 从中检索答案
- **创建自定义技能** — 用简单的 Markdown 文件教 AI 新技能
- **接入聊天平台** — 通过钉钉、Discord 或微信使用 Ruri
- **在 IDE 中使用** — 将 Ruri 接入 Zed 或 JetBrains，作为 AI 编程助手

听起来不错？让我们开始配置吧！

## 准备工作

安装 Ruri 之前，请确保你拥有：

- AI 模型提供商的 **API 密钥**（如 OpenAI、Anthropic 或 DeepSeek） — 或者安装 [Ollama](https://ollama.com) 以免费使用本地模型

## 安装

1. **下载** [GitHub Releases](https://github.com/Vincent-the-gamer/ruri/releases) 中的最新版本
2. **解压** 到你选择的文件夹
3. **将 Ruri 添加到 PATH** 中，以便在任意位置运行（或直接进入文件夹运行）
4. 在终端中运行 `ruri` 启动服务

```bash
# 查看帮助
ruri -h
# 基本启动
ruri
# 默认端口为 3000
ruri --port 8080
# 远程访问，将服务暴露到互联网
ruri --remote
```

就这样！服务端和 Web UI 会自动一同启动。

## 首次启动

### 第 1 步：登录

打开浏览器，访问：

```
http://localhost:3000
```

使用默认凭据登录：

| 字段   | 默认值 |
| ------ | ------ |
| 用户名 | `ruri` |
| 密码   | `ruri` |

::: warning
首次登录时系统会提示你修改密码。请设置一个强密码以确保实例安全！
:::

### 第 2 步：添加模型提供商

在开始聊天之前，你需要至少配置一个 AI 模型提供商：

1. 在侧边栏进入 **提供商** 页面
2. 点击 **添加提供商**
3. 选择提供商类型（如 OpenAI Compatible）
4. 输入你的 API URL、API 密钥和模型名称
5. 点击 **保存**，然后 **激活** 该提供商

请参阅[模型提供商](/zh_hans/providers)页面，查看主流提供商的详细配置指南。

### 第 3 步：开始聊天！

前往聊天页面，开始对话。随便问 Ruri 点什么 — 它已经准备好了！

## 下一步

现在你已经启动并运行了，来探索 Ruri 的更多功能吧：

- [内置工具](/zh_hans/tools) — 了解 AI 能为你做什么
- [技能系统](/zh_hans/skills) — 创建自定义 AI 行为
- [人格系统](/zh_hans/personas) — 自定义 AI 的个性
- [MCP 客户端](/zh_hans/mcp) — 连接外部工具服务器
- [知识库](/zh_hans/knowledge-base) — 为 AI 添加文档搜索能力
- [聊天平台](/zh_hans/platforms) — 接入钉钉、Discord 或微信
- [Computer Use](/zh_hans/computer-use) — 让 AI 在你的系统上执行命令
- [ACP 服务端](/zh_hans/acp) — 将 Ruri 用作 IDE 中的 AI 助手
- [配置档案](/zh_hans/config-profiles) — 为不同任务设置不同配置

## 常见问题

### 服务无法启动，我该检查什么？

- 确保端口 `3000` 没有被其他应用占用
- 检查你对 Ruri 安装目录是否有写入权限
- 在 macOS / Linux 上，确保二进制文件有执行权限（`chmod +x ruri`）

### 无法登录 Web UI

- 再次确认服务正在运行，并且你访问的是 `http://localhost:3000`
- 尝试使用默认凭据（`ruri` / `ruri`）
- 清除浏览器缓存和 Cookie，然后重试

### AI 不回复我的消息

- 检查你是否已添加并激活了模型提供商
- 验证 API 密钥是否正确，以及是否有可用额度
- 确认模型名称拼写正确（例如 `gpt-4o`，而不是 `gpt4o`）

### 如何免费使用 Ruri？

安装 [Ollama](https://ollama.com)，下载一个模型（例如 `ollama pull llama3`），然后添加一个 **OpenAI Compatible** 类型的提供商，地址指向 `http://localhost:11434/v1`，模型填写 `llama3`。无需 API 密钥！详见 [Ollama 配置指南](/zh_hans/providers)。

### 可以修改默认端口吗？

可以 — 你可以在启动 Ruri 时通过命令行参数修改端口。运行 `ruri --help` 查看所有可用选项。
