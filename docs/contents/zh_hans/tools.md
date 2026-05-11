---
layout: doc
title: "内置工具"
lastUpdated: true
---

# 内置工具

Ruri 附带一组内置工具，AI 智能体可以使用这些工具与文件系统交互、执行命令和搜索网页。当模型支持工具调用时，这些工具会自动可用。

## 概览

| 工具             | 描述                                     |
| ---------------- | ---------------------------------------- |
| `read_file`      | 读取文件内容，支持指定行范围             |
| `write_file`     | 将内容写入文件                           |
| `create_file`    | 创建新文件                               |
| `edit_file`      | 对文件进行定向替换                       |
| `list_directory` | 列出文件和目录                           |
| `search_files`   | 按名称模式或内容搜索文件                 |
| `bash`           | 执行 Shell 命令（Computer Use 模式）     |
| `web_search`     | 搜索网络信息                             |

## 文件操作

### `read_file`

读取文件内容，可选择指定行范围。

**参数：**

| 参数         | 类型   | 必填 | 描述                          |
| ------------ | ------ | ---- | ----------------------------- |
| `path`       | string | 是   | 要读取的文件路径              |
| `start_line` | number | 否   | 起始行号（从 1 开始）        |
| `end_line`   | number | 否   | 结束行号（包含）              |

**智能体使用示例：**

智能体可能会读取配置文件来了解项目结构：

```
read_file(path="config.toml")
read_file(path="src/main.rs", start_line=1, end_line=50)
```

### `write_file`

将内容写入文件，覆盖任何现有内容。

**参数：**

| 参数      | 类型   | 必填 | 描述             |
| --------- | ------ | ---- | ---------------- |
| `path`    | string | 是   | 要写入的文件路径 |
| `content` | string | 是   | 要写入的内容     |

### `create_file`

使用指定内容创建新文件。此工具用于创建尚不存在的文件。

**参数：**

| 参数      | 类型   | 必填 | 描述             |
| --------- | ------ | ---- | ---------------- |
| `path`    | string | 是   | 要创建的文件路径 |
| `content` | string | 是   | 文件的初始内容   |

### `edit_file`

对现有文件进行定向替换。这对于进行精确编辑而无需重写整个文件非常有用。

**参数：**

| 参数       | 类型   | 必填 | 描述                   |
| ---------- | ------ | ---- | ---------------------- |
| `path`     | string | 是   | 要编辑的文件路径       |
| `old_text` | string | 是   | 要查找并替换的文本     |
| `new_text` | string | 是   | 替换后的文本           |

### `list_directory`

列出指定路径下的文件和目录。

**参数：**

| 参数   | 类型   | 必填 | 描述             |
| ------ | ------ | ---- | ---------------- |
| `path` | string | 是   | 要列出的目录路径 |

## 搜索工具

### `search_files`

按名称模式（glob）或按内容（正则表达式）搜索文件。

**参数：**

| 参数              | 类型   | 必填 | 描述                         |
| ----------------- | ------ | ---- | ---------------------------- |
| `pattern`         | string | 是   | 文件名匹配的 glob 模式      |
| `content_pattern` | string | 否   | 内容匹配的正则表达式模式    |
| `path`            | string | 否   | 搜索的基础目录               |

**使用示例：**

- 搜索所有 Rust 源文件：`search_files(pattern="**/*.rs")`
- 搜索包含特定函数的文件：`search_files(pattern="**/*.rs", content_pattern="fn main")`

## Shell 执行

### `bash`

在主机系统上执行 Shell 命令。此工具仅在 **Computer Use** 模式启用时可用。

**参数：**

| 参数      | 类型   | 必填 | 描述             |
| --------- | ------ | ---- | ---------------- |
| `command` | string | 是   | 要执行的 Shell 命令 |

::: warning
`bash` 工具功能强大，可以执行任意命令。仅在您信任环境和智能体指令的情况下启用 Computer Use 模式。建议使用沙盒模式以获得额外的安全性。
:::

更多关于运行时模式和安全功能的信息，请参阅 [Computer Use](/zh_hans/computer-use) 页面。

## 网页搜索

### `web_search`

使用可配置的搜索后端搜索网络信息。搜索结果使用 `scraper` 库从 HTML 中解析。

**参数：**

| 参数    | 类型   | 必填 | 描述         |
| ------- | ------ | ---- | ------------ |
| `query` | string | 是   | 搜索查询内容 |

::: info
网页搜索必须在[配置方案](/zh_hans/config-profiles)中启用，并且需要配置搜索提供商。
:::

## 工具可用性

工具根据当前配置对 AI 模型可用：

- **文件工具**（`read_file`、`write_file`、`create_file`、`edit_file`、`list_directory`、`search_files`）始终可用
- **`bash`** 仅在 Computer Use 启用时可用
- **`web_search`** 仅在活跃配置方案中启用网页搜索时可用
- **技能** 可以通过 `allowed_tools` 字段限制可用工具

有关如何控制特定技能可用工具的信息，请参阅[技能系统](/zh_hans/skills)。
