---
id: TASK-18
title: Codex adapter
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-23T18:10:00Z'
labels:
  - area:adapter
milestone: m-3
dependencies: []
priority: high
ordinal: 300
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The spec promises Claude Code and Codex in v1 (section 8.2); today targets: [codex] fails as not implemented. Several Codex facts in docs/target-facts.md section 6 are UNVERIFIED, including project trust and custom agents. The architect splits this into subtasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every Codex behaviour the adapter relies on is verified and recorded in docs/target-facts.md
- [ ] #2 targets: [claude, codex] builds AGENTS.md, .agents/skills, .codex/config.toml (MCP servers, models) and custom agents where Codex supports them
- [ ] #3 Items Codex cannot express produce build warnings and never break Codex startup
- [ ] #4 Acceptance test 13 (two targets) passes
- [ ] #5 alz doctor checks the installed Codex version against the adapter's verified range
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From the R4 review fixes (2026-11-23, recorded by the architect while planning R5, TASK-55.6): the TOML emitter for Codex config must quote every key that holds a dot, so a YAML key written as one dotted string stays one literal TOML key and never addresses a nested table; add a test. The Codex adapter is not part of R5.
<!-- SECTION:NOTES:END -->
