---
id: tur-0xkt
status: closed
deps: []
links: []
created: 2026-08-28T21:14:14Z
type: bug
priority: 1
assignee: ProbabilityEngineer
---
# Replace expiring lock files with OS advisory locks

Replace timestamp-based stale-lock removal with an OS advisory lock so a live slow writer cannot have its lock stolen and locks release automatically on process exit.

## Acceptance Criteria

Cross-process mutations serialize; lock release after process exit requires no stale-file deletion; tests cover lock-file persistence/reacquisition.

