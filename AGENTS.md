# AGENTS.md - Ruri

Agent entrypoint for this repository; Use this file for repo-wide rules only.

## Project Knowledge

Ruri is an AI agent.

- **Tech Stack**:
  - Ruri backend: Rust
  - Ruri frontend: Vue 3 + TypeScript
  - Docs: VitePress

- **File Structure**
  - `docs/`: VitePress documentation of this project.
  - `src/`: Backend source code of this project, written in Rust.
  - `webui/`: Frontend source code of this project, written in Vue 3 + TypeScript.

## Code Style

- Backend(Rust):
  - no #[allow(dead_code)]
  - no unused variables
  - no dead code

- Frontend(Vue + TypeScript):
  - no unused variables
  - no dead code
  - no `any` type
  - Vue Components: Always use Composition API and `<script setup lang="ts">`

## Forbidden operations

- CRITICAL: Do NOT run dangerous shell commands: e.g. `rm -rf`, `sudo`.

## Build Ruri

Ruri use rust-embed to embed static assets into the binary, you need to build frontend first.

```bash
# Build Frontend
pnpm -C webui run build
```

Then build the backend:

```bash
# Build Backend for development
cargo build

# Build Backend for production
cargo build --release
```
