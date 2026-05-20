<div align="center">
    <img src=".github/ruri-avatar.png" style="width: 120px;" alt="Ruri Logo"/>
    <h1>Ruri 琉璃</h1>
    <p>
        <b>一个可自定义的 AI 智能体，使用 Rust + Vue 编写。</b>
    </p>
    <p>
        <a href="https://github.com/Vincent-the-gamer/ruri/releases"><img src="https://img.shields.io/github/v/release/Vincent-the-gamer/ruri?style=flat-square" alt="Release"/></a>
        <a href="./COPYING"><img src="https://img.shields.io/badge/license-GPLv3-blue.svg?style=flat-square" alt="License"/></a>
        <a href="https://ruri.vince-g.xyz/"><img src="https://img.shields.io/badge/docs-在线文档-green.svg?style=flat-square" alt="Docs"/></a>
    </p>
    <p><a href="README.md">English</a> | <a href="README.zh-CN.md">中文</a></p>
</div>

> [!IMPORTANT]
> 本项目已进入公测阶段。

---

## 📖 文档

完整文档请访问 **[ruri.vince-g.xyz](https://ruri.vince-g.xyz/)**。

| 章节                                                          | 说明                                   |
| ------------------------------------------------------------- | -------------------------------------- |
| [快速开始](https://ruri.vince-g.xyz/zh_hans/getting-started)  | 安装与首次启动指南                     |
| [模型提供商](https://ruri.vince-g.xyz/zh_hans/providers)      | 配置 OpenAI、Anthropic、Gemini、Ollama |
| [内置工具](https://ruri.vince-g.xyz/zh_hans/tools)            | 文件操作、Shell、网页搜索、图像分析    |
| [技能系统](https://ruri.vince-g.xyz/zh_hans/skills)           | 创建自定义 AI 行为                     |
| [人格系统](https://ruri.vince-g.xyz/zh_hans/personas)         | 自定义 AI 个性                         |
| [MCP 客户端](https://ruri.vince-g.xyz/zh_hans/mcp)            | 连接外部工具服务器                     |
| [知识库](https://ruri.vince-g.xyz/zh_hans/knowledge-base)     | RAG 文档搜索                           |
| [聊天平台](https://ruri.vince-g.xyz/zh_hans/platforms)        | 钉钉、Discord、微信、OneBot12          |
| [Computer Use](https://ruri.vince-g.xyz/zh_hans/computer-use) | 沙盒化命令执行                         |
| [API 参考](https://ruri.vince-g.xyz/zh_hans/api)              | 完整 REST API 文档                     |
| [开发者指南](https://ruri.vince-g.xyz/zh_hans/dev/)           | 构建、集成与贡献                       |

---

## ✨ 功能特性

### 🤖 多提供商 AI 聊天

同时连接多个 AI 模型提供商 — **OpenAI**、**Anthropic (Claude)**、**Google Gemini**、**DeepSeek**、**Ollama**（本地）以及任何兼容 OpenAI 格式的 API。通过[配置档案](https://ruri.vince-g.xyz/zh_hans/config-profiles)一键切换模型。

### 🔧 内置工具

赋予 AI 真正的操作能力，超越纯文本对话：

- **文件操作** — 读取、写入、创建、编辑和删除文件
- **Shell 执行** — 在隔离的 Docker 沙盒中或直接在系统上运行命令
- **网页搜索** — DuckDuckGo、Tavily、Brave、百度等
- **目录浏览** — 使用正则和 glob 模式列出和搜索文件
- **图像分析** — 支持多模态视觉模型

### 🎯 技能系统

用 Markdown 写成的技能教 AI 新能力。每个技能通过 YAML frontmatter 定义自定义行为，支持工具限制、模型覆盖、Shell 钩子和自动触发。可将技能打包为 ZIP 分享。

### 🧠 人格系统

自定义 AI 的个性 — 在代码专家、创意作家、学习导师或任何你创建的人格之间切换。定义系统提示词，一键更换人格，绑定到配置档案。

### 🔌 MCP 客户端

通过 Stdio、SSE、WebSocket 或 HTTP 连接外部 **模型上下文协议（MCP）** 服务器。用文件系统服务器、数据库工具、浏览器自动化等扩展 AI 能力。

### 📚 知识库（RAG）

上传 PDF、Excel 电子表格、Word 文档和文本文件。AI 通过嵌入模型搜索你的文档，可选的排序模型提升搜索精度。

### 💬 聊天平台

在你最常用的聊天应用中使用 Ruri：

- **钉钉**
- **Discord**
- **微信**（通过 Wechat ClawBot）
- **OneBot12**（QQ、飞书等）

支持热重载平台配置、单独重启适配器，并自动持久化凭证。

### 🛡️ AIO 沙盒

通过 [AIO Sandbox](https://github.com/agent-infra/sandbox) 在隔离的 Docker 容器中安全执行 AI 命令。对瞬时错误自动进行指数退避重试。

### ⌨️ 指令系统

便捷的斜杠指令，用于会话控制：`/help`、`/new`、`/reset`、`/sid`、`/whoami`、`/set`、`/stop` 等。

### 🧩 ACP 服务端

通过 Agent Client Protocol（基于 stdio 的 JSON-RPC）在 **Zed** 和 **JetBrains IDE** 中将 Ruri 作为 AI 编程助手使用。

### ⚙️ 配置档案

创建基于场景的配置档案（编程、写作、研究、闲聊），将提供商、人格、技能、平台、知识库和代理设置打包 — 一键切换所有配置。

### 🌐 代理支持

支持 HTTP、HTTPS、SOCKS4 和 SOCKS5 代理，具备 Clash 风格的规则路由。可按配置档案设置代理，支持本地地址绕过。

---

## 🚀 快速开始

### 准备工作

- 拥有一个 AI 提供商的 API 密钥（OpenAI、Anthropic、DeepSeek 等）**或**
- 安装 [Ollama](https://ollama.com) 免费使用本地模型

### 安装

1. 从 [GitHub Releases](https://github.com/Vincent-the-gamer/ruri/releases) 下载最新版本
2. 将二进制文件添加到系统环境变量(`PATH`)
3. 启动服务：

```bash
# 查看可用选项
ruri -h

# 使用默认设置启动（端口 3000）
ruri

# 自定义端口
ruri --port 8080

# 暴露到局域网
ruri --remote
```

4. 在浏览器中打开 `http://localhost:3000`
5. 使用默认凭据登录（`ruri` / `ruri`）— 系统会提示修改密码
6. 在 **提供商** 页面添加模型提供商并激活
7. 开始聊天！

> 📖 详细的配置指南请参阅[快速开始](https://ruri.vince-g.xyz/zh_hans/getting-started)文档。

---

## 🏗️ 技术栈

| 层级 | 技术                                                  |
| ---- | ----------------------------------------------------- |
| 后端 | Rust + Axum + Tokio + SQLx (SQLite)                   |
| 前端 | Vue 3 + TypeScript + Vite + Pinia + UnoCSS + vue-i18n |
| 文档 | VitePress + Teek Theme + Twoslash                     |

---

## 🔧 开发指南

### 准备工作

- [Rust](https://www.rust-lang.org/tools/install)（stable，2024 edition）
- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/installation) 9+

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/Vincent-the-gamer/ruri.git
cd ruri

# 构建前端（嵌入静态资源必需）
pnpm -C webui run build

# 构建并运行后端
cargo run
```

### 开发模式

前端开发使用热重载：

```bash
# 终端 1：启动 Rust 后端
cargo run

# 终端 2：启动 Vite 开发服务器
cd webui && pnpm dev
```

Vite 开发服务器将 API 请求代理到后端，并为 Vue 组件提供即时热模块替换。

### 常用命令

```bash
# 生产构建
cargo build --release

# 开启详细日志运行
RUST_LOG=debug cargo run

# 运行全部测试
cargo test

# 本地构建并启动文档站点
cd docs && pnpm dev
```

### IDE 集成（ACP 模式）

```bash
cargo run -- --acp
```

在 Zed/Jetbrains/其它IDE 的配置文件中添加：（根据具体IDE配置，可能有所区别）

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

---

## 📁 项目结构

```
ruri/
├── src/                    # Rust 后端
│   ├── main.rs             # 入口、CLI 参数、服务器配置
│   ├── api/                # REST API 处理器、路由、模型
│   ├── agent/              # AI agent 聊天循环与工具调度
│   ├── acp/                # Agent Client Protocol（IDE 集成）
│   ├── auth/               # 会话认证 + Argon2 密码哈希
│   ├── command/            # 斜杠指令系统
│   ├── computer_use/       # Shell 执行与沙盒
│   ├── conversation/       # 对话持久化
│   ├── db/                 # SQLite 初始化与连接池
│   ├── knowledge/          # RAG 知识库
│   ├── logging/            # 日志基础设施
│   ├── mcp/                # MCP 客户端（外部工具服务器）
│   ├── platform/           # 聊天平台适配器
│   ├── provider/           # AI 模型提供商抽象
│   ├── transport/          # 传输层（stdio、SSE、WS、HTTP）
│   ├── types/              # 共享类型定义
│   └── web_dist/           # 嵌入式前端构建产物
├── webui/                  # Vue 3 前端
│   └── src/
│       ├── api/            # API 客户端模块
│       ├── components/     # 可复用 Vue 组件
│       ├── composables/    # Vue 组合式函数
│       ├── locales/        # i18n 翻译文件
│       ├── router/         # Vue Router 配置
│       ├── stores/         # Pinia 状态管理
│       ├── views/          # 页面级组件
│       └── App.vue         # 根组件
├── docs/                   # VitePress 文档
│   └── contents/           # 中英文文档
├── Cargo.toml              # Rust 依赖
└── pnpm-workspace.yaml     # pnpm 工作空间配置
```

---

## 🤝 参与贡献

欢迎贡献！你可以通过以下方式参与：

1. **报告 Bug** — 在 [GitHub Issues](https://github.com/Vincent-the-gamer/ruri/issues) 提交问题
2. **建议功能** — 分享你对新功能的想法
3. **提交 PR** — 修复 Bug、添加功能或改进文档
4. **编写技能** — 与社区分享有用的 AI 技能
5. **完善文档** — 协助翻译或修复文档错误

提交代码前请先阅读[开发者指南](https://ruri.vince-g.xyz/zh_hans/dev/getting-started)。

---

## 📄 开源许可证

[GPLv3 License](./COPYING)

版权所有 (C) 2026-现在 Vincent-the-gamer <https://github.com/Vincent-the-gamer>
