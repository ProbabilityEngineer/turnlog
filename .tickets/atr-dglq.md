---
id: atr-dglq
status: closed
deps: []
links: []
created: 2026-05-22T16:57:13Z
type: feature
priority: 1
assignee: ProbabilityEngineer
---
# Add diff attachments

Allow atrace record to attach the current jj/git diff to a turn.

## Acceptance Criteria

atrace record --attach-diff writes .atrace/attachments/<turn-id>.diff; jj repos use jj diff --git; Git repos use git diff; turn JSON records the attachment path; no-VCS dirs skip with a clear note.

