---
id: TASK-30.1
title: Verify nested agent spawning and agent teams on the installed Claude Code
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-20T16:59:00Z'
labels:
  - area:spec
  - area:process
milestone: m-6
dependencies: []
documentation:
  - >-
    backlog/docs/doc-3 -
    Delivery-model-engineering-manager-sprints-triage-and-commands.md
parent_task_id: TASK-30
priority: high
ordinal: 42
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Whether an agent can dispatch other agents decides how the engineering manager works. Nesting changed several times in 2026 and the docs disagreed; some hubs set CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The default nesting depth, the effect of CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH and CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS, and the Agent tool requirement are verified with smoke tests and recorded in docs/target-facts.md
- [ ] #2 Whether agent teams exist and are enabled in the installed version is recorded
- [ ] #3 Uses the verify-target-fact skill
<!-- AC:END -->
