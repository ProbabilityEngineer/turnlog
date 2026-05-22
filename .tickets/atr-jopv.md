---
id: atr-jopv
status: open
deps: [atr-1opw]
links: []
created: 2026-05-22T15:45:34Z
type: feature
priority: 1
assignee: ProbabilityEngineer
parent: atr-rs5y
---
# Implement .atrace storage

Create idempotent atrace init and storage helpers for .atrace/index.jsonl, sessions, turns, and attachments.

## Acceptance Criteria

atrace init creates directory layout; rerunning init is safe; tests cover layout creation.

