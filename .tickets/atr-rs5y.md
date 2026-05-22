---
id: atr-rs5y
status: closed
deps: []
links: []
created: 2026-05-22T15:45:21Z
type: epic
priority: 1
assignee: ProbabilityEngineer
---
# Design atrace MVP

Define atrace as a Rust CLI provenance recorder linking tickets, sessions, turns, and jj/git state.

## Design

See DESIGN.md. CLI-first, JSONL canonical storage, jj-aware with Git fallback, no daemon, not a VCS.

## Acceptance Criteria

DESIGN.md exists and describes purpose, storage, commands, VCS detection, record model, invariants, non-goals, and implementation plan.

