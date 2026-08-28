---
id: tur-r6a1
status: closed
deps: []
links: []
created: 2026-08-28T20:58:23Z
type: feature
priority: 1
assignee: ProbabilityEngineer
---
# Complete storage recovery and concurrency hardening

Add graceful partial reads, canonical orphan detection, stale lock recovery, and dedicated concurrency/crash safety tests on top of initial repair implementation.

## Acceptance Criteria

Read commands continue with warnings after malformed JSONL; missing index events are detectable; stale locks recover safely; tests exercise concurrent writers and interrupted/repaired state.

