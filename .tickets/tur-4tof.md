---
id: tur-4tof
status: closed
deps: []
links: []
created: 2026-08-28T20:53:54Z
type: feature
priority: 1
assignee: ProbabilityEngineer
---
# Harden turnlog storage against concurrent index corruption

Implement repository-scoped locking, atomic canonical/index writes, repair/reindex, resilient JSONL parsing, diagnostics, and concurrency/crash-safety coverage per turnlog-plugin-improvements brief.

## Acceptance Criteria

Concurrent records cannot interleave; malformed indexes have actionable repair path; canonical files can rebuild index; tests cover concurrency and recovery; docs describe storage invariants.

