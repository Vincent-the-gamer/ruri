---
layout: doc
title: "Skills"
lastUpdated: true
---

# Skills

Skills let you teach Ruri new tricks. A skill is simply a Markdown file that tells the AI how to behave for a specific task — like reviewing code, writing documentation, or translating text.

## What Are Skills?

Think of skills as custom instructions you give the AI. Each skill defines:

- **What the AI should do** — The instructions written in Markdown
- **When to use it** — Triggers that tell the AI when this skill is relevant
- **Which tools it can use** — Limit or expand the AI's capabilities for this skill
- **What model to use** — Optionally use a different AI model for this skill

For example, you could create a "Code Review" skill that:

- Only uses file-reading tools (no writing)
- Uses a high-effort model for thorough analysis
- Automatically kicks in when you ask about code quality

## Creating Your First Skill

Let's walk through creating a practical skill from scratch. We'll make a **"Summarize"** skill that reads any file and gives you a clear summary.

### Step 1: Open the Skills Page

Navigate to **Skills** in the sidebar and click **Add Skill** (or create a new skill).

### Step 2: Fill in the Frontmatter

The frontmatter is the configuration at the top of your skill file, between the `---` markers:

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

### Step 3: Save and Activate

1. Click **Save** to create the skill
2. Toggle the skill **on** using the switch on the Skills page
3. Try it out! In a chat, ask: "Summarize my README.md"

::: tip
The `allowed_tools` field is important — by only listing `read_file`, `search_files`, and `list_directory`, you're telling the AI it can look at files but NOT modify them. This makes the skill safe to use.
:::

## Skill Frontmatter Reference

Here's a quick reference for the fields you can use in a skill:

| Field            | Required | What It Does                                          |
| ---------------- | -------- | ----------------------------------------------------- |
| `name`           | Yes      | Unique name for the skill                             |
| `description`    | Yes      | What the skill does (shown in the UI)                 |
| `when_to_use`    | No       | When the AI should automatically use this skill       |
| `argument_hint`  | No       | Hint for what arguments the user should provide       |
| `arguments`      | No       | Define specific arguments the skill accepts           |
| `user_invocable` | No       | Whether users can manually trigger it (default: true) |
| `allowed_tools`  | No       | List of tools this skill can use (default: all)       |
| `model`          | No       | Override the active model for this skill              |
| `effort`         | No       | Reasoning effort: `low`, `medium`, or `high`          |
| `context`        | No       | Extra files to include when the skill runs            |
| `hooks`          | No       | Run custom logic before or after the skill executes   |
| `paths`          | No       | File patterns that trigger this skill automatically   |

### Arguments

You can define arguments that the skill expects:

```markdown
arguments:

- name: "path"
  description: "File or directory path to review"
  required: true
```

### Effort Levels

The `effort` field controls how much thinking the model does:

- **`low`** — Quick, lightweight responses
- **`medium`** — Balanced (default)
- **`high`** — Thorough, detailed analysis (great for code reviews or complex tasks)

## Skill Examples

### Code Review Skill

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

### Translation Skill

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

## Managing Skills

### Via Web UI

1. Go to **Skills** in the sidebar
2. **Browse** your installed skills
3. **Toggle** skills on/off with the switch
4. **Add** new skills manually or upload a skill package
5. **Edit** any skill by clicking on it
6. **Delete** skills you no longer need

### Skill Packages

You can package multiple skills as a ZIP file for easy sharing:

1. Create your skill Markdown files
2. Place them in a folder
3. Compress the folder into a ZIP archive
4. Upload the ZIP on the Skills page

This is great for sharing skill sets with your team or across different Ruri instances.

::: tip
When creating skill packages, make sure each Markdown file has valid frontmatter with at least `name` and `description` fields.
:::

### Skill Activation

Deactivated skills are preserved but won't be available to the AI or the user. You can also control which skills are active per [Config Profile](/config-profiles) — for example, enable coding skills only in your "Development" profile.

## Tips

- **Start simple** — Create a basic skill first, test it, then add complexity
- **Limit tools** — Only give a skill the tools it needs. A translation skill doesn't need file write access!
- **Use effort wisely** — Set `effort: high` for analysis tasks, `effort: low` for quick lookups
- **Write clear `when_to_use`** — This helps the AI know when to automatically invoke your skill
- **Iterate** — Test your skills and refine the prompts based on the results
