---
id: TASK-55.6.6
title: 'R5.6: Passthrough, protected paths and the Rendered build'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-24T02:19:00Z'
labels:
  - area:rust
  - area:render
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.3
  - TASK-55.6.4
  - TASK-55.6.5
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 138390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: the pure entry point `render(&Merged, &Manifest) -> Rendered` and `lock_of`.
- Order: AGENTS.md, then the Claude adapter (CLAUDE.md, agents, skills, rules, settings with hooks, .mcp.json), then passthrough, then the protected drop.
- Passthrough files of claude at their path; a path whose folded key a generated file already holds is the error passthrough naming the layer and file (case-folded, so targets/claude/.claude/Settings.json collides with .claude/settings.json).
- Protected paths (R5.2) dropped with the protected warning, after every adapter output is in the tree.
- A target without an adapter (codex) is the error target at layer.yaml, as the reference reports it.
- Rendered { tree, diagnostics, managed folders, protected } has no public constructor; lock_of(&Rendered, &ResolvedStack) gives the lock.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A passthrough path equal to a generated path under ASCII case folding is the error passthrough and the generated file is kept (unit test with .claude/Settings.json)
- [ ] #2 A file a layer ships under a protected path, in any ASCII case, is dropped with the warning protected and is absent from the lock's files (unit tests)
- [ ] #3 render and lock_of are deterministic and the lock's files map lists exactly the tree's paths with their sha256 (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.3 (2026-11-24): the Rendered build must add AgentsMd::budget_problems(manifest budgets) to the build diagnostics, and when AgentsMd::compose or claude_md returns errors the build writes nothing.

From R5.5 (2026-11-24): wire ClaudeSettingsFiles::of(&Merged, manifest.workspace()) into Rendered beside ClaudeFiles; their paths do not overlap (.claude/settings.json, .claude/hooks/**, .mcp.json).
<!-- SECTION:NOTES:END -->
