---
id: atr-umkn
status: closed
deps: []
links: []
created: 2026-05-22T15:45:34Z
type: feature
priority: 1
assignee: ProbabilityEngineer
parent: atr-rs5y
---
# Implement jj/git VCS detection

Detect jj first, fall back to Git, and collect current change/head/branch/dirty/changed-file metadata.

## Acceptance Criteria

Works in jj colocated repos, Git-only repos, and non-VCS dirs; detection does not mutate VCS.

