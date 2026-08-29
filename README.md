# turnlog

A lightweight, Git-compatible agent trace recorder.

`turnlog` records agent sessions and turns, then links them to tickets and the current VCS state. If `jj` is available in a colocated repo, it records `jj` change/operation metadata; otherwise it falls back to Git metadata.

## Goal

Connect:

```text
ticket → agent session → agent turn → jj change or git commit
```

without becoming a VCS.

## Install

From crates.io, after publication:

```bash
cargo install turnlog
```

From source:

```bash
cargo build --release
```

## Use

```bash
turnlog init
turnlog start --ticket AUTH-123 --goal "Fix auth token validation"
turnlog current
turnlog use <session-id>
turnlog record --model claude-sonnet-4-5 --summary "Updated token validation" --verification "cargo test auth"
turnlog record --attach-diff --summary "Record with patch snapshot"
turnlog record --session <session-id> --summary "Explicit session record"
turnlog status
turnlog --cwd /path/to/repo status
turnlog -C /path/to/repo record --summary "Record a different repo"
turnlog log
turnlog log --ticket AUTH-123
turnlog log --session <session-id>
turnlog log --changed src/auth.rs
turnlog log --grep validation
turnlog show <session-or-turn-id>
turnlog show <session-or-turn-id> --json
turnlog report <session-id>
turnlog report <session-id> --stdout
turnlog repair
turnlog grep "cargo test"
```

## Storage

`turnlog --cwd DIR` (or `-C DIR`) runs against `DIR` instead of the process current directory, which is useful when an agent process is launched from one directory while editing another repository.

`turnlog init` adds `.turnlog/` to `.gitignore` when missing, keeping local provenance out of GitHub by default. Treat turnlog as local-only provenance unless a repo explicitly opts into sharing it. If shared provenance is needed, commit a curated report or summary intentionally.

`turnlog` writes human-reviewable files under `.turnlog/`:

```text
.turnlog/
  index.jsonl
  sessions/
  turns/
  attachments/
```

Session and turn JSON files are canonical. `index.jsonl` is a rebuildable derived index, protected by a repository lock. Read commands reconcile valid index entries with canonical records, using canonical content and deterministic timestamp/ID ordering; malformed, stale, missing, or disagreeing index entries produce an incomplete-results warning. Write commands refuse to append until repair. If the index is damaged or a crash leaves canonical records unindexed, run `turnlog repair`; the existing index is backed up and rebuilt from canonical files. Mutations use an OS advisory lock, which is released automatically when a process exits; the persistent `.lock` file itself is harmless.
