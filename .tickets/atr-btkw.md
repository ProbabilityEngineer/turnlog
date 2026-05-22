---
id: atr-btkw
status: closed
deps: [atr-1opw]
links: []
created: 2026-05-22T16:48:39Z
type: feature
priority: 1
assignee: ProbabilityEngineer
parent: atr-ysb8
---
# Store current session marker

Persist active session in .atrace/current-session.

## Acceptance Criteria

atrace start writes current-session; store can read/write/clear current session id.

