<div align="center">
    <h1>Ruri 琉璃</h1>
    <p><b>一个可自定义的 AI 智能体，使用 Rust + Vue 编写。</b></p>
    <p><a href="README.md">English</a> | <a href="README.zh-CN.md">中文</a></p>
</div>

> [!WARNING]
> 本项目仍在建设中，目前尚不能用于生产环境。

## 特性

- [x] 模型提供商 - Anthropic兼容, OpenAI兼容, 自定义
- [x] 工具调用
- [x] 技能
- [x] 网页搜索
- [x] ACP (Agent Client Protocol) 服务端
- [x] 人格系统
- [x] MCP (Model Context Protocol) 客户端
- [x] 指令系统
- [x] 基于RAG的知识库 (嵌入模型 + 重排序模型) 支持
- [x] 聊天平台 - 钉钉
- [x] 聊天平台 - Discord
- [ ] 聊天平台 - OneBot V11
- [x] 聊天平台 - 个人微信(微信ClawBot插件)

### 未来计划

- 更多模型提供商
- Sub Agent
- 聊天平台 - Matrix
- 聊天平台 - VoceChat
- 聊天平台 - QQ
- 聊天平台 - WeCom
- 聊天平台 - Custom API(You can write your own program to talk to Ruri)

...

## 使用

文档即将添加...

## 开发

```bash
cargo run
```

在 `http://localhost:3000` 打开 Web UI。

### ACP (Agent Client Protocol，Agent客户端协议) 服务端配置

Zed配置范例:

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

## 预览

![preview](.github/preview-cn.png)

## 许可证

[MIT License](./LICENSE) © 2026-PRESENT Vincent-the-gamer
