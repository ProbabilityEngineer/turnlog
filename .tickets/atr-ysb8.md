---
id: atr-ysb8
status: closed
deps: []
links: []
created: 2026-05-22T16:48:39Z
type: epic
priority: 1
assignee: ProbabilityEngineer
---
# Add active session support

Make atrace record turns against an explicit active session instead of implicitly using latest session.

## Acceptance Criteria

start sets current session; record can use current session or --session; current/use commands work; missing current sessions error clearly.

