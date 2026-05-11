---
layout: doc
title: "Chat Platforms"
lastUpdated: true
---

# Chat Platforms

Ruri can connect to multiple chat platforms, allowing users to interact with the AI agent through their preferred messaging service. Supported platforms include DingTalk, Discord, and personal WeChat.

## Overview

| Platform   | Identifier        | Description                          |
| ---------- | ----------------- | ------------------------------------ |
| DingTalk   | `dingtalk`        | 钉钉 — Alibaba's enterprise messaging platform |
| Discord    | `discord`         | Discord bot integration              |
| WeChat     | `wechat_clawbot`  | Personal WeChat via Wechat ClawBot   |

All platform configurations are managed through the `platforms.yaml` configuration file and support hot-reload.

## Configuration

Platform settings are defined in `platforms.yaml`. When this file is modified, Ruri automatically reloads the configuration without restarting.

### DingTalk (钉钉)

DingTalk integration allows the AI agent to respond to messages in DingTalk group chats or direct messages.

**Configuration fields:**

| Field            | Description                                    |
| ---------------- | ---------------------------------------------- |
| `app_key`        | DingTalk application key                       |
| `app_secret`     | DingTalk application secret                    |
| `robot_code`     | The robot's code identifier                   |
| `keyword`        | Optional keyword trigger for the bot           |

### Discord

Discord integration connects the AI agent as a Discord bot that can respond to messages in channels and direct messages.

**Configuration fields:**

| Field            | Description                                    |
| ---------------- | ---------------------------------------------- |
| `token`          | Discord bot token                              |
| `application_id` | Discord application ID                         |
| `guild_id`       | (Optional) Restrict to a specific server       |

### WeChat (Wechat ClawBot)

Personal WeChat integration uses the Wechat ClawBot framework to connect the AI agent to a personal WeChat account.

**Configuration fields:**

| Field            | Description                                    |
| ---------------- | ---------------------------------------------- |
| `base_url`       | Wechat ClawBot server URL                      |
| `token`          | Authentication token                           |

## Hot-Reload

One of the key features of the platform system is **hot-reload**. When you modify `platforms.yaml`, Ruri automatically detects the changes and reloads the platform configuration without requiring a server restart.

This enables:

- Updating API tokens without downtime
- Adding or removing platform connections on the fly
- Adjusting platform-specific settings in real-time

## Managing Platforms

### Via Web UI

1. Navigate to the **Platforms** page
2. View configured platforms and their connection status
3. Enable or disable individual platforms
4. Edit platform configurations

### Via API

Platform CRUD endpoints are available for programmatic management:

```bash
# List all platforms
curl http://localhost:3000/api/platforms \
  -H "Cookie: session=<your-session-cookie>"

# Create a platform
curl -X POST http://localhost:3000/api/platforms \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{
    "type": "discord",
    "config": {
      "token": "your-bot-token",
      "application_id": "your-app-id"
    }
  }'
```

## Platform and Config Profiles

You can control which platforms are active per [Config Profile](/config-profiles). This allows you to:

- Enable different platform sets for different use cases
- Test a platform configuration without enabling it in production
- Create profiles that only use specific platforms

## Message Flow

When a message is received from a platform:

1. The platform adapter receives the incoming message
2. The message is processed through the active persona and skill pipeline
3. The AI model generates a response (potentially using tools)
4. The response is sent back through the platform adapter

The AI agent behaves consistently across all platforms, using the same persona, skills, and tools regardless of which platform the message came from.
