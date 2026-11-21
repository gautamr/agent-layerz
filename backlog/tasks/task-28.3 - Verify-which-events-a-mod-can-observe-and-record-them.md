---
id: TASK-28.3
title: Verify which events a mod can observe and record them
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:06:00Z'
labels:
  - area:lens
  - area:spec
  - uc:L1
  - uc:L2
  - uc:L3
  - uc:L4
  - uc:L5
milestone: m-5
dependencies:
  - TASK-28.2
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 90
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The design must rest on observed behaviour, not on the API types alone. Open questions: do subagents' tool calls reach the mod, is a skill preloaded into an agent visible, how often do usage measurements arrive, when does InstructionsLoaded fire for rules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each event the design uses (agent.spawn, SubagentStart and SubagentStop, skill.prompt, InstructionsLoaded, tool.call, session.measure, compaction and permission events) is verified with a smoke test and recorded in docs/target-facts.md with Claude Code version and status
- [ ] #2 Answers to the four open questions are recorded, including what cannot be observed
- [ ] #3 Uses the plugin-authoring and verify-target-fact skills
<!-- AC:END -->
