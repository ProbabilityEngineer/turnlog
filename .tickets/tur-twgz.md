---
id: tur-twgz
status: closed
deps: []
links: []
created: 2026-08-29T14:48:36Z
type: feature
priority: 1
assignee: ProbabilityEngineer
---
# Make CLI read queries canonical-aware

Reconcile the rebuildable index with canonical session/turn JSON on read; report index/canonical drift without mutation; make writes require repair when the index is malformed or incomplete.

## Acceptance Criteria

status/log/grep/show/report use canonical-aware queries; warnings explain index drift; writes refuse malformed or orphaned indexes; tests cover malformed entries, orphan recovery, index/canonical disagreement, and deterministic ordering.

