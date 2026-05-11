---
layout: doc
title: "聊天平台"
lastUpdated: true
---

# 聊天平台

Ruri 不只是一个网页应用 — 你还可以在钉钉、Discord、微信等平台上和 AI 聊天。配置一次，多平台同步使用。

## 支持的平台

| 平台           | 说明                                    |
| -------------- | --------------------------------------- |
| 💬 **钉钉**    | 企业群聊和私聊中使用 AI                 |
| 🎮 **Discord** | 在 Discord 服务器中部署 AI 机器人       |
| 💚 **微信**    | 通过 Wechat ClawBot 在个人微信中使用 AI |

::: tip
在任何平台上，AI 都使用相同的人格、技能和工具，体验完全一致。
:::

## 配置教程

### 钉钉

#### 第 1 步：创建钉钉应用

1. 登录[钉钉开发者后台](https://open-dev.dingtalk.com/)
2. 创建一个企业内部应用
3. 获取 App Key 和 App Secret
4. 开启机器人功能，获取 Robot Code

#### 第 2 步：在 Ruri 中配置

1. 在 Web UI 侧边栏点击 **平台**
2. 添加钉钉平台
3. 填写 App Key、App Secret 和 Robot Code
4. 如果需要，设置触发关键词
5. 保存并启用

#### 配置字段

| 字段       | 说明                       | 在哪获取       |
| ---------- | -------------------------- | -------------- |
| App Key    | 应用标识                   | 钉钉开发者后台 |
| App Secret | 应用密钥                   | 钉钉开发者后台 |
| Robot Code | 机器人代码                 | 钉钉开发者后台 |
| 关键词     | 触发机器人的关键词（可选） | 自定义         |

### Discord

#### 第 1 步：创建 Discord 机器人

1. 前往 [Discord 开发者门户](https://discord.com/developers/applications)
2. 创建新应用，添加 Bot
3. 复制 Bot Token
4. 记录 Application ID
5. 将机器人邀请到你的服务器

#### 第 2 步：在 Ruri 中配置

1. 在 Web UI 侧边栏点击 **平台**
2. 添加 Discord 平台
3. 填写 Token 和 Application ID
4. 如需限制到特定服务器，填写 Guild ID
5. 保存并启用

#### 配置字段

| 字段           | 说明                                | 在哪获取           |
| -------------- | ----------------------------------- | ------------------ |
| Token          | 机器人令牌                          | Discord 开发者门户 |
| Application ID | 应用 ID                             | Discord 开发者门户 |
| Guild ID       | 服务器 ID，限制到特定服务器（可选） | Discord 服务器设置 |

### 微信（Wechat ClawBot）

#### 第 1 步：部署 Wechat ClawBot

参考 [Wechat ClawBot](https://github.com/WechatClawBot) 项目文档部署服务。

#### 第 2 步：在 Ruri 中配置

1. 在 Web UI 侧边栏点击 **平台**
2. 添加微信平台
3. 填写 Wechat ClawBot 的 Base URL 和 Token
4. 保存并启用

#### 配置字段

| 字段     | 说明                      |
| -------- | ------------------------- |
| Base URL | Wechat ClawBot 服务器地址 |
| Token    | 身份验证令牌              |

## 管理平台

在 Web UI 的平台页面，你可以：

- 📋 查看所有已配置平台及其连接状态
- ✅ 启用或禁用单个平台
- ✏️ 编辑平台配置
- 🔄 修改配置后自动生效，无需重启

::: info
平台配置支持热重载 — 修改配置后会自动更新，不需要重启 Ruri。
:::

## 平台与配置方案

你可以在[配置方案](/zh_hans/config-profiles)中控制每个方案启用哪些平台。这让你可以：

- 🏢 工作方案只启用钉钉
- 🎮 个人方案只启用 Discord
- 🧪 测试方案不启用任何平台

## 消息如何处理

无论消息来自哪个平台，处理流程都是一样的：

1. 平台收到你的消息
2. 消息进入 AI 处理流程（应用当前人格和技能）
3. AI 生成回复（可能会使用工具）
4. 回复通过平台发送给你

所以，无论你在哪里和 AI 对话，它都是同一个 AI，拥有相同的能力。
