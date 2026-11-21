---
id: TASK-39
title: Record that hook input carries agent_type and agent_id for subagent tool calls
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-21T10:09:00Z'
labels:
  - area:spec
milestone: m-3
dependencies:
  - TASK-36
priority: medium
ordinal: 5390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verified 2026-11-21 with claude 2.1.323 during task-36: a PreToolUse hook on Write received agent_type=platform-engineer and an agent_id when a subagent made the call. The role path guard depends on it; adapters may only rely on facts in docs/target-facts.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/target-facts.md records the fact with version, method and source, using the verify-target-fact skill
<!-- AC:END -->
