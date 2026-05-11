---
layout: doc
title: "Skills"
lastUpdated: true
---

# Skills

Skills are the primary mechanism for customizing Ruri's behavior. A skill is a Markdown file with YAML frontmatter that defines how the AI agent should behave in a particular context.

## Overview

Skills allow you to:

- Define specialized behaviors for specific tasks
- Control which tools the agent can use
- Set custom model parameters and personas per skill
- Create triggered behaviors with hooks
- Package and share skills as ZIP archives

## Skill File Format

Each skill is a Markdown file (`.md`) with YAML frontmatter. The frontmatter contains metadata and configuration, while the body contains the skill prompt or instructions.

```markdown
---
name: "code-review"
description: "Review code changes and provide feedback"
when_to_use: "When the user asks for a code review or when code changes are detected"
argument_hint: "path to the file or directory to review"
arguments:
  - name: "path"
    description: "File or directory path to review"
    required: true
user_invocable: true
allowed_tools:
  - read_file
  - search_files
  - list_directory
model: "claude-sonnet-4-20250514"
effort: "high"
---

You are a code reviewer. Analyze the code at the specified path and provide:
1. A summary of what the code does
2. Potential bugs or issues
3. Suggestions for improvement
4. Code style observations
```

## Frontmatter Fields

| Field                       | Type       | Default     | Description                                                        |
| --------------------------- | ---------- | ----------- | ------------------------------------------------------------------ |
| `name`                      | string     | required    | Unique identifier for the skill                                     |
| `description`               | string     | required    | Human-readable description of what the skill does                   |
| `when_to_use`               | string     | —           | Description of when this skill should be automatically invoked      |
| `argument_hint`             | string     | —           | Hint text describing the expected arguments                         |
| `arguments`                 | array      | `[]`        | List of argument definitions (see below)                            |
| `disable_model_invocation`  | boolean    | `false`     | If true, the model won't auto-invoke this skill                     |
| `user_invocable`            | boolean    | `true`      | Whether the user can manually trigger this skill                    |
| `allowed_tools`             | array      | all tools   | List of tool names this skill is allowed to use                     |
| `model`                     | string     | —           | Override the active model for this skill                            |
| `effort`                    | string     | —           | Model reasoning effort (e.g., `low`, `medium`, `high`)             |
| `context`                   | array      | —           | Additional context files to include when the skill runs             |
| `agent`                     | string     | —           | Agent configuration to use                                          |
| `hooks`                     | object     | —           | Lifecycle hooks (see below)                                         |
| `paths`                     | array      | —           | File path patterns that trigger this skill                          |
| `shell`                     | string     | —           | Shell command to run before or during skill execution               |

### Arguments

Each argument in the `arguments` array has the following structure:

| Field          | Type    | Required | Description                             |
| -------------- | ------- | -------- | --------------------------------------- |
| `name`         | string  | Yes      | Argument name                           |
| `description`  | string  | Yes      | Description of the argument             |
| `required`     | boolean | No       | Whether this argument is required       |

### Hooks

Hooks allow you to run custom logic at specific points in the skill lifecycle:

- **Pre-execution hooks**: Run before the skill prompt is sent to the model
- **Post-execution hooks**: Run after the model generates a response

## Managing Skills

### Via Web UI

1. Navigate to the **Skills** page in the sidebar
2. Browse the list of installed skills
3. Toggle skills on/off using the switch
4. Add new skills manually or by uploading a package

### Via API

**List all skills:**

```bash
curl http://localhost:3000/api/skills \
  -H "Cookie: session=<your-session-cookie>"
```

**Add a skill:**

```bash
curl -X POST http://localhost:3000/api/skills \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{
    "name": "my-skill",
    "content": "---\nname: my-skill\ndescription: My custom skill\n---\nDo something useful."
  }'
```

**Upload a skill package (ZIP):**

```bash
curl -X POST http://localhost:3000/api/skills/upload \
  -H "Cookie: session=<your-session-cookie>" \
  -F "file=@my-skills.zip"
```

**Toggle a skill on/off:**

```bash
curl -X PATCH http://localhost:3000/api/skills/my-skill \
  -H "Content-Type: application/json" \
  -H "Cookie: session=<your-session-cookie>" \
  -d '{"enabled": true}'
```

**Delete a skill:**

```bash
curl -X DELETE http://localhost:3000/api/skills/my-skill \
  -H "Cookie: session=<your-session-cookie>"
```

## Skill Packages

Skills can be packaged as ZIP archives for easy sharing and distribution. A skill package is a ZIP file containing one or more skill Markdown files.

**To create a skill package:**

1. Create your skill Markdown files
2. Place them in a directory
3. Compress the directory into a ZIP archive
4. Upload via the API or Web UI

::: tip
When creating skill packages, ensure each Markdown file has valid YAML frontmatter with at least `name` and `description` fields.
:::

## Skill Activation

Skills can be activated or deactivated through:

1. **Web UI toggle** — Use the switch on the Skills page
2. **API** — Use the `PATCH /api/skills/:name` endpoint
3. **Config Profiles** — Include specific skills in a profile's active skills list

When a skill is deactivated, it won't be available to the AI model or user, but its definition is preserved.
