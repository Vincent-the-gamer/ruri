---
layout: doc
title: "技能系统"
lastUpdated: true
---

# 技能系统

技能让你可以教会 Ruri 新的能力。一个技能本质上就是一个 Markdown 文件，告诉 AI 在特定任务中应该如何表现 — 比如审查代码、编写文档或翻译文本。

## 什么是技能？

把技能想象成你给 AI 的自定义指令。每个技能定义：

- **AI 应该做什么** — 用 Markdown 写成的指令
- **何时使用** — 告诉 AI 何时该技能相关的触发器
- **可以使用哪些工具** — 限制或扩展 AI 在此技能中的能力
- **使用什么模型** — 可选地为该技能指定不同的 AI 模型

例如，你可以创建一个"代码审查"技能，它可以：

- 只能使用文件读取工具（不能写入）
- 使用高推理力度的模型进行深入分析
- 当你询问代码质量时自动激活

## 创建你的第一个技能

![技能页面](/ruri-pics/zh_hans/skills-cn.png)

让我们从头开始创建一个实用的技能。我们将制作一个 **"Summarize"（摘要）** 技能，它可以读取任何文件并给出清晰的摘要。

### 第 1 步：打开技能页面

在侧边栏中导航到 **技能**，然后点击 **添加技能**（或创建一个新技能）。

### 第 2 步：填写 Frontmatter

Frontmatter 是技能文件顶部的配置，位于 `---` 标记之间：

```markdown
---
name: "summarize"
description: "Summarize the content of any file or text"
when_to_use: "When the user asks for a summary of a file, document, or text"
argument_hint: "path to the file to summarize"
user_invocable: true
allowed_tools:
  - read_file
  - search_files
  - list_directory
---

You are a summarization expert. When given a file or text:

1. Read the content carefully
2. Identify the main topics and key points
3. Write a clear, concise summary in bullet points
4. Highlight any important details, decisions, or action items
5. Keep the summary under 200 words unless the user asks for more detail
```

### 第 3 步：保存并启用

1. 点击 **保存** 创建技能
2. 在技能页面上使用开关将技能 **开启**
3. 试试看！在聊天中询问："Summarize my README.md"

::: tip
`allowed_tools` 字段非常重要 — 只列出 `read_file`、`search_files` 和 `list_directory`，你就是在告诉 AI 它可以查看文件但**不能**修改它们。这让技能使用起来更加安全。
:::

---

### 另一个示例：翻译助手

你也可以创建符合自己需求的技能。以下是一个"翻译助手"的示例：

**名称**：`translator`

**描述**：`专业的中英翻译助手`

**技能内容**（Markdown 格式）：

```markdown
---
name: "translator"
description: "专业的中英翻译助手，自动检测语言并翻译"
when_to_use: "当用户需要翻译文本时"
user_invocable: true
---

你是一位专业的中英翻译专家。你的工作规则：

1. 自动检测输入文本的语言
2. 如果是中文，翻译成英文；如果是英文，翻译成中文
3. 翻译时保持原文的语气和风格
4. 对于专业术语，在翻译后用括号标注原文
5. 如果原文有歧义，给出多个翻译选项并说明

请直接给出翻译结果，不需要额外解释。
```

## 技能 Frontmatter 参考

以下是技能中可用的字段快速参考：

| 字段                       | 必填 | 说明                                                |
| -------------------------- | ---- | --------------------------------------------------- |
| `name`                     | ✅   | 技能的唯一名称                                      |
| `description`              | ✅   | 技能的功能描述（在 UI 中显示）                      |
| `when_to_use`              | ❌   | AI 应在何时自动使用此技能                           |
| `argument_hint`            | ❌   | 提示用户应提供什么参数                              |
| `arguments`                | ❌   | 定义技能接受的特定参数                              |
| `user_invocable`           | ❌   | 用户是否可以手动触发（默认：`true`）                |
| `disable_model_invocation` | ❌   | 禁止为此技能调用 AI 模型                            |
| `allowed_tools`            | ❌   | 此技能可以使用的工具列表（默认：全部）              |
| `model`                    | ❌   | 为此技能覆盖当前激活的模型                          |
| `effort`                   | ❌   | 推理力度：`low`、`medium` 或 `high`                 |
| `agent`                    | ❌   | 为此技能覆盖 Agent 配置                             |
| `context`                  | ❌   | 技能运行时包含的额外文件                            |
| `hooks`                    | ❌   | 在技能执行前或执行后运行 Shell 命令                 |
| `paths`                    | ❌   | 自动触发此技能的文件路径模式                        |
| `shell`                    | ❌   | 在技能执行前运行 Shell 命令并将其输出包含在上下文中 |

