# atrace Design

## Purpose

`atrace` is a lightweight CLI for recording agent work provenance and linking it to task and VCS state.

It connects:

```text
ticket → agent session → agent turn → jj change or git commit
```

`atrace` is not a version control system. It does not commit, rebase, merge, push, move bookmarks, or replace Git/Jujutsu.

## Product shape

- Rust CLI first.
- Passive recorder: explicit commands write trace records.
- Git-compatible metadata directory in the repository.
- Jujutsu-aware when `jj` is installed and the current repo is a jj repo.
- Git fallback when jj is unavailable.
- Pi extension can be added later as a thin wrapper around the CLI/core library.
- No daemon in v1.

## Storage

Canonical storage is append-only JSONL plus small JSON documents and Markdown reports.

```text
.atrace/
  index.jsonl
  sessions/
    <session-id>.json
    <session-id>.md
  turns/
    <turn-id>.json
    <turn-id>.md
  attachments/
    <turn-id>.diff
```

`index.jsonl` is the source of truth for event history. Per-session/turn JSON files are convenient snapshots. Markdown files are human-readable reports. SQLite may be added later as a derived cache, not canonical storage.

## Core commands

```bash
atrace init
atrace start --ticket AUTH-123 --goal "Fix auth token validation"
atrace record --model claude-sonnet-4-5 --verification "cargo test auth"
atrace status
atrace log
atrace show <session-or-turn>
```

## VCS detection

Detection order:

1. If `jj` exists and `jj root` succeeds: record jj metadata and Git metadata where available.
2. Else if `git rev-parse --show-toplevel` succeeds: record Git metadata.
3. Else: record trace metadata with `vcs.kind = "none"`.

## VCS metadata

With jj:

```json
{
  "kind": "jj",
  "jj_change": "kmqzvrlp",
  "jj_commit": "def789",
  "jj_operation": "abcxyz",
  "git_head": "abc123",
  "git_branch": "main",
  "dirty": true,
  "changed_files": ["src/auth.rs"]
}
```

Without jj:

```json
{
  "kind": "git",
  "git_head": "abc123",
  "git_branch": "main",
  "dirty": true,
  "changed_files": ["src/auth.rs"]
}
```

Outside VCS:

```json
{
  "kind": "none"
}
```

## Record model

Session:

```json
{
  "id": "sess_01HX...",
  "ticket": "AUTH-123",
  "goal": "Fix auth token validation",
  "created_at": "2026-05-22T12:00:00Z",
  "repo_root": "/repo",
  "vcs_start": {}
}
```

Turn:

```json
{
  "id": "turn_01HX...",
  "session": "sess_01HX...",
  "created_at": "2026-05-22T12:05:00Z",
  "model": "claude-sonnet-4-5",
  "summary": "Updated token validation and added tests",
  "verification": ["cargo test auth"],
  "vcs": {}
}
```

## Invariants

- `atrace init` is idempotent.
- Recording never mutates VCS history.
- Every event has an ID, timestamp, schema version, and event type.
- A turn references exactly one session.
- JSONL appends should be safe to review and easy to recover manually.
- The CLI must work in Git-only repos and non-VCS directories.

## Non-goals for v1

- Replacing Git or jj.
- Automatic filesystem watching.
- Daemon/background process.
- Token-level diff or merge.
- Remote sync protocol.
- Cryptographic identity/delegation.
- Knowledge graph or project memory vault.

## Implementation plan

Rust modules:

```text
src/
  main.rs       CLI entrypoint
  cli.rs        clap command definitions
  model.rs      typed records and VCS enums
  store.rs      .atrace read/write and markdown rendering
  vcs.rs        jj/git detection and metadata capture
  ids.rs        ID generation
```

Suggested dependencies:

- `clap` for CLI parsing
- `serde`, `serde_json` for records
- `time` for timestamps
- `ulid` for sortable IDs
- `anyhow` for CLI errors
- `camino` optional for UTF-8 paths
