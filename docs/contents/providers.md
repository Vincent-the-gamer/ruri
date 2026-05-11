---
layout: doc
title: "Model Providers"
lastUpdated: true
---

# Model Providers

Model providers are the AI backends that power Ruri's conversations. You can connect to cloud providers like OpenAI and Anthropic, or run models locally on your own machine.

## Provider Types

Ruri supports three types of model providers:

- **Anthropic Compatible** — For Anthropic's Claude models or any service that uses the Anthropic API format
- **OpenAI Compatible** — For OpenAI models or any service that uses the OpenAI API format (this covers most providers!)
- **Custom** — For providers with unique API formats

::: tip
Most providers you'll encounter use the OpenAI Compatible format. DeepSeek, Ollama, and many others all work with this option.
:::

## Managing Providers

### Adding a Provider

1. Navigate to the **Providers** page in the sidebar
2. Click **Add Provider**
3. Select the provider type
4. Fill in the details:
   - **Name** — A friendly name (e.g., "My OpenAI")
   - **API URL** — The endpoint URL
   - **API Key** — Your authentication key
   - **Model** — The model identifier
5. Click **Save**
6. Click **Activate** on the provider you want to use

::: info
Only one provider can be active at a time. The active provider is used for all chat interactions.
:::

### Switching Providers

You can switch between providers at any time — just click **Activate** on a different one. This is useful for:

- Using different models for different tasks
- Testing how different models handle the same prompt
- Falling back to another provider if one is having issues

::: tip
If you frequently switch between providers, check out [Config Profiles](/config-profiles) — you can create profiles with different providers and switch between them instantly.
:::

### Editing or Removing a Provider

1. Go to the **Providers** page
2. Click on a provider to edit its settings, or click **Delete** to remove it
3. Remember: you can't delete the currently active provider — switch to another one first

## Popular Provider Setup

### OpenAI (GPT-4o)

1. Go to **Providers** → **Add Provider**
2. Select **OpenAI Compatible**
3. Fill in:
   - **Name:** `OpenAI`
   - **API URL:** `https://api.openai.com/v1`
   - **API Key:** Your OpenAI API key (starts with `sk-`)
   - **Model:** `gpt-4o`
4. Save and activate

::: info
Get your API key from [platform.openai.com](https://platform.openai.com/api-keys). You'll need an OpenAI account with billing enabled.
:::

### Anthropic (Claude Sonnet 4)

1. Go to **Providers** → **Add Provider**
2. Select **Anthropic Compatible**
3. Fill in:
   - **Name:** `Anthropic`
   - **API URL:** `https://api.anthropic.com`
   - **API Key:** Your Anthropic API key
   - **Model:** `claude-sonnet-4-20250514`
4. Save and activate

::: info
Get your API key from [console.anthropic.com](https://console.anthropic.com/).
:::

### DeepSeek

1. Go to **Providers** → **Add Provider**
2. Select **OpenAI Compatible**
3. Fill in:
   - **Name:** `DeepSeek`
   - **API URL:** `https://api.deepseek.com/v1`
   - **API Key:** Your DeepSeek API key
   - **Model:** `deepseek-chat`
4. Save and activate

::: info
Get your API key from [platform.deepseek.com](https://platform.deepseek.com/). DeepSeek offers competitive pricing and strong coding capabilities.
:::

### Ollama (Local, Free)

Ollama lets you run AI models on your own computer — no API key needed, no usage fees!

**Step 1: Install Ollama**

Download and install Ollama from [ollama.com](https://ollama.com).

**Step 2: Download a model**

Open a terminal and pull a model:

```bash
ollama pull llama3
```

Other popular models: `llama3.1`, `mistral`, `codellama`, `qwen2`

**Step 3: Add Ollama as a provider in Ruri**

1. Go to **Providers** → **Add Provider**
2. Select **OpenAI Compatible**
3. Fill in:
   - **Name:** `Ollama`
   - **API URL:** `http://localhost:11434/v1`
   - **API Key:** Anything (e.g., `ollama`) — Ollama doesn't require a real key
   - **Model:** `llama3` (or whichever model you pulled)
4. Save and activate

::: tip
Ollama is the easiest way to try Ruri for free. The quality of responses depends on your hardware — larger models need more RAM and GPU. Start with `llama3` (8B) and experiment from there!
:::

## Tips

- **Try Ollama first** — If you're new to Ruri, start with a free local model via Ollama before signing up for paid API services
- **Use a proxy provider** — If you can't access certain APIs directly, use an OpenAI-compatible proxy service and set the API URL to your proxy endpoint
- **Keep your API keys safe** — Never share your API keys or commit them to version control
- **Monitor your usage** — Cloud providers charge per token. Keep an eye on your usage dashboard to avoid surprises
- **Multiple providers for fallback** — Set up two or more providers so you can quickly switch if one goes down
