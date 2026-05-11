---
layout: doc
title: "Config Profiles"
lastUpdated: true
---

# Config Profiles

Config Profiles let you save and switch between different setups with one click. Instead of manually changing settings every time, create a profile for each way you use Ruri and switch instantly.

## Why Use Profiles?

Imagine you use Ruri for coding at work, casual chatting at home, and research on weekends. Instead of changing 10 settings every time, you can:

- Create a **Coding** profile with a technical persona, file tools, and sandbox mode
- Create a **Casual** profile with a friendly persona and web search
- Create a **Research** profile with knowledge base access and web search
- Switch between them with one click!

## Creating a Profile

### Step 1: Open Profiles

Go to **Settings** or **Profiles** in the sidebar.

### Step 2: Create a New Profile

Click **Create Profile** and fill in the settings:

- **Name** — Something descriptive like "Coding" or "Research"
- **Provider** — Which AI model to use
- **Persona** — The AI's personality
- **Skills** — Which skills are active
- **Platforms** — Which chat platforms are connected
- **Knowledge Bases** — Which knowledge bases are searchable
- **Web Search** — On or off
- **Computer Use** — Off, Sandbox, or Local
- **ACP** — Whether the IDE server is running
- **Command Prefix** — The character for commands (default: `/`)

### Step 3: Save and Activate

Click **Save**, then switch to the profile by clicking **Activate**. All settings in the profile are applied immediately.

## Profile Examples

### Coding Profile

Perfect for software development:

| Setting        | Value                       |
| -------------- | --------------------------- |
| Provider       | Anthropic (Claude Sonnet 4) |
| Persona        | Code Expert                 |
| Web Search     | On                          |
| Computer Use   | Sandbox                     |
| Skills         | code-review, summarize      |
| Knowledge Base | Project Documentation       |

### Casual Chat Profile

For relaxed, friendly conversations:

| Setting      | Value           |
| ------------ | --------------- |
| Provider     | OpenAI (GPT-4o) |
| Persona      | Casual Chat     |
| Web Search   | On              |
| Computer Use | Off             |
| Skills       | translate       |

### Research Profile

For deep-dive research with document search:

| Setting         | Value                           |
| --------------- | ------------------------------- |
| Provider        | Anthropic (Claude Sonnet 4)     |
| Persona         | Concise Responder               |
| Web Search      | On                              |
| Computer Use    | Off                             |
| Skills          | summarize                       |
| Knowledge Bases | Research Papers, Technical Docs |

### Platform Profile

For chat platform integrations:

| Setting      | Value             |
| ------------ | ----------------- |
| Provider     | DeepSeek          |
| Persona      | Helpful Assistant |
| Web Search   | On                |
| Computer Use | Off               |
| Platforms    | Discord, DingTalk |

## Switching Profiles

When you switch profiles, everything changes at once:

- The AI model changes
- The persona is swapped
- Skills are activated or deactivated
- Platform connections update
- Knowledge base access adjusts

You can switch profiles anytime from the Web UI — it takes effect immediately.

## Managing Profiles

### Via Web UI

1. Go to **Settings** or **Profiles**
2. **View** all your profiles
3. **Create** new profiles for different use cases
4. **Edit** existing profiles to tweak settings
5. **Switch** between profiles instantly
6. **Delete** profiles you no longer need

## Tips

- **Start with a general profile** — Create a default profile that works for most things, then add specialized profiles
- **Use descriptive names** — "Coding" is better than "Profile 1"
- **Test new settings safely** — Create a test profile when experimenting, so your working setup stays intact
- **Keep profiles focused** — Each profile should serve a specific purpose
- **Share configurations** — If multiple people use the same Ruri instance, each person can have their own profile
