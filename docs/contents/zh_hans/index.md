---
layout: home
title: "Ruri 琉璃"

hero:
  name: "Ruri 琉璃"
  text: "可自定义的 AI 智能体"
  tagline: "一个可自定义的 AI 智能体，使用 Rust + Vue 编写。"
  image:
    src: "/logo/logo.png"
    alt: logo
  actions:
    - theme: brand
      text: 快速开始
      link: /zh_hans/getting-started
    - theme: alt
      text: API 参考
      link: /zh_hans/api

features:
  - title: 模型提供商
    details: 支持 Anthropic 兼容、OpenAI 兼容和自定义模型提供商。
    icon:
      src: "/imgs/model-providers.svg"
  - title: 工具调用
    details: 内置文件操作、Shell 执行和网页搜索工具。
    icon:
      src: "/imgs/tool-call.svg"
  - title: 技能系统
    details: 基于 Markdown 和 YAML frontmatter 的技能系统，支持自定义行为。
    icon:
      src: "/imgs/skills.svg"
  - title: MCP 客户端
    details: 通过 Stdio、SSE、WebSocket 或 HTTP 连接外部 MCP 服务器。
    icon:
      src: "/imgs/mcp-client.svg"
  - title: 知识库
    details: 基于 RAG 的知识库，支持嵌入模型和重排序模型。
    icon:
      src: "/imgs/knowledge-base.svg"
  - title: 聊天平台
    details: 钉钉、Discord、微信集成，支持热重载配置。
    icon:
      src: "/imgs/chat-platforms.svg"
---
