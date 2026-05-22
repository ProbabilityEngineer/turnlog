# Agent Instructions

## Project goal

Build `atrace`: a Rust CLI provenance recorder for agent sessions and turns.

`atrace` links:

```text
ticket → agent session → agent turn → jj change or git commit
```

It should remain small, understandable, and Git-compatible.

## Non-goals

- Do not replace Git or Jujutsu (`jj`).
- Do not implement a daemon in v1.
- Do not add SQLite/database storage as canonical storage in v1.
- Do not add graph UI, remotes, merge logic, branch/view semantics, or VCS behavior.
- Do not mutate VCS history from `atrace` commands.

## Architecture

Follow `DESIGN.md`.

Canonical storage is append-only JSONL plus JSON snapshots and Markdown reports:

```text
.atrace/
  index.jsonl
  sessions/
  turns/
  attachments/
```

`atrace` detects `jj` first and falls back to Git when `jj` is unavailable.

## Version control

Use `jj` for local work.

Useful commands:

```bash
jj status
jj diff
jj log
jj describe -m "message"
jj new
jj op log
jj undo
```

Avoid Git staged-index workflows unless explicitly needed for remote compatibility.
Use Git primarily for remote interoperability.

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
