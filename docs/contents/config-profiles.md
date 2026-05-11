---
layout: doc
title: "Config Profiles"
lastUpdated: true
---

# Config Profiles

Config Profiles allow you to create and switch between different configuration presets. Each profile defines a complete set of settings that determine how the AI agent behaves.

## Overview

Instead of manually changing individual settings every time you want the agent to behave differently, you can create multiple profiles and switch between them instantly. This is especially useful when you use Ruri for different purposes.

**Example scenarios:**

- A **coding** profile with a technical persona, file tools, and sandbox mode
- A **writing** profile with a creative persona and web search enabled
- A **research** profile with knowledge base access and web search
- A **platform** profile with chat platform integrations active

## Profile Structure

Each config profile contains the following settings:

| Field                  | Type    | Description                                       |
| ---------------------- | ------- | ------------------------------------------------- |
| `provider`             | string  | The active model provider                         |
| `persona`              | string  | The active persona                                |
| `web_search`           | boolean | Enable or disable web search                      |
| `computer_use`         | boolean | Enable or disable Computer Use                    |
| `computer_use_mode`    | string  | Computer Use runtime mode (`none`, `local`, `sandbox`) |
| `acp`                  | boolean | Enable or disable ACP server                      |
| `active_skills`        | array   | List of skills to enable in this profile           |
| `active_platforms`     | array   | List of platforms to enable in this profile        |
| `active_knowledge_bases` | array | List of knowledge bases to enable in this profile  |
| `proxy`                | object  | Proxy configuration for network requests           |
| `command_prefix`       | string  | Prefix for command recognition (default: `/`)      |

## Managing Profiles

### Via Web UI

1. Navigate to the **Settings** or **Profiles** page
2. View existing profiles
3. Create a new profile with the desired settings
4. Switch between profiles instantly
5. Edit or delete profiles

### Profile Switching

When you switch profiles, all settings in the profile are applied immediately:

- The model provider changes
- The persona is switched
- Tools and skills are enabled/disabled accordingly
- Platform connections are updated
- Knowledge base access is adjusted

## Profile Configuration Details

### Provider

Select which model provider to use when the profile is active. The provider must already be configured in the [Model Providers](/providers) section.

### Persona

Choose the default [persona](/personas) for the profile. This determines the AI's personality and communication style.

### Web Search

When enabled, the `web_search` tool is available to the AI agent. You also need to configure the search provider settings.

### Computer Use

When enabled, the agent can execute system commands. You can specify the runtime mode:

- `none` — Disabled
- `local` — Commands run on the host system
- `sandbox` — Commands run in a sandboxed environment

See [Computer Use](/computer-use) for more details.

### ACP

Enable or disable the [ACP Server](/acp) for this profile. When the ACP server is running, external IDE clients can connect to Ruri as an agent server.

### Active Skills

Specify which [skills](/skills) are available in this profile. Only the listed skills will be active when the profile is in use.

### Active Platforms

Specify which [chat platforms](/platforms) are connected in this profile. This allows you to create profiles that only use specific messaging services.

### Active Knowledge Bases

Specify which [knowledge bases](/knowledge-base) are available for RAG queries in this profile.

### Proxy

Configure network proxy settings for outgoing requests (API calls, web search, etc.):

```yaml
proxy:
  enabled: true
  url: "http://proxy.example.com:8080"
  no_proxy:
    - "localhost"
    - "127.0.0.1"
```

### Command Prefix

Customize the prefix used to identify [commands](/commands). The default is `/`.

## Tips

- **Start with a default profile** — Create a general-purpose profile first, then create specialized profiles for specific use cases
- **Use descriptive names** — Name profiles based on their purpose (e.g., "Coding", "Research", "Casual Chat")
- **Test new settings** — Create a test profile when experimenting with new configurations
- **Keep profiles focused** — A profile should serve a specific purpose rather than trying to do everything
