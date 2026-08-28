---
id: tur-8oot
status: closed
deps: []
links: []
created: 2026-08-28T21:14:19Z
type: task
priority: 1
assignee: ProbabilityEngineer
---
# Test concurrent CLI record processes

Add an integration test launching many turnlog record subprocesses against one repository and validating the JSONL index has complete unique parseable events.

## Acceptance Criteria

Test uses independent CLI processes; every requested record appears exactly once; all JSONL lines parse.

