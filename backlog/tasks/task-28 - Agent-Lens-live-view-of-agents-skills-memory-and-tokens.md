---
id: TASK-28
title: 'Agent Lens: live view of agents, skills, memory and tokens'
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T18:00:00Z'
labels:
  - area:lens
  - epic
milestone: m-5
dependencies: []
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
priority: high
ordinal: 50
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
People running multi-agent work in Claude Code cannot see which subagent, skill, memory file or how much context is in play without digging through transcripts and commands. A Claude Code mod can observe all of it and draw it beside the chat, and alz can add which layer each item comes from. Built by this project's agent team with custom and community skills; waves, team, skills, risks and use cases UC-L1 to UC-L7 are in doc-2.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Live pane shows the active subagent, skills, loaded memory files with their layer, context use, cost and recent tools (UC-L1 to UC-L5)
- [ ] #2 Compact view below 144 columns (UC-L6)
- [ ] #3 Plugin tests pass in CI and the security review has no open findings
- [ ] #4 Installs with one /plugin install line (UC-L7)
<!-- AC:END -->
