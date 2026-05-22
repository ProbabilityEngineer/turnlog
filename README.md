# atrace

A lightweight, Git-compatible agent trace recorder.

`atrace` records agent sessions and turns, then links them to tickets and the current VCS state. If `jj` is available in a colocated repo, it records `jj` change/operation metadata; otherwise it falls back to Git metadata.

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
atrace init
atrace start --ticket AUTH-123 --goal "Fix auth token validation"
atrace record --model claude-sonnet-4-5 --summary "Updated token validation" --verification "cargo test auth"
atrace status
atrace log
atrace log --ticket AUTH-123
atrace log --session <session-id>
atrace log --changed src/auth.rs
atrace log --grep validation
atrace show <session-or-turn-id>
atrace show <session-or-turn-id> --json
atrace grep "cargo test"
```

## Storage

`atrace` writes human-reviewable files under `.atrace/`:

```text
.atrace/
  index.jsonl
  sessions/
  turns/
  attachments/
```

`index.jsonl` is the append-only canonical event log. JSON snapshots and Markdown reports are written for sessions and turns.
