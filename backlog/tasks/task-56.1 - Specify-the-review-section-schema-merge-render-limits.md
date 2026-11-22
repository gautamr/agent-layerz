---
id: TASK-56.1
title: Specify the review section (schema, merge, render, limits)
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
labels:
  - area:spec
  - area:process
milestone: m-3
dependencies: []
parent_task_id: TASK-56
priority: high
ordinal: 43390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Design before code. Add a LAYERING-SPEC section for review, next to models and settings, marked planned with task IDs:
- Schema: unit (subtask | task | parent | milestone, with what each means on a Backlog.md board: a parent is a task that has subtasks; milestone means the last open task of a milestone); reviewers (agent name, when: always, or labels and touches conditions; locked); gate (done | commit). Where it lives: a review: key in layer.yaml or a workflow.yaml file; choose one and say why.
- Merge: the most specific layer wins for unit and gate; reviewers combine across layers by agent name; a locked reviewer or condition cannot be removed or narrowed by a later layer (same floor as locked hooks and deny rules, §7.5, §7.6); a later layer may add reviewers or widen conditions.
- Default when no layer declares review: today's behaviour (unit: task, reviewer always, security-reviewer on the security conditions), so nothing changes for existing repositories.
- Rendering: a generated Review section in AGENTS.md and a slot filled in the next-task skill (§ slots); alz explain review shows each layer's contribution.
- Lint: unknown unit or gate, a reviewer no layer defines as an agent, an empty when, a label condition not in the board's label families.
- Limits: alz cannot force an agent to run a review; enforcement is optional (separate task).
- Record the design as a decision with its body filled (standing exception).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LAYERING-SPEC has a review section covering schema, location, default, merge rules with the locked floor, rendering, lint and limits, all marked planned with task IDs
- [ ] #2 A decision records the design and the chosen file location, with its body filled
- [ ] #3 The spec states how each unit maps onto Backlog.md structure (parent and subtasks, milestones, labels), checked against the installed Backlog.md version and its CLI output
<!-- AC:END -->
