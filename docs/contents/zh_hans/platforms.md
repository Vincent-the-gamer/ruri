---
layout: doc
title: "聊天平台"
lastUpdated: true
---

# 聊天平台

Ruri 可以连接到多个聊天平台，允许用户通过他们偏好的消息服务与 AI 智能体交互。支持的平台包括钉钉、Discord 和个人微信。

## 概述

| 平台     | 标识符            | 描述                                 |
| -------- | ----------------- | ------------------------------------ |
| 钉钉     | `dingtalk`        | 阿里巴巴的企业通讯平台               |
| Discord  | `discord`         | Discord 机器人集成                   |
| 微信     | `wechat_clawbot`  | 通过 Wechat ClawBot 的个人微信集成   |

所有平台配置通过 `platforms.yaml` 配置文件管理，并支持热重载。

## 配置

平台设置在 `platforms.yaml` 中定义。当此文件被修改时，Ruri 会自动重新加载配置而无需重启。

### 钉钉

钉钉集成允许 AI 智能体在钉钉群聊或私聊中响应消息。

**配置字段：**

| 字段          | 描述                           |
| ------------- | ------------------------------ |
| `app_key`     | 钉钉应用 Key                   |
| `app_secret`  | 钉钉应用 Secret                |
| `robot_code`  | 机器人的代码标识符             |
| `keyword`     | 可选的机器人触发关键词         |

### Discord

Discord 集成将 AI 智能体连接为 Discord 机器人，可以在频道和私聊中响应消息。

**配置字段：**

| 字段             | 描述                              |
| ---------------- | --------------------------------- |
| `token`          | Discord 机器人令牌                |
| `application_id` | Discord 应用 ID                   |
| `guild_id`       | （可选）限制到特定服务器          |

### 微信（Wechat ClawBot）

个人微信集成使用 Wechat ClawBot 框架将 AI 智能体连接到个人微信账号。

**配置字段：**

| 字段       | 描述                       |
| ---------- | -------------------------- |
| `base_url` | Wechat ClawBot 服务器 URL  |
| `token`    | 身份验证令牌               |

## 热重载

平台系统的一个关键特性是**热重载**。当您修改 `platforms.yaml` 时，Ruri 会自动检测变更并重新加载平台配置，无需重启服务器。

这使得：

- 无需停机即可更新 API 令牌
- 动态添加或移除平台连接
- 实时调整平台特定设置

## 管理平台

### 通过 Web UI

1. 导航到 **平台** 页面
2. 查看已配置的平台及其连接状态
3. 启用或禁用单个平台
4. 编辑平台配置

### 通过 API

平台 CRUD 端点可用于编程管理：

```bash
# 列出所有平台
curl http://localhost:3000/api/platforms \
  -H "Cookie: session=<your-session-cookie>"

# 创建平台
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

## 平台与配置方案

您可以在每个[配置方案](/zh_hans/config-profiles)中控制哪些平台处于活跃状态。这允许您：

- 为不同用例启用不同的平台集
- 测试平台配置而无需在生产环境中启用
- 创建仅使用特定平台的方案

## 消息流程

当从平台收到消息时：

1. 平台适配器接收传入消息
2. 消息通过活跃人格和技能流程处理
3. AI 模型生成响应（可能使用工具）
4. 响应通过平台适配器发送回去

AI 智能体在所有平台上的行为一致，使用相同的人格、技能和工具，无论消息来自哪个平台。
