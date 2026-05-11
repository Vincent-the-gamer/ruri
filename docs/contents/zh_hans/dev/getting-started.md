---
layout: doc
title: "开发者快速开始"
lastUpdated: true
---

# 开发者快速开始

本指南将带你完成 Ruri 开发环境搭建、从源码构建，以及了解项目架构。

## 前置条件

在开始之前，请确保已安装以下工具：

| 工具 | 版本 | 用途 |
| --- | --- | --- |
| [Rust](https://www.rust-lang.org/tools/install) | Stable（2024 edition） | 后端开发语言 |
| [Node.js](https://nodejs.org/) | 18+ | 前端工具链 |
| [pnpm](https://pnpm.io/installation) | 9+ | 包管理器（必需 —— Ruri 使用 pnpm 工作区） |

::: warning
Ruri 使用 **Rust 2024 edition**。请确保你的 Rust 工具链是最新的：

```bash
rustup update stable
```
:::

### 可选工具

- [Ollama](https://ollama.com) — 用于在本地运行 AI 模型进行测试，无需 API 密钥
- 模型提供商的 API 密钥（OpenAI、Anthropic、DeepSeek 等）用于端到端测试

::: info
如果你在中国大陆使用，可以考虑使用国内可访问的模型提供商（如 DeepSeek、智谱 AI 等），无需配置代理即可正常使用。
:::

## 从源码构建

### 1. 克隆仓库

```bash
git clone https://github.com/Vincent-the-gamer/ruri.git
cd ruri
```

::: tip
如果你在国内遭遇 GitHub 克隆速度慢的问题，可以使用镜像站点（如 `gitclone.com` 或 `ghproxy.com`），或配置 Git 代理：
```bash
git config --global http.proxy http://127.0.0.1:7890
```
:::

### 2. 构建后端

```bash
cargo build
```

此命令编译 Rust 后端。首次构建会花费较长时间，因为需要下载并编译所有依赖项。

::: info
如果在国内下载 Rust crate 依赖速度较慢，可以配置国内镜像源。编辑 `~/.cargo/config.toml`：

```toml
[source.crates-io]
replace-with = "ustc"

[source.ustc]
registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"
```

常见的国内镜像还有字节跳动的 `rsproxy.cn` 和清华大学的 `tuna`。
:::

### 3. 安装前端依赖

```bash
pnpm install
```

此命令安装根工作区和 `webui/` 包的依赖。

::: info
如果 `pnpm install` 速度较慢，可以配置国内 npm 镜像：

```bash
pnpm config set registry https://registry.npmmirror.com
```
:::

### 4. 构建前端

```bash
cd webui
pnpm build
```

Vite 构建输出会放置在 `webui/dist/` 目录。在执行完整的 `cargo build` 或 `cargo run` 时，Rust 构建脚本会将这些资源复制到 `src/web_dist/`，通过 `rust-embed` 嵌入到二进制文件中。

### 5. 运行服务器

```bash
cargo run
```

在浏览器中打开 `http://localhost:3000`，使用默认凭据（`ruri` / `ruri`）登录。

::: tip
如果你只修改了前端代码并希望更快地迭代，可以跳过 `cargo run`，改用 Vite 的开发服务器——参见下面的[开发模式](#开发模式)部分。
:::

## 开发模式

### 后端（Rust）

运行后端：

```bash
cargo run
```

Rust 后端没有内置的热重载功能。修改后端代码后，需要停止服务器（`Ctrl+C`）并重新运行 `cargo run`。为了在开发过程中加快重新编译，你可以使用：

```bash
# 安装 cargo-watch，实现文件变更时自动重新构建
cargo install cargo-watch
cargo watch -x run
```

### 前端（Vue 3 + Vite）

运行前端 Vite 开发服务器，支持热模块替换：

```bash
cd webui
pnpm dev
```

这会启动一个 Vite 开发服务器（默认地址：`http://localhost:5173`），它将 API 请求代理到 `http://localhost:3000` 的 Rust 后端。对 Vue 组件、样式或 TypeScript 文件的任何修改都会即时反映在浏览器中。

::: info
在前端开发期间，你需要**同时**运行 Rust 后端和 Vite 开发服务器。在一个终端中启动 `cargo run`，然后在另一个终端中运行 `cd webui && pnpm dev`。
:::

### CLI 参数

Ruri 支持多个命令行参数用于开发：

| 参数 | 简写 | 描述 |
| --- | --- | --- |
| `--port <PORT>` | `-p` | 设置服务器端口（默认：`3000`） |
| `--remote` | `-r` | 绑定到 `0.0.0.0`（可从网络访问） |
| `--acp` | `-a` | 以 ACP 模式启动（用于 IDE 集成的 stdio 传输） |
| `--acp-config <PATH>` | `-c` | 覆盖 ACP 配置文件路径 |

```bash
# 使用自定义端口运行
cargo run -- --port 8080

# 允许网络中的其他机器访问
cargo run -- --remote

# 以 ACP 模式运行（用于 IDE 集成测试）
cargo run -- --acp
```

## 项目结构

了解代码库的布局有助于你更有效地浏览和贡献代码。

```
ruri/
├── src/                    # Rust 后端
│   ├── main.rs             # 入口点，服务器设置，CLI 参数解析
│   ├── api/                # REST API 处理器、路由、模型、状态
│   │   ├── handlers.rs     # 路由处理器实现
│   │   ├── mod.rs          # 路由创建
│   │   ├── models.rs       # API 请求/响应类型
│   │   └── state.rs        # 共享应用状态（AppState）
│   ├── agent/              # AI 代理逻辑（聊天循环，工具调度）
│   ├── acp/                # Agent Client Protocol 服务器（IDE 集成）
│   ├── auth/               # 认证（会话，密码哈希）
│   ├── command/            # 命令系统（/help, /status 等）
│   ├── computer_use/       # Shell 命令执行 & 沙箱
│   ├── conversation/       # 对话数据库 & 历史
│   ├── db/                 # SQLite 数据库初始化 & 连接池
│   ├── knowledge/          # RAG 知识库（嵌入，搜索，文件解析）
│   ├── logging/            # 日志基础设施 & 日志管理器
│   ├── mcp/                # MCP 客户端（连接外部工具服务器）
│   ├── platform/           # 聊天平台适配器（钉钉、Discord、微信）
│   ├── provider/           # AI 模型提供商抽象
│   ├── tools/              # 内置工具（read_file, write_file, web_search 等）
│   ├── transport/          # MCP 传输层（stdio, SSE, WebSocket）
│   ├── types/              # 共享类型（ChatMessage, ContentPart 等）
│   └── web_dist/           # 嵌入的前端构建输出（自动生成）
├── webui/                  # Vue 3 前端
│   ├── src/
│   │   ├── api/            # API 客户端模块
│   │   ├── components/     # Vue 组件
│   │   ├── composables/    # Vue 组合式函数（可复用逻辑）
│   │   ├── locales/        # i18n 翻译文件
│   │   ├── router/         # Vue Router 配置
│   │   ├── stores/         # Pinia 状态存储
│   │   ├── types/          # TypeScript 类型定义
│   │   ├── views/          # 页面级 Vue 组件
│   │   ├── App.vue         # 根组件
│   │   ├── main.ts         # 前端入口点
│   │   └── style.css       # 全局样式
│   ├── public/             # 静态资源
│   ├── index.html          # HTML 入口点
│   ├── vite.config.ts      # Vite 配置
│   └── uno.config.ts       # UnoCSS 配置
├── docs/                   # 文档（VitePress）
│   └── contents/           # 文档页面 & VitePress 配置
├── Cargo.toml              # Rust 依赖
├── Cargo.lock              # Rust 依赖锁定文件
├── package.json            # 工作区根 package.json
├── pnpm-workspace.yaml     # pnpm 工作区配置
└── pnpm-lock.yaml          # 前端依赖锁定文件
```

### 核心架构概念

**后端（Rust + Axum）**

- 后端使用 [Axum](https://github.com/tokio-rs/axum) 作为 HTTP 框架，[Tokio](https://tokio.rs) 作为异步运行时
- `AppState` 是核心共享状态，通过 `Arc` 持有并传递给所有处理器
- 使用 SQLite（通过 [SQLx](https://github.com/launchbadge/sqlx)）进行持久化存储——对话、MCP 配置、知识库
- 前端通过 `rust-embed` 在编译时嵌入到二进制文件中，因此最终的二进制文件是自包含的
- 基于 Session 的认证，使用 [Argon2](https://en.wikipedia.org/wiki/Argon2) 密码哈希

**前端（Vue 3 + Vite + UnoCSS）**

- [Vue 3](https://vuejs.org/) 组合式 API
- [Vite](https://vitejs.dev/) 构建工具和开发服务器
- [UnoCSS](https://unocss.dev/) 实用优先的 CSS 框架
- [Pinia](https://pinia.vuejs.org/) 状态管理
- [Vue Router](https://router.vuejs.dev/) 路由
- [vue-i18n](https://kazupon.github.io/vue-i18n/) 国际化
- [Axios](https://axios-http.com/) HTTP API 调用

## 环境变量

Ruri 使用标准的 `RUST_LOG` 环境变量来控制日志详细程度（通过 `tracing-subscriber` 实现）：

```bash
# 仅显示警告和错误
RUST_LOG=warn cargo run

# 显示 info 级别日志（默认）
RUST_LOG=info cargo run

# 显示整个应用的 debug 日志
RUST_LOG=debug cargo run

# 仅显示特定模块的 debug 日志
RUST_LOG=ruri::agent=debug cargo run

# 显示所有内容的 trace 级别日志
RUST_LOG=trace cargo run
```

::: warning
`RUST_LOG=trace` 会产生大量输出。建议使用模块特定的过滤器，如 `RUST_LOG=ruri::api=debug`，来聚焦你需要关注的部分。
:::

## 常见开发任务

### 添加新的 API 端点

1. 在 `src/api/mod.rs` 中定义路由
2. 在 `src/api/handlers.rs` 中添加处理函数
3. 如有需要，在 `src/api/models.rs` 中添加请求/响应类型
4. 使用 `cargo run` 重新构建并测试

### 添加新的前端页面

1. 在 `webui/src/views/` 中创建新的 `.vue` 文件
2. 在 `webui/src/router/` 中添加路由
3. 在 `webui/src/api/` 中添加所需的 API 客户端函数
4. 如果页面管理状态，在 `webui/src/stores/` 中添加 Pinia store
5. 运行 `cd webui && pnpm dev` 使用热重载测试

### 运行测试

```bash
# 运行所有 Rust 测试
cargo test

# 运行特定模块的测试
cargo test -p ruri --lib agent
```

::: info
当前项目结构中尚未配置前端测试。欢迎贡献添加 Vitest 或 Cypress 测试框架！
:::

## 下一步

- [API 使用指南](/zh_hans/dev/api-usage) — 学习如何以编程方式使用 Ruri 的 REST API
- [集成指南](/zh_hans/dev/integration) — 将 Ruri 集成到你自己的应用中
- [API 参考](/api) — 完整的端点文档
