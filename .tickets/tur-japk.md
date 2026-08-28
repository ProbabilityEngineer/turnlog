---
id: tur-japk
status: closed
deps: []
links: []
created: 2026-08-28T21:28:06Z
type: task
priority: 1
assignee: ProbabilityEngineer
---
# Add crash-safety fault-injection tests for storage writes

Exercise failures before canonical atomic rename, after canonical write before index append, and before repair's atomic index replacement; verify valid data remains recoverable through repair.

## Acceptance Criteria

Tests cover canonical write failure, orphan creation after index append failure, and failed reindex replacement preserving prior index; cargo tests pass.

