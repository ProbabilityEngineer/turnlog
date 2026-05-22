# turnlog

A lightweight, Git-compatible agent trace recorder.

`turnlog` records agent sessions and turns, then links them to tickets and the current VCS state. If `jj` is available in a colocated repo, it records `jj` change/operation metadata; otherwise it falls back to Git metadata.

## Goal

Connect:

```text
ticket → agent session → agent turn → jj change or git commit
```

without becoming a VCS.

## Install / build

```bash
cargo build
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
turnlog log
turnlog log --ticket AUTH-123
turnlog log --session <session-id>
turnlog log --changed src/auth.rs
turnlog log --grep validation
turnlog show <session-or-turn-id>
turnlog show <session-or-turn-id> --json
turnlog report <session-id>
turnlog report <session-id> --stdout
turnlog grep "cargo test"
```

## Storage

`turnlog` writes human-reviewable files under `.turnlog/`:

```text
.turnlog/
  index.jsonl
  sessions/
  turns/
  attachments/
```

`index.jsonl` is the append-only canonical event log. JSON snapshots and Markdown reports are written for sessions and turns.
