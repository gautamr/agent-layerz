---
id: TASK-55.5.5
title: 'R4.5: Agents, overlays and native fields'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:09:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.4
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 118390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.2). Tool narrowing is R4.6.
- item/agent.rs: Agent (kebab-case name equal to the file stem, description required, role, model, tools, skills, isolation, permission_mode, memory, targets); Overlay (extends equal to the file stem, OverlayMode Extend or Replace, `~` keeps the inherited value, ListEdit for skills and tools, allow_widen); NativeFields, an ordered map of TargetName to tree, which refuses a key that has a neutral field.
- merge/agents.rs: scalars Last; lists by ListEdit in stack order, deduplicated with order kept; extend appends one body section per layer under "## <layer kind> context: <layer name>"; replace replaces the body with an agent-replace warning; replace with no base defines a new agent; an overlay without a base is `unresolved`; two base definitions of one name are an error; native fields DeepMerge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 03's merge case has a Rust test with the same verdict: base, skills overlay and body overlay give one agent with the merged skills, three body sections and three provenance entries (unit test)
- [ ] #2 Test 18's cases have Rust tests with the same verdict: claude fields pass through and overlays deep-merge them, and a claude key with a neutral field is refused naming the key (unit tests)
- [ ] #3 Two base agents with one name, an overlay without a base and an extends that differs from the file name are errors, replace warns, and ListEdit application is associative (unit tests and proptest)
<!-- AC:END -->
