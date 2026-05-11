---
layout: doc
title: "Model Providers"
lastUpdated: true
---

# Model Providers

Ruri supports multiple model provider types, allowing you to connect to various AI backends. You can manage providers through the Web UI or the REST API.

## Provider Types

### Anthropic Compatible

Connect to the Anthropic API or any Anthropic-compatible endpoint. This provider type uses the Anthropic Messages API format.

**Configuration fields:**

| Field        | Description                                         |
| ------------ | --------------------------------------------------- |
| Name         | A friendly name for this provider                   |
| API URL      | The base URL of the Anthropic-compatible endpoint   |
| API Key      | Your API key for authentication                     |
| Model        | The model identifier (e.g., `claude-sonnet-4-20250514`) |

**Example:** Connect directly to Anthropic's API:

- **API URL:** `https://api.anthropic.com`
- **Model:** `claude-sonnet-4-20250514`

**Example:** Connect to a custom Anthropic-compatible proxy:

- **API URL:** `https://your-proxy.example.com`
- **Model:** `claude-sonnet-4-20250514`

### OpenAI Compatible

Connect to the OpenAI API or any OpenAI-compatible endpoint. This provider type uses the OpenAI Chat Completions API format, which is the de facto standard for many model providers.

**Configuration fields:**

| Field        | Description                                         |
| ------------ | --------------------------------------------------- |
| Name         | A friendly name for this provider                   |
| API URL      | The base URL of the OpenAI-compatible endpoint      |
| API Key      | Your API key for authentication                     |
| Model        | The model identifier (e.g., `gpt-4o`, `deepseek-chat`) |

**Example:** Connect to OpenAI's API:

- **API URL:** `https://api.openai.com/v1`
- **Model:** `gpt-4o`

**Example:** Connect to a compatible provider like DeepSeek:

- **API URL:** `https://api.deepseek.com/v1`
- **Model:** `deepseek-chat`

**Example:** Connect to a local model via Ollama:

- **API URL:** `http://localhost:11434/v1`
- **Model:** `llama3`

### Custom Providers

For providers that don't follow the Anthropic or OpenAI API formats, Ruri supports custom provider configurations. This allows you to define custom request and response mappings.

## Managing Providers

### Via Web UI

1. Navigate to the **Providers** page in the sidebar
2. Click **Add Provider** to create a new provider
3. Fill in the required fields and save
4. Click **Activate** on the provider you want to use

Only one provider can be active at a time. The active provider is used for all chat interactions.

### Via API

You can also manage providers programmatically:

**Create a provider:**

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

**Activate a provider:**

```bash
curl -X POST http://localhost:3000/api/providers/<id>/activate \
  -H "Cookie: session=<your-session-cookie>"
```

See the [API Reference](/api) for the complete list of provider endpoints.

## Switching Providers

You can switch between providers at any time by activating a different one. This is useful for:

- Switching between different models for different tasks
- Testing prompts across multiple providers
- Fallback when a provider is experiencing issues

::: tip
If you have multiple providers configured, you can create different [Config Profiles](/config-profiles) to quickly switch between provider configurations.
:::