### Arguments（参数）

你可以定义技能期望接收的参数：

```markdown
arguments:

- name: "path"
  description: "File or directory path to review"
  required: true
```

### Disable Model Invocation（禁用模型调用）

当 `disable_model_invocation` 设置为 `true` 时，技能执行时不会调用 AI 模型。这对于只需要运行 Shell 命令或钩子而无需 AI 响应的技能非常有用：

```markdown
---
disable_model_invocation: true
shell: "cat {{path}}"
---
```

### Hooks（钩子）

钩子让你可以在技能执行的特定时间点运行 Shell 命令：

```markdown
---
hooks:
  pre: "echo 'Starting analysis...'"
  post: "echo 'Analysis complete!'"
---
```

- **pre** — 在技能执行前运行
- **post** — 在技能完成后运行

### Shell

运行一个 Shell 命令，并将其输出作为技能的上下文：

```markdown
---
shell: "git diff --stat"
---
```

命令输出会被捕获并包含在发送给 AI 的提示词中，为技能提供来自系统的实时上下文。

### Agent Override（Agent 覆盖）

为此技能自定义 Agent 行为：

```markdown
---
agent:
  max_tool_rounds: 10
  auto_execute_tools: true
---
```

### Effort Levels（推理力度级别）

`effort` 字段控制模型的思考深度：

- **`low`** — 快速、轻量级的响应
- **`medium`** — 平衡模式（默认）
- **`high`** — 深入、详细的分析（非常适合代码审查或复杂任务）

## 技能示例

### 代码审查技能

```markdown
---
name: "code-review"
description: "Review code changes and provide feedback"
when_to_use: "When the user asks for a code review"
allowed_tools:
  - read_file
  - search_files
  - list_directory
effort: "high"
---

You are a code reviewer. Analyze the code and provide:

1. A summary of what the code does
2. Potential bugs or issues
3. Suggestions for improvement
4. Code style observations
```

### 翻译技能

```markdown
---
name: "translate"
description: "Translate text between languages"
argument_hint: "text to translate and target language"
allowed_tools: []
---

You are a professional translator. Translate the given text naturally,
preserving the tone and meaning. If no target language is specified,
ask the user which language they want.
```

### 文档生成器

```markdown
---
name: "doc-writer"
description: "根据代码自动生成文档"
when_to_use: "当用户需要生成文档时"
allowed_tools:
  - read_file
  - write_file
  - create_file
  - list_directory
---

你是一位技术文档专家。根据给定的代码或项目，生成清晰、结构化的文档。
使用 Markdown 格式，包含代码示例和使用说明。
```

## 管理技能

### 通过 Web UI

1. 在侧边栏点击 **技能**
2. **浏览** 已安装的技能列表
3. 使用开关 **切换** 技能的启用 / 禁用状态
4. 手动 **添加** 新技能或上传技能包
5. 点击任意技能进行 **编辑**
6. **删除** 不再需要的技能

### 技能包

你可以将多个技能打包为 ZIP 文件以便分享：

1. 创建你的技能 Markdown 文件
2. 将它们放入一个文件夹
3. 将文件夹压缩为 ZIP 归档
4. 在技能页面上传 ZIP 文件

这对于与团队或跨不同 Ruri 实例分享技能集非常方便。

::: tip
创建技能包时，确保每个 Markdown 文件都有有效的 Frontmatter，至少包含 `name` 和 `description` 字段。
:::

### 技能激活

被禁用的技能会被保留，但不会对 AI 或用户可用。你还可以通过 [配置方案](/zh_hans/config-profiles) 控制每个方案中哪些技能处于活跃状态 — 例如，只在"开发"方案中启用编程相关技能。

## 小贴士

- **从简单开始** — 先创建一个基础技能，测试后再逐步增加复杂度
- **限制工具** — 只给技能分配它需要的工具。翻译技能不需要文件写入权限！
- **合理使用推理力度** — 分析类任务设置 `effort: high`，快速查询设置 `effort: low`
- **编写清晰的 `when_to_use`** — 这能帮助 AI 知道何时自动调用你的技能
- **持续迭代** — 测试你的技能，根据结果不断优化提示词
