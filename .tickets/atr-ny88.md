---
id: atr-ny88
status: closed
deps: []
links: []
created: 2026-05-22T17:05:05Z
type: feature
priority: 1
assignee: ProbabilityEngineer
---
# Add session reports

Generate durable Markdown review packets for atrace sessions.

## Acceptance Criteria

atrace report <session-id> writes .atrace/reports/<session-id>.md; --stdout prints instead; report includes child turns, verification, attachments, changed files, and VCS state; init creates reports dir; tests cover report rendering.

