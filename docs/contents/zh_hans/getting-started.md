---
layout: doc
title: "快速开始"
lastUpdated: true
---

# 快速开始

欢迎使用 **Ruri 琉璃** — 一个使用 Rust 和 Vue 构建的可自定义 AI 智能体。本指南将帮助您快速上手。

## 概述

Ruri 是一个全栈 AI 智能体应用，结合了高性能的 Rust 后端和现代的 Vue 3 前端。它提供了丰富的功能来与 AI 模型交互，包括工具调用、技能管理、知识库集成和多平台聊天支持。

**技术栈：**

| 层级   | 技术                          |
| ------ | ----------------------------- |
| 后端   | Rust (Axum, SQLite, Tokio)    |
| 前端   | Vue 3 + Vite + UnoCSS         |

## 前置条件

- [Rust](https://www.rust-lang.org/tools/install)（最新稳定版）
- [Node.js](https://nodejs.org/)（用于前端构建，如从源码构建）
- AI 模型提供商 API 密钥（Anthropic、OpenAI 或兼容接口）

## 安装

### 从源码构建

克隆仓库并构建项目：

```bash
git clone https://github.com/Vincent-the-gamer/ruri.git
cd ruri
cargo build --release
```

编译后的二进制文件位于 `target/release/ruri`。

### 开发模式运行

```bash
cargo run
```

这将启动后端服务器以及 Web UI。

## 首次启动

### 默认凭据

首次启动时，Ruri 使用以下默认凭据：

| 字段   | 默认值   |
| ------ | -------- |
| 用户名 | `ruri`   |
| 密码   | `ruri`   |

::: warning
首次登录时，系统会提示您修改密码以提高安全性。
:::

### 访问 Web UI

服务器启动后，在浏览器中打开：

```
http://localhost:3000
```

使用默认凭据登录，然后配置模型提供商即可开始对话。

## 配置模型提供商

在使用 Ruri 之前，您需要至少配置一个模型提供商。在 Web UI 中导航到 **提供商** 页面，添加您的提供商详细信息：

1. 选择提供商类型（Anthropic 兼容、OpenAI 兼容或自定义）
2. 输入 API 端点 URL
3. 提供 API 密钥
4. 设置默认模型名称
5. 激活该提供商

详细配置说明请参阅[模型提供商](/zh_hans/providers)页面。

## 下一步

现在您已经运行起来了，可以探索核心功能：

- [内置工具](/zh_hans/tools) — 文件操作、Shell 命令和网页搜索
- [技能系统](/zh_hans/skills) — 使用 Markdown 创建自定义 AI 行为
- [人格系统](/zh_hans/personas) — 自定义 AI 助手的性格
- [MCP 客户端](/zh_hans/mcp) — 连接外部工具服务器
- [知识库](/zh_hans/knowledge-base) — 添加 RAG 驱动的文档搜索
- [聊天平台](/zh_hans/platforms) — 连接钉钉、Discord 或微信
- [ACP 服务端](/zh_hans/acp) — 在 IDE 中将 Ruri 用作智能体服务器
