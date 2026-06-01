# Agent Instructions

## Project goal

Build `turnlog`: a Rust CLI provenance recorder for agent sessions and turns.

Use turnlog as local-only provenance by default. Record meaningful repository work for continuity, validation, and handoff, even when `.turnlog/` is ignored. Do not assume turnlog records will be committed or pushed. If shared provenance is needed, create a curated report or summary and commit it intentionally. Do not record routine chat-only turns.

`turnlog` links:

```text
ticket → agent session → agent turn → jj change or git commit
```

It should remain small, understandable, and Git-compatible.

## Non-goals

- Do not replace Git or Jujutsu (`jj`).
- Do not implement a daemon in v1.
- Do not add SQLite/database storage as canonical storage in v1.
- Do not add graph UI, remotes, merge logic, branch/view semantics, or VCS behavior.
- Do not mutate VCS history from `turnlog` commands.

## Architecture

Follow `DESIGN.md`.

Canonical storage is append-only JSONL plus JSON snapshots and Markdown reports:

```text
.turnlog/
  index.jsonl
  sessions/
  turns/
  attachments/
```

`turnlog` detects `jj` first and falls back to Git when `jj` is unavailable.

## Jujutsu Version Control

- Use JJ for local work: `jj status`, `jj diff`, `jj log`, `jj describe -m "message"`, `jj new --no-edit`, `jj op log`, and `jj undo`.
- Do not use Git staged-index workflows: no `git add`, `git commit`, `git diff --cached`, or `git pull --rebase`.
- After completing coherent agent-owned work, run `jj describe -m "message"` and `jj new --no-edit`; `@` should be empty and `@-` should be the completed change.
- Before pushing, ensure the target bookmark/branch points to the completed change (`@-`), not the empty `@`; after push, `@-` should show `<branch> <branch>@origin`.
- Desired final publish shape: `@` is empty; `@-` is the completed change; `main`, `main@git`, and `main@origin` point to `@-`; Git HEAD is attached to `main`; `git status --short --branch` is clean and shows `## main...origin/main`.
- If `jj status` is dirty before you start, treat it as pre-existing user work unless explicitly told to continue it.
- For off-machine backup or publishing, prefer `/jj-align-push [branch]` after `@` is empty and `@-` is the completed change.

## Tasks

Use `tk` for task tracking.

Before starting work:

```bash
tk ready
tk start <id>
```

After finishing work:

```bash
tk close <id>
```

Keep tickets updated when scope or decisions change.

## Build and test

Use Cargo:

```bash
cargo fmt
cargo test
cargo build
```

Run relevant tests before closing implementation tickets.
