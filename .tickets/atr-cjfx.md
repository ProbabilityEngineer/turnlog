---
id: atr-cjfx
status: closed
deps: []
links: []
created: 2026-05-22T17:02:05Z
type: feature
priority: 1
assignee: ProbabilityEngineer
---
# Improve atrace status

Make atrace status a useful current-context summary for humans and agents.

## Acceptance Criteria

Status shows initialized/not initialized, current session, latest session if different, last turn, VCS kind/IDs, dirty state, and changed files; works in jj, Git-only, and non-VCS dirs.

