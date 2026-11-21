---
paths:
  - "test/**"
---
- Acceptance tests follow the add-acceptance-test skill: copied fixture, isolated `HOME`,
  assertions on exit code, message and files on disk.
- Never read or write the real `~/.claude*`, `~/.codex*` or `~/.cache` from a test.
- Fixtures are inputs: a test copies them first and never edits `test/fixtures/` in place.
