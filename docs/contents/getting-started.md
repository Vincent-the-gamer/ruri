---
layout: doc
title: "Getting Started"
lastUpdated: true
---

# Getting Started

Welcome to **Ruri 琉璃** — a customizable AI Agent built with Rust and Vue. This guide will help you get up and running quickly.

## Overview

Ruri is a full-stack AI agent application that combines a high-performance Rust backend with a modern Vue 3 frontend. It provides a rich set of features for interacting with AI models, including tool calling, skill management, knowledge base integration, and multi-platform chat support.

**Tech Stack:**

| Layer    | Technology                 |
| -------- | -------------------------- |
| Backend  | Rust (Axum, SQLite, Tokio) |
| Frontend | Vue 3 + Vite + UnoCSS      |

## Prerequisites

An AI model provider(chat/embedding), you'll need API key (Anthropic, OpenAI, or compatible)

## Installation

Download the latest release from [GitHub Releases](https://github.com/Vincent-the-gamer/ruri/releases)

and add it to your PATH.

This starts the backend server along with the Web UI.

## First Launch

### Default Credentials

On first launch, Ruri uses the following default credentials:

| Field    | Default Value |
| -------- | ------------- |
| Username | `ruri`        |
| Password | `ruri`        |

::: warning
You will be prompted to change your password on first login for security purposes.
:::

### Access the Web UI

Once the server is running, open your browser and navigate to:

```
http://localhost:3000
```

Log in with the default credentials, then configure your model provider to start chatting.

## Configure a Model Provider

Before you can start using Ruri, you need to configure at least one model provider. Navigate to the **Providers** section in the Web UI and add your provider details:

1. Choose a provider type (Anthropic Compatible, OpenAI Compatible, or Custom)
2. Enter your API endpoint URL
3. Provide your API key
4. Set the default model name
5. Activate the provider

See the [Model Providers](/providers) page for detailed configuration instructions.

## Next Steps

Now that you're up and running, explore the core features:

- [Built-in Tools](/tools) — File operations, shell commands, and web search
- [Skills](/skills) — Create custom AI behaviors with Markdown
- [Personas](/personas) — Customize your AI assistant's personality
- [MCP Client](/mcp) — Connect to external tool servers
- [Knowledge Base](/knowledge-base) — Add RAG-powered document search
- [Chat Platforms](/platforms) — Connect to DingTalk, Discord, or WeChat
- [ACP Server](/acp) — Use Ruri as an agent server in IDEs
