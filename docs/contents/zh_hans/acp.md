---
layout: doc
title: "ACP 服务端"
lastUpdated: true
---

# ACP 服务端

**智能体客户端协议（Agent Client Protocol，ACP）** 服务器允许 Ruri 作为智能体服务器，可以集成到支持 ACP 标准的 IDE 和其他工具中。

## 概述

在 ACP 模式下运行时，Ruri 通过 stdio 传输进行通信，兼容以下客户端：

- **Zed** — 高性能代码编辑器
- **JetBrains** — IntelliJ IDEA、PyCharm、WebStorm 等 IDE
- **其他 ACP 兼容客户端** — 任何实现了智能体客户端协议的工具

这使您能够在开发环境中直接使用 Ruri 的 AI 功能，完全访问工具、技能和模型提供商。

## 以 ACP 模式启动

使用 `--acp` 标志将 Ruri 作为 ACP 服务器启动：

```bash
ruri --acp
```

使用 `--acp` 启动时，Ruri 会：

1. 通过 stdio（标准输入/输出）进行通信
2. 处理 ACP 协议消息
3. 不启动 Web UI 服务器
4. 使用活跃的配置方案

## 配置

### 在 Zed 中配置

要将 Ruri 用作 Zed 中的智能体服务器，将以下内容添加到您的 Zed 设置中：

```json
{
  "agent_servers": {
    "ruri": {
      "type": "custom",
      "command": "/<path_to>/ruri",
      "args": ["--acp"]
    }
  }
}
```

将 `/<path_to>/ruri` 替换为您 Ruri 二进制文件的实际路径。

### 在 JetBrains 中配置

在 JetBrains IDE 中，通过 AI 助手设置将 Ruri 配置为外部智能体服务器。将命令指向带有 `--acp` 参数的 Ruri 二进制文件。

### ACP 配置

您可以通过 API 管理 ACP 配置：

**获取 ACP 配置：**

```bash
curl http://localhost:3000/api/acp/config \
  -H "Cookie: session=<your-session-cookie>"
```

**更新 ACP 配置：**

```bash
curl -X PUT http://localhost:3000/api/acp/config \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{
    "enabled": true,
    "allowed_tools": ["read_file", "write_file", "edit_file", "bash"]
  }'
```

## 工作原理

ACP 通信流程：

```
IDE/客户端 ←→ stdio ←→ Ruri ACP 服务器 ←→ AI 模型
```

1. IDE 通过 stdio 使用 ACP 协议发送请求
2. Ruri 解析请求并创建聊天消息
3. 消息通过活跃人格、技能和工具流程处理
4. AI 模型生成响应，可能使用工具
5. 响应通过 stdio 发送回 IDE

## ACP 模式中的可用功能

作为 ACP 服务器运行时，Ruri 提供：

- **所有模型提供商** — 使用任何已配置的提供商
- **内置工具** — 文件操作、搜索和可选的 bash 命令
- **技能** — 当前配置方案中的活跃技能
- **人格** — 当前方案中的活跃人格
- **知识库** — 如果已配置且在方案中激活

::: tip
如果您希望智能体通过 ACP 连接执行 Shell 命令，请确保在[配置方案](/zh_hans/config-profiles)中启用 Computer Use。
:::

## 安全注意事项

- ACP 服务器以与 Ruri 进程相同的权限运行
- 工具执行权限遵循当前[配置方案](/zh_hans/config-profiles)设置
- 仅在受信任的环境中启用 `bash` 工具
- 使用[配置方案](/zh_hans/config-profiles)创建具有有限工具访问权限的受限方案用于 ACP 使用

## 故障排除

### ACP 服务器无响应

- 确保使用 `--acp` 标志启动 Ruri
- 检查 IDE 配置中的二进制文件路径是否正确
- 确认 Ruri 二进制文件具有执行权限

### 工具不可用

- 检查工具是否在活跃的[配置方案](/zh_hans/config-profiles)中启用
- 确保为 `bash` 和 Shell 工具启用了 Computer Use
- 验证技能是否未通过 `allowed_tools` 限制可用工具

### 模型无响应

- 验证模型提供商已配置并激活
- 检查 API 密钥有效性
- 确保 ACP 服务器可以访问模型提供商 API 的网络
