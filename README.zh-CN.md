<div align="center">
    <h1>Ruri 琉璃</h1>
    <p><b>一个可自定义的 AI 智能体，使用 Rust + Vue 编写。</b></p>
    <p><a href="README.md">English</a> | <a href="README.zh-CN.md">中文</a></p>
</div>

> [!WARNING]
> 本项目仍在建设中，目前尚不能用于生产环境。

## 计划

- [x] 工具调用
- [x] 技能系统
- [x] 网络搜索
- [x] ACP (Agent Client Protocol)
- [x] 人设系统
- [x] MCP (Model Context Protocol)
- [x] 指令系统
- [ ] 基于RAG的知识库 (嵌入模型 + 重排序模型) 支持
- [ ] Sub Agent
- [x] 聊天 - Discord
- [ ] 聊天 - OneBot V11
- [ ] 聊天 - 微信 (Wechat ClawBot)
- [ ] 聊天 - Matrix
- [ ] 聊天 - VoceChat
- [ ] 聊天 - QQ
- [x] 聊天 - 钉钉
- [ ] 聊天 - 企业微信
- [ ] 聊天 - 自定义 API（你可以编写自己的程序与琉璃对话）

...

## 使用

文档即将添加...

## 开发

```bash
cargo run
```

在 `http://localhost:3000` 打开 Web UI。

### ACP (Agent Client Protocol，Agent客户端协议)

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
