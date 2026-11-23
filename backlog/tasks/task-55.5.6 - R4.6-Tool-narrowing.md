---
id: TASK-55.5.6
title: 'R4.6: Tool narrowing'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.5
  - TASK-55.5.1
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 119390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.2 narrowing rule). ToolGrant { tool: NeutralTool, scope: Option<ToolScope> }, with NeutralTool the closed neutral vocabulary; an unknown tool is an `agent` error. `covers` follows the scope rule that R4.1 writes from owner question 6. After an overlay's tools edit every grant must be covered by a grant of the base, unless the overlay sets allow_widen and its layer is not public; allow_widen in a public layer is a `tool-widen` error. The error names the agent, the layer and the widened entry, stripped (decision-26).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 04's cases have Rust tests with the same verdict: an overlay that adds a tool fails naming the agent, the tool and the layer, and one that removes a tool or narrows a scope passes (unit tests)
- [ ] #2 Proptest: covers is reflexive and transitive, and without allow_widen in a non-public layer every merged grant is covered by a base grant
- [ ] #3 allow_widen in a public layer is refused, and in a private layer lets an overlay add a tool (unit tests)
<!-- AC:END -->
