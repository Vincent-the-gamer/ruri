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
| **Gemini 兼容**    | Google Gemini 系列模型                 | Gemini 2.5 Flash、Gemini 2.5 Pro |

::: tip
不确定选哪个？选 **OpenAI 兼容** 就对了！它是最通用的格式，绝大多数服务商都支持。
:::

## 管理提供商

### 添加提供商

1. 在侧边栏点击 **提供商**
2. 点击 **添加提供商**
3. 选择提供商类型（OpenAI 兼容 / Anthropic 兼容 / Gemini 兼容 / 自定义）
4. 填写名称、API URL、API Key、模型名称和是否支持多模态
5. 点击 **保存**

### 拉取可用模型

Ruri 可以从提供商的 API 自动获取可用模型列表：

1. 进入 **提供商** 页面
2. 点击某个提供商的 **拉取模型** 按钮
3. Ruri 会查询该提供商的 API 并返回所有可用的模型 ID
4. 从列表中选择你想使用的模型

这适用于 OpenAI 兼容和 Anthropic 兼容的 API，让你无需手动查找模型 ID。

### 激活提供商

同一时间只能有一个提供商处于激活状态。在提供商列表中，点击想要使用的提供商的 **激活** 按钮即可切换。

### 编辑和删除

在提供商列表中，你可以随时编辑已有提供商的配置或删除不再需要的提供商。

## 热门提供商快速配置

### OpenAI（GPT-4o）

1. 添加提供商，选择 **OpenAI 兼容**
2. 填写以下信息：

| 字段       | 值                                   |
| ---------- | ------------------------------------ |
| 名称       | `OpenAI`                             |
| API URL    | `https://api.openai.com/v1`          |
| API Key    | 你的 OpenAI API Key（以 `sk-` 开头） |
| 模型       | `gpt-4o`                             |
| 支持多模态 | `true`（GPT-4o 支持图像输入）        |

3. 保存并激活

::: info
API Key 可以在 [OpenAI 平台](https://platform.openai.com/api-keys) 获取。
:::

### Anthropic（Claude Sonnet 4）

1. 添加提供商，选择 **Anthropic 兼容**
2. 填写以下信息：

| 字段       | 值                            |
| ---------- | ----------------------------- |
| 名称       | `Anthropic`                   |
| API URL    | `https://api.anthropic.com`   |
| API Key    | 你的 Anthropic API Key        |
| 模型       | `claude-sonnet-4-20250514`    |
| 支持多模态 | `true`（Claude 支持图像输入） |

3. 保存并激活

::: info
API Key 可以在 [Anthropic 控制台](https://console.anthropic.com/) 获取。
:::

### Google Gemini

1. 添加提供商，选择 **Gemini 兼容**
2. 填写以下信息：

| 字段       | 值                                          |
| ---------- | ------------------------------------------- |
| 名称       | `Gemini`                                    |
| API URL    | `https://generativelanguage.googleapis.com` |
| API Key    | 你的 Gemini API Key                         |
| 模型       | `gemini-2.5-flash`                          |
| 支持多模态 | `true`（Gemini 原生支持图像输入）           |

3. 保存并激活

::: info
API Key 可以在 [Google AI Studio](https://aistudio.google.com/) 免费获取。Gemini 提供慷慨的免费额度，是零成本入门的好选择！
:::

::: tip 模型选择
常用的 Gemini 模型：

- `gemini-2.5-flash` — 快速高效，适合大多数场景
- `gemini-2.5-pro` — 更强推理能力，适合复杂任务
  :::

### DeepSeek

1. 添加提供商，选择 **OpenAI 兼容**
2. 填写以下信息：

| 字段       | 值                            |
| ---------- | ----------------------------- |
| 名称       | `DeepSeek`                    |
| API URL    | `https://api.deepseek.com/v1` |
| API Key    | 你的 DeepSeek API Key         |
| 模型       | `deepseek-chat`               |
| 支持多模态 | `false`                       |

3. 保存并激活

::: info
DeepSeek 兼容 OpenAI API 格式，所以选择 **OpenAI 兼容** 类型。API Key 在 [DeepSeek 平台](https://platform.deepseek.com/) 获取。
:::

### Ollama（本地模型，免费！）

1. 先安装并启动 [Ollama](https://ollama.com)，然后拉取一个模型（如 `ollama pull llama3`）
2. 在 Ruri 中添加提供商，选择 **OpenAI 兼容**
3. 填写以下信息：

| 字段       | 值                                     |
| ---------- | -------------------------------------- |
| 名称       | `Ollama`                               |
| API URL    | `http://localhost:11434/v1`            |
| API Key    | 随意填写（如 `ollama`，Ollama 不校验） |
| 模型       | 你拉取的模型名（如 `llama3`）          |
| 支持多模态 | `false`（除非使用 `llava` 等视觉模型） |

4. 保存并激活

::: tip
Ollama 完全本地运行，**不需要付费**，也不需要网络连接。如果你还没准备好购买 API 服务，不妨先用 Ollama 体验 Ruri 的全部功能！
:::

## 多模态支持

部分模型提供商支持**多模态输入**——即在聊天消息中包含图像。启用后，你可以向 AI 发送图片，AI 可以"看到"并分析它们。

### 工作原理

- 如果提供商的**支持多模态**设置为 `true`，AI 可以接收你在聊天中粘贴或附加的图片
- 如果模型收到图片但不支持，Ruri 会自动回退，去除图片后重试请求——不会报错，只是静默降级
- Gemini 和 Claude 模型原生支持多模态，许多 OpenAI 兼容模型（GPT-4o、GPT-4V）也支持

### 设置多模态支持

添加或编辑提供商时，可以切换**支持多模态**选项：

- **开** — 提供商将接收请求中的图像内容
- **关** — 图像在发送给提供商前被移除（适用于不支持视觉的本地模型）

::: warning
如果为不支持多模态的模型启用了此选项，可能会看到错误。Ruri 会自动回退重试，但最好从一开始就正确设置。
:::

## 小贴士

- 💡 **试试 Ollama** — 免费的本地模型，适合体验和测试，无需 API Key
- 🆓 **试试 Gemini 免费额度** — Google Gemini 提供慷慨的免费额度，零成本开始
- 🔄 **配置多个提供商** — 可以添加多个提供商随时切换，应对不同场景
- 📋 **方案联动** — 在[配置方案](/zh_hans/config-profiles)中绑定不同提供商，一键切换整个配置
- 🔗 **代理服务** — 如果你使用 API 代理（如中转站），只需将 API URL 改为代理地址即可
- ✅ **注意 URL 路径** — OpenAI 兼容的 URL 通常以 `/v1` 结尾，Anthropic 兼容则不需要
- 🔍 **拉取模型** — 使用"拉取模型"按钮从提供商 API 自动发现可用模型
