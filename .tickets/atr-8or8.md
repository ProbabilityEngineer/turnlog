---
id: atr-8or8
status: closed
deps: [atr-btkw]
links: []
created: 2026-05-22T16:48:50Z
type: feature
priority: 1
assignee: ProbabilityEngineer
parent: atr-ysb8
---
# Add explicit record --session

Allow record to target a specific session without changing ticket tooling.

## Acceptance Criteria

atrace record --session <id> records against that session; without --session it uses current-session; it no longer falls back silently to latest session.

