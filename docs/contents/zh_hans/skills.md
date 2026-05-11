---
layout: doc
title: "技能系统"
lastUpdated: true
---

# 技能系统

技能是自定义 Ruri 行为的主要机制。技能是一个带有 YAML frontmatter 的 Markdown 文件，定义了 AI 智能体在特定上下文中的行为方式。

## 概述

技能允许您：

- 为特定任务定义专业化行为
- 控制智能体可以使用哪些工具
- 为每个技能设置自定义模型参数和人格
- 通过钩子创建触发式行为
- 将技能打包为 ZIP 压缩包并分享

## 技能文件格式

每个技能都是一个 Markdown 文件（`.md`），包含 YAML frontmatter。frontmatter 包含元数据和配置，而正文包含技能提示词或指令。

```markdown
---
name: "code-review"
description: "审查代码变更并提供反馈"
when_to_use: "当用户要求进行代码审查，或检测到代码变更时"
argument_hint: "要审查的文件或目录路径"
arguments:
  - name: "path"
    description: "要审查的文件或目录路径"
    required: true
user_invocable: true
allowed_tools:
  - read_file
  - search_files
  - list_directory
model: "claude-sonnet-4-20250514"
effort: "high"
---

你是一名代码审查员。分析指定路径的代码并提供：
1. 代码功能的概述
2. 潜在的缺陷或问题
3. 改进建议
4. 代码风格观察
```

## Frontmatter 字段

| 字段                       | 类型    | 默认值   | 描述                                               |
| -------------------------- | ------- | -------- | -------------------------------------------------- |
| `name`                     | string  | 必填     | 技能的唯一标识符                                   |
| `description`              | string  | 必填     | 技能功能的人类可读描述                             |
| `when_to_use`              | string  | —        | 描述何时应自动调用此技能                           |
| `argument_hint`            | string  | —        | 描述预期参数的提示文本                             |
| `arguments`                | array   | `[]`     | 参数定义列表（见下文）                             |
| `disable_model_invocation` | boolean | `false`  | 如果为 true，模型不会自动调用此技能                |
| `user_invocable`           | boolean | `true`   | 用户是否可以手动触发此技能                         |
| `allowed_tools`            | array   | 所有工具 | 此技能允许使用的工具名称列表                       |
| `model`                    | string  | —        | 覆盖此技能的活跃模型                               |
| `effort`                   | string  | —        | 模型推理力度（如 `low`、`medium`、`high`）         |
| `context`                  | array   | —        | 技能运行时要包含的附加上下文文件                   |
| `agent`                    | string  | —        | 要使用的智能体配置                                  |
| `hooks`                    | object  | —        | 生命周期钩子（见下文）                              |
| `paths`                    | array   | —        | 触发此技能的文件路径模式                            |
| `shell`                    | string  | —        | 在技能执行前或执行期间运行的 Shell 命令             |

### 参数（Arguments）

`arguments` 数组中的每个参数具有以下结构：

| 字段          | 类型    | 必填 | 描述                 |
| ------------- | ------- | ---- | -------------------- |
| `name`        | string  | 是   | 参数名称             |
| `description` | string  | 是   | 参数描述             |
| `required`    | boolean | 否   | 此参数是否为必填     |

### 钩子（Hooks）

钩子允许您在技能生命周期的特定点运行自定义逻辑：

- **执行前钩子**：在技能提示词发送给模型之前运行
- **执行后钩子**：在模型生成响应之后运行

## 管理技能

### 通过 Web UI

1. 在侧边栏中导航到 **技能** 页面
2. 浏览已安装的技能列表
3. 使用开关切换技能的启用/禁用状态
4. 手动添加新技能或上传技能包

### 通过 API

**列出所有技能：**

```bash
curl http://localhost:3000/api/skills \
  -H "Cookie: session=<your-session-cookie>"
```

**添加技能：**

```bash
curl -X POST http://localhost:3000/api/skills \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{
    "name": "my-skill",
    "content": "---\nname: my-skill\ndescription: 我的自定义技能\n---\n做一些有用的事情。"
  }'
```

**上传技能包（ZIP）：**

```bash
curl -X POST http://localhost:3000/api/skills/upload \
  -H "Cookie: session=<your-session-cookie>" \
  -F "file=@my-skills.zip"
```

**切换技能启用/禁用：**

```bash
curl -X PATCH http://localhost:3000/api/skills/my-skill \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{"enabled": true}'
```

**删除技能：**

```bash
curl -X DELETE http://localhost:3000/api/skills/my-skill \
  -H "Cookie: session=<your-session-cookie>"
```

## 技能包

技能可以打包为 ZIP 压缩包，便于分享和分发。技能包是一个包含一个或多个技能 Markdown 文件的 ZIP 文件。

**创建技能包：**

1. 创建您的技能 Markdown 文件
2. 将它们放入一个目录中
3. 将目录压缩为 ZIP 压缩包
4. 通过 API 或 Web UI 上传

::: tip
创建技能包时，确保每个 Markdown 文件都有有效的 YAML frontmatter，至少包含 `name` 和 `description` 字段。
:::

## 技能激活

技能可以通过以下方式激活或停用：

1. **Web UI 开关** — 在技能页面使用开关
2. **API** — 使用 `PATCH /api/skills/:name` 端点
3. **配置方案** — 在配置方案的活跃技能列表中包含特定技能

当技能被停用时，它不会对 AI 模型或用户可用，但其定义会被保留。
