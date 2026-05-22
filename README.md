# atrace

A lightweight, Git-compatible agent trace recorder.

`atrace` records agent sessions and turns, then links them to tickets and the current VCS state. If `jj` is available in a colocated repo, it records `jj` change/operation metadata; otherwise it falls back to Git metadata.

## Goal

Connect:

```text
ticket → agent session → agent turn → jj change or git commit
```

without becoming a VCS.
