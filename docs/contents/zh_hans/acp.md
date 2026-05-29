---
layout: doc
title: "在 IDE 中使用 Ruri"
lastUpdated: true
---

# 在 IDE 中使用 Ruri

你可以直接在代码编辑器中使用 Ruri 的 AI 功能！通过 ACP（智能体客户端协议），Ruri 可以作为 AI 助手集成到 Zed、JetBrains 等 IDE 中。

## 支持的 IDE

| IDE                 | 说明                                |
| ------------------- | ----------------------------------- |
| **Zed**             | 高性能代码编辑器，原生支持 ACP      |
| **JetBrains**       | IntelliJ IDEA、PyCharm、WebStorm 等 |
| **其他 ACP 客户端** | 任何支持智能体客户端协议的工具      |

![ACP 配置](/ruri-pics/zh_hans/acp-config-cn.png)

## 配置指南

### 在 Zed 中使用 Ruri

1. 确认 Ruri 已安装并可用（`ruri --acp` 命令能正常运行）
2. 打开 Zed 的设置文件
3. 在 `agent_servers` 部分添加：

```json
{
  "agent_servers": {
    "ruri": {
      "type": "custom",
      "command": "/path/to/ruri",
      "args": ["--acp"]
    }
  }
}
```

4. 将 `/path/to/ruri` 替换为你 Ruri 二进制文件的实际路径
5. 保存设置后，Ruri 就可以作为 AI 智能体在 Zed 中使用了

### 在 JetBrains 中使用 Ruri

1. 确认 Ruri 已安装并可用
2. 在 JetBrains IDE 中打开 AI 助手设置
3. 将 Ruri 配置为外部智能体服务器
4. 命令设置为 Ruri 的路径，参数添加 `--acp`
5. 保存后即可使用

## 在 IDE 中你能做什么？

在 IDE 中使用 Ruri 时，AI 拥有和 Web UI 相同的能力：

- 📖 **阅读代码** — AI 可以读取项目文件、理解代码结构
- ✏️ **编辑代码** — AI 可以直接修改和创建文件
- 🔍 **搜索代码** — AI 可以搜索项目中的代码和文件
- 💻 **执行命令** — 如果启用了 Computer Use，AI 可以运行构建和测试命令
- 🛠️ **使用技能** — 当前配置方案中的活跃技能都可以使用
- 🔄 **流式响应** — AI 的输出会实时流式传输到你的 IDE
- 🌐 **独立代理** — ACP 有自己的代理配置，可以独立路由大模型请求

::: tip
在 IDE 中使用时，确保[配置方案](/zh_hans/config-profiles)中的设置符合你的需求。你可以为 IDE 使用创建一个专用的方案。
:::

## 工作原理

在 IDE 模式下，Ruri 作为一个后台服务运行：

1. IDE 将你的请求发送给 Ruri
2. Ruri 使用当前的人格、技能和工具来处理
3. AI 生成响应（可能会使用文件操作等工具）
4. 结果返回给 IDE

对 AI 来说，无论请求来自 Web UI 还是 IDE，处理方式都是一样的。

## 常见问题

### IDE 中 AI 没有响应

- 确认 Ruri 二进制文件路径正确
- 试试在终端手动运行 `ruri --acp`，看是否有报错
- 确认 Ruri 有执行权限

### 工具不可用

- 检查当前[配置方案](/zh_hans/config-profiles)中是否启用了所需工具
- Computer Use（Shell 命令等）需要在方案中单独开启
- 检查技能的 `allowed_tools` 是否限制了对某些工具的访问

### 模型无响应

- 确认提供商已正确配置并激活
- 检查 API Key 是否有效
- 确认网络连接正常
