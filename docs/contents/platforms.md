---
layout: doc
title: "Chat Platforms"
lastUpdated: true
---

# Chat Platforms

Ruri can connect to popular chat platforms so you can interact with the AI from your favorite messaging app. No more switching between your browser and your chat tools!

## Supported Platforms

| Platform     | What It Is                                     |
| ------------ | ---------------------------------------------- |
| **DingTalk** | 钉钉 — Alibaba's enterprise messaging platform |
| **Discord**  | Discord bot integration                        |
| **WeChat**   | Personal WeChat via Wechat ClawBot             |

::: tip
The AI behaves consistently across all platforms — same persona, skills, and tools regardless of where you message it from.
:::

## Setting Up DingTalk (钉钉)

DingTalk integration lets the AI respond to messages in group chats or direct messages.

### What You'll Need

- A DingTalk developer account
- A DingTalk custom robot application

### Step-by-Step

1. Go to the **Platforms** page in Ruri's Web UI
2. Click **Add Platform** and select **DingTalk**
3. Fill in your DingTalk credentials:
   - **App Key** — From your DingTalk application
   - **App Secret** — From your DingTalk application
   - **Robot Code** — The robot's identifier
   - **Keyword** — (Optional) A keyword that triggers the bot in group chats
4. Save and enable the platform
5. Head to DingTalk and send a message to your robot — Ruri will respond!

## Setting Up Discord

Discord integration connects Ruri as a bot that can respond to messages in channels and DMs.

### What You'll Need

- A Discord application and bot token from the [Discord Developer Portal](https://discord.com/developers/applications)

### Step-by-Step

1. Go to the **Platforms** page in Ruri's Web UI
2. Click **Add Platform** and select **Discord**
3. Fill in your Discord credentials:
   - **Token** — Your Discord bot token
   - **Application ID** — Your Discord application ID
   - **Guild ID** — (Optional) Restrict the bot to a specific server
4. Save and enable the platform
5. Invite the bot to your Discord server using the OAuth2 URL from the Developer Portal
6. Mention or DM the bot to start chatting!

::: tip
Make sure your Discord bot has the "Message Content Intent" enabled in the Developer Portal, otherwise it won't be able to read messages.
:::

## Setting Up WeChat

Personal WeChat integration uses the Wechat ClawBot framework to connect Ruri to your personal WeChat account.

### What You'll Need

- A running Wechat ClawBot server

### Step-by-Step

1. Go to the **Platforms** page in Ruri's Web UI
2. Click **Add Platform** and select **WeChat**
3. Fill in your credentials:
   - **Base URL** — The URL of your Wechat ClawBot server
   - **Token** — Your authentication token
4. Save and enable the platform
5. Send a message to your WeChat account — Ruri will respond!

## Managing Platforms

### Via Web UI

1. Go to **Platforms** in the sidebar
2. **View** all configured platforms and their connection status
3. **Enable/disable** individual platforms with the toggle
4. **Edit** platform configurations
5. **Remove** platforms you no longer need

### Hot-Reload

When you edit platform settings in the Web UI, changes take effect immediately — no server restart needed. This means you can:

- Update API tokens without downtime
- Add or remove platforms on the fly
- Adjust settings in real-time

## Platforms and Config Profiles

You can control which platforms are active per [Config Profile](/config-profiles). This lets you:

- Enable different platform sets for different use cases
- Test a platform setup without enabling it everywhere
- Create profiles that only connect to specific messaging services

For example, you could have a "Work" profile that only connects to DingTalk, and a "Personal" profile that only connects to Discord.

## Tips

- **Start with one platform** — Set up and test one integration before adding more
- **Check bot permissions** — Make sure your bot has the right permissions in each platform to read and send messages
- **Use different profiles** — Keep work and personal platforms separate with [Config Profiles](/config-profiles)
- **Monitor connection status** — Check the Platforms page periodically to make sure everything is connected
