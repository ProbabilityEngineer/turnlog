---
id: atr-1opw
status: open
deps: []
links: []
created: 2026-05-22T15:45:34Z
type: feature
priority: 1
assignee: ProbabilityEngineer
parent: atr-rs5y
---
# Implement typed record model

Define strongly typed Session, Turn, Event, and VcsInfo models with serde support.

## Acceptance Criteria

Models serialize to expected JSON; schema_version and timestamps are present; VcsInfo supports jj, git, and none.

