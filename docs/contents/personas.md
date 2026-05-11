---
layout: doc
title: "Personas"
lastUpdated: true
---

# Persona System

Personas define the personality and behavior of the AI assistant. Each persona has a name, description, and a system prompt that shapes how the AI responds to messages.

## Overview

With the persona system, you can create multiple AI personas for different use cases:

- A **coding assistant** persona focused on technical accuracy
- A **creative writer** persona with a more expressive tone
- A **tutor** persona that explains concepts step-by-step
- A **concise responder** persona that gives brief, to-the-point answers

## Persona Structure

Each persona consists of:

| Field         | Type   | Description                                      |
| ------------- | ------ | ------------------------------------------------ |
| `name`        | string | A unique name for the persona                    |
| `description` | string | A short description of the persona's style/role  |
| `prompt`      | string | The system prompt that defines the persona's behavior |

**Example persona:**

```yaml
name: "Code Expert"
description: "A senior software engineer who provides precise, well-documented code solutions"
prompt: |
  You are a senior software engineer with 20 years of experience.
  When writing code, you always:
  - Include proper error handling
  - Add clear comments and documentation
  - Follow language-specific best practices and conventions
  - Suggest tests for critical logic
  Keep explanations concise and focused on code quality.
```

## Managing Personas

### Via Web UI

1. Navigate to the **Personas** page in the sidebar
2. View the list of existing personas
3. Create new personas by filling in the name, description, and prompt
4. Edit existing personas to refine their behavior
5. Delete personas you no longer need
6. Set a persona as active for the current session

### Active Persona

The active persona is applied to all conversations. You can change the active persona at any time:

- Through the Web UI
- Through [Config Profiles](/config-profiles) — each profile can specify a default persona
- Through the [command system](/commands) — use `/set` to change the persona for the current session

## Persona and Skills

Personas and skills work together to provide a complete AI behavior:

- The **persona** defines the general tone and behavior of the AI
- The **skill** defines a specific task or behavior with tool access control

When a skill specifies a `model` or uses its own prompt, it may override the active persona for that specific skill execution. Otherwise, the active persona is used as the base system prompt.

## Tips for Writing Effective Personas

1. **Be specific** — Clearly define the persona's role, expertise, and communication style
2. **Set boundaries** — Specify what the persona should and shouldn't do
3. **Include examples** — Show the desired response format in the prompt
4. **Keep it focused** — Avoid conflicting instructions in the same persona
5. **Test and iterate** — Refine your persona based on the AI's actual responses

::: tip
You can create multiple personas for different contexts and switch between them using [Config Profiles](/config-profiles). This lets you quickly adapt the AI's behavior without rewriting prompts.
:::
