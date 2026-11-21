---
id: TASK-40
title: Stop memory from giving write tools to read-only agents
status: To Do
assignee: []
created_date: '2026-11-21T10:31:00Z'
labels:
  - area:security
  - area:spec
milestone: m-3
dependencies: []
priority: high
ordinal: 6390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Claude Code adds Write and Edit to any subagent with memory: project, so an agent whose neutral tools list has no write or edit tool can still change files. Seen 2026-11-21 on claude 2.1.323: the Agent tool listed Write and Edit for engineering-manager (tools Read, Grep, Glob, Bash, Skill, TodoWrite, Agent) and for architect-review, domain-review, design-review and security-reviewer (tools Read, Grep, Glob). The reviewers also run with permission_mode plan; engineering-manager does not. The neutral schema promises a tools list that the generated agent may silently exceed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The memory-adds-write-tools behaviour is verified with a smoke test and recorded in docs/target-facts.md (verify-target-fact), including whether permission_mode plan blocks those edits
- [ ] #2 alz warns or fails when an agent has memory but no write or edit tool, or the spec documents the behaviour and how to keep such an agent read-only
- [ ] #3 The personal layer's engineering-manager and read-only reviewers cannot edit files outside their memory folder, proven by a live test (for example a pre_tool hook keyed on agent_type, or memory removed)
- [ ] #4 Spec and README describe the chosen rule
<!-- AC:END -->
