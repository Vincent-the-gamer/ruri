---
layout: doc
title: "Personas"
lastUpdated: true
---

# Personas

Personas let you change the AI's personality. Want a formal coding assistant? A friendly chat buddy? A patient tutor? Just create a persona and switch to it.

## What Is a Persona?

A persona is a set of instructions that tells the AI how to communicate. It includes:

- **Name** — To identify the persona
- **Description** — A quick summary of what this persona is like
- **System Prompt** — The actual instructions that shape the AI's behavior

You can have multiple personas and switch between them anytime — the AI's personality changes instantly.

## Creating Your First Persona

Let's create a **"Study Buddy"** persona — a patient tutor that explains things step by step.

### Step 1: Open the Personas Page

Navigate to **Personas** in the sidebar and click **Add Persona**.

### Step 2: Fill in the Details

- **Name:** `Study Buddy`
- **Description:** `A patient tutor who explains concepts step by step`

### Step 3: Write the System Prompt

The system prompt is where the magic happens. Here's an example:

```
You are a patient and encouraging tutor. When explaining concepts:

1. Start with a simple, high-level overview
2. Break complex topics into small, digestible steps
3. Use analogies and real-world examples
4. Check understanding before moving on
5. Never make the user feel silly for asking questions

Keep your explanations clear and jargon-free unless the topic
requires technical terms. When you use a technical term,
always define it first.
```

### Step 4: Save and Activate

Click **Save**, then set this persona as active. Try asking the AI to explain something — you'll notice a much more patient, educational tone!

::: tip
Experiment with your prompts! Small changes in wording can significantly affect the AI's behavior. Try adding specific instructions like "always use bullet points" or "keep responses under 100 words."
:::

## Persona Examples

### Code Expert

```
Name: Code Expert
Description: A senior software engineer who writes clean, well-documented code

You are a senior software engineer with 20 years of experience.
When writing code, you always:
- Include proper error handling
- Add clear comments and documentation
- Follow language-specific best practices
- Suggest tests for critical logic
Keep explanations concise and focused on code quality.
```

### Casual Chat

```
Name: Casual Chat
Description: A friendly, relaxed conversationalist

You are a friendly and casual chat buddy. Talk like a friend,
not an assistant. Use informal language, be warm and approachable.
Feel free to share opinions and be a bit playful. Keep things
light but still helpful.
```

### Translator

```
Name: Translator
Description: Professional multilingual translator

You are a professional translator. When given text, translate it
naturally while preserving the original tone and meaning. If the
target language isn't specified, ask which language to translate to.
Always provide the translation first, then optionally explain
any interesting linguistic choices.
```

### Concise Responder

```
Name: Concise Responder
Description: Gives brief, to-the-point answers

Be extremely concise. Give the shortest accurate answer possible.
No pleasantries, no filler. If a yes/no question, answer yes or no.
Use bullet points for lists. Skip explanations unless asked.
```

## Managing Personas

### Via Web UI

1. Go to **Personas** in the sidebar
2. **Browse** your personas
3. **Create** a new one with the Add button
4. **Edit** any persona by clicking on it
5. **Delete** personas you no longer need
6. **Set active** — Click to make a persona the current one

### Switching Personas

You can switch your active persona at any time:

- **Web UI** — Click a persona to activate it
- **Config Profiles** — Each [Config Profile](/config-profiles) can specify a default persona
- **Chat command** — Use `/set persona "Study Buddy"` in the chat to switch for the current session

## Personas and Skills

Personas and skills work together:

- The **persona** sets the AI's general personality and communication style
- The **skill** defines a specific task with its own instructions and tool access

When a skill has its own prompt, it may override the persona for that specific task. Otherwise, the active persona is used as the base personality.

## Tips for Writing Great Personas

1. **Be specific** — "You are a senior Python developer" works better than "You are a coder"
2. **Set boundaries** — Tell the AI what it should and shouldn't do
3. **Show, don't tell** — Include examples of the desired response style
4. **Keep it focused** — Don't put conflicting instructions in one persona
5. **Test and refine** — Try your persona, see how it responds, then tweak the prompt

::: tip
Create multiple personas for different moods and tasks, then use [Config Profiles](/config-profiles) to quickly switch between them!
:::
