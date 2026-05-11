---
layout: doc
title: "模型提供商"
lastUpdated: true
---

# 模型提供商

模型提供商是 Ruri 连接 AI 大脑的方式。你需要至少配置一个提供商，AI 才能和你对话。

## 提供商类型简介

Ruri 支持三种提供商类型，覆盖了几乎所有主流 AI 模型服务：

| 类型               | 适用场景                               | 常见服务                         |
| ------------------ | -------------------------------------- | -------------------------------- |
| **OpenAI 兼容**    | 最通用的类型，大多数服务商都兼容此格式 | OpenAI、DeepSeek、Ollama、智谱等 |
| **Anthropic 兼容** | 专为 Anthropic 的 Claude 系列设计      | Anthropic、Claude 代理服务       |
| **自定义**         | 不走标准 API 的情况                    | 自建服务、特殊接口               |

::: tip
不确定选哪个？选 **OpenAI 兼容** 就对了！它是最通用的格式，绝大多数服务商都支持。
:::

## 管理提供商

### 添加提供商

1. 在侧边栏点击 **提供商**
2. 点击 **添加提供商**
3. 选择提供商类型（OpenAI 兼容 / Anthropic 兼容 / 自定义）
4. 填写名称、API URL、API Key 和模型名称
5. 点击 **保存**

### 激活提供商

同一时间只能有一个提供商处于激活状态。在提供商列表中，点击想要使用的提供商的 **激活** 按钮即可切换。

### 编辑和删除

在提供商列表中，你可以随时编辑已有提供商的配置或删除不再需要的提供商。

## 热门提供商快速配置

### OpenAI（GPT-4o）

1. 添加提供商，选择 **OpenAI 兼容**
2. 填写以下信息：

| 字段    | 值                                   |
| ------- | ------------------------------------ |
| 名称    | `OpenAI`                             |
| API URL | `https://api.openai.com/v1`          |
| API Key | 你的 OpenAI API Key（以 `sk-` 开头） |
| 模型    | `gpt-4o`                             |

3. 保存并激活

::: info
API Key 可以在 [OpenAI 平台](https://platform.openai.com/api-keys) 获取。
:::

### Anthropic（Claude Sonnet 4）

1. 添加提供商，选择 **Anthropic 兼容**
2. 填写以下信息：

| 字段    | 值                          |
| ------- | --------------------------- |
| 名称    | `Anthropic`                 |
| API URL | `https://api.anthropic.com` |
| API Key | 你的 Anthropic API Key      |
| 模型    | `claude-sonnet-4-20250514`  |

3. 保存并激活

::: info
API Key 可以在 [Anthropic 控制台](https://console.anthropic.com/) 获取。
:::

### DeepSeek

1. 添加提供商，选择 **OpenAI 兼容**
2. 填写以下信息：

| 字段    | 值                            |
| ------- | ----------------------------- |
| 名称    | `DeepSeek`                    |
| API URL | `https://api.deepseek.com/v1` |
| API Key | 你的 DeepSeek API Key         |
| 模型    | `deepseek-chat`               |

3. 保存并激活

::: info
DeepSeek 兼容 OpenAI API 格式，所以选择 **OpenAI 兼容** 类型。API Key 在 [DeepSeek 平台](https://platform.deepseek.com/) 获取。
:::

### Ollama（本地模型，免费！）

1. 先安装并启动 [Ollama](https://ollama.com)，然后拉取一个模型（如 `ollama pull llama3`）
2. 在 Ruri 中添加提供商，选择 **OpenAI 兼容**
3. 填写以下信息：

| 字段    | 值                                     |
| ------- | -------------------------------------- |
| 名称    | `Ollama`                               |
| API URL | `http://localhost:11434/v1`            |
| API Key | 随意填写（如 `ollama`，Ollama 不校验） |
| 模型    | 你拉取的模型名（如 `llama3`）          |

4. 保存并激活

::: tip
Ollama 完全本地运行，**不需要付费**，也不需要网络连接。如果你还没准备好购买 API 服务，不妨先用 Ollama 体验 Ruri 的全部功能！
:::

## 小贴士

- 💡 **试试 Ollama** — 免费的本地模型，适合体验和测试，无需 API Key
- 🔄 **配置多个提供商** — 可以添加多个提供商随时切换，应对不同场景
- 📋 **方案联动** — 在[配置方案](/zh_hans/config-profiles)中绑定不同提供商，一键切换整个配置
- 🔗 **代理服务** — 如果你使用 API 代理（如中转站），只需将 API URL 改为代理地址即可
- ✅ **注意 URL 路径** — OpenAI 兼容的 URL 通常以 `/v1` 结尾，Anthropic 兼容则不需要
