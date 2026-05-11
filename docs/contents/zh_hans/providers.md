---
layout: doc
title: "模型提供商"
lastUpdated: true
---

# 模型提供商

Ruri 支持多种模型提供商类型，允许您连接到各种 AI 后端。您可以通过 Web UI 或 REST API 管理提供商。

## 提供商类型

### Anthropic 兼容

连接到 Anthropic API 或任何兼容 Anthropic 的端点。此提供商类型使用 Anthropic Messages API 格式。

**配置字段：**

| 字段     | 描述                                           |
| -------- | ---------------------------------------------- |
| 名称     | 此提供商的友好名称                             |
| API URL  | Anthropic 兼容端点的基础 URL                   |
| API Key  | 用于身份验证的 API 密钥                        |
| 模型     | 模型标识符（如 `claude-sonnet-4-20250514`）    |

**示例：** 直接连接到 Anthropic 的 API：

- **API URL：** `https://api.anthropic.com`
- **模型：** `claude-sonnet-4-20250514`

**示例：** 连接到自定义的 Anthropic 兼容代理：

- **API URL：** `https://your-proxy.example.com`
- **模型：** `claude-sonnet-4-20250514`

### OpenAI 兼容

连接到 OpenAI API 或任何兼容 OpenAI 的端点。此提供商类型使用 OpenAI Chat Completions API 格式，这是许多模型提供商的事实标准。

**配置字段：**

| 字段     | 描述                                           |
| -------- | ---------------------------------------------- |
| 名称     | 此提供商的友好名称                             |
| API URL  | OpenAI 兼容端点的基础 URL                      |
| API Key  | 用于身份验证的 API 密钥                        |
| 模型     | 模型标识符（如 `gpt-4o`、`deepseek-chat`）     |

**示例：** 连接到 OpenAI 的 API：

- **API URL：** `https://api.openai.com/v1`
- **模型：** `gpt-4o`

**示例：** 连接到兼容提供商如 DeepSeek：

- **API URL：** `https://api.deepseek.com/v1`
- **模型：** `deepseek-chat`

**示例：** 通过 Ollama 连接本地模型：

- **API URL：** `http://localhost:11434/v1`
- **模型：** `llama3`

### 自定义提供商

对于不遵循 Anthropic 或 OpenAI API 格式的提供商，Ruri 支持自定义提供商配置。这允许您定义自定义的请求和响应映射。

## 管理提供商

### 通过 Web UI

1. 在侧边栏中导航到 **提供商** 页面
2. 点击 **添加提供商** 创建新提供商
3. 填写所需字段并保存
4. 在要使用的提供商上点击 **激活**

同一时间只能有一个提供商处于激活状态。激活的提供商用于所有聊天交互。

### 通过 API

您也可以通过编程方式管理提供商：

**创建提供商：**

```bash
curl -X POST http://localhost:3000/api/providers \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{
    "name": "My Provider",
    "provider_type": "openai_compatible",
    "api_url": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "gpt-4o"
  }'
```

**激活提供商：**

```bash
curl -X POST http://localhost:3000/api/providers/<id>/activate \
  -H "Cookie: session=<your-session-cookie>"
```

完整的提供商端点列表请参阅 [API 参考](/zh_hans/api)。

## 切换提供商

您可以随时通过激活不同的提供商来切换。这在以下场景中非常有用：

- 在不同模型之间切换以处理不同任务
- 跨多个提供商测试提示词
- 当某个提供商出现问题时进行故障转移

::: tip
如果您配置了多个提供商，可以创建不同的[配置方案](/zh_hans/config-profiles)来快速切换提供商配置。
:::
