---
id: TASK-55.6.6
title: 'R5.6: Passthrough, protected paths and the Rendered build'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
updated_date: '2026-11-24T03:03:00Z'
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
- [x] #1 A passthrough path equal to a generated path under ASCII case folding is the error passthrough and the generated file is kept (unit test with .claude/Settings.json)
- [x] #2 A file a layer ships under a protected path, in any ASCII case, is dropped with the warning protected and is absent from the lock's files (unit tests)
- [x] #3 render and lock_of are deterministic and the lock's files map lists exactly the tree's paths with their sha256 (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.3 (2026-11-24): the Rendered build must add AgentsMd::budget_problems(manifest budgets) to the build diagnostics, and when AgentsMd::compose or claude_md returns errors the build writes nothing.

From R5.5 (2026-11-24): wire ClaudeSettingsFiles::of(&Merged, manifest.workspace()) into Rendered beside ClaudeFiles; their paths do not overlap (.claude/settings.json, .claude/hooks/**, .mcp.json).

R5.6 implementation (2026-11-24, uncommitted). Files: crates/alz-core/src/render/rendered.rs (Rendered, render), render/passthrough.rs (join), render/rendered/tests.rs; Code::Target added in diagnostic.rs; PassthroughFile gained is_executable (item/passthrough.rs) so the executable bit survives (spec §8.1); claude::tests and stacks made pub(crate) so the fixture layers are reused.
render(&Merged, &Manifest) -> Rendered {tree, diagnostics, managed_folders, protected}; no public constructor. Order: AGENTS.md (AgentsMd::compose + budget_problems(manifest.budgets())), then per target: claude = CLAUDE.md + ClaudeFiles + ClaudeSettingsFiles::of(merged, manifest.workspace()) (collision error between sources), codex = error target at layer.yaml; then passthrough.join; then Protected::drop_from. Rendered::to_write() is None when any error was found (a build with an error writes nothing); Rendered::lock(&ResolvedStack) calls lock_of. Compose or claude_md failure gives no AGENTS.md or CLAUDE.md plus the error layer.
AC1: a_passthrough_path_equal_to_a_generated_one_after_case_folding_is_the_error_passthrough (.claude/Settings.json vs generated .claude/settings.json: error [passthrough] shop-api:targets/claude/.claude/Settings.json, generated file kept) and the exact-case twin; two passthrough files folding together are error collision, first kept. AC2: a_file_under_a_protected_path_in_any_ascii_case_is_dropped_with_the_warning_and_not_locked (6 casings, 6 protected warnings, lock files exactly AGENTS.md, CLAUDE.md, kept/ok.md); fixture_15 whole tree = AGENTS.md, CLAUDE.md, 7 warnings, no backlog or notes in the lock. AC3: proptest render_is_deterministic_never_folds_two_paths_together_and_locks_exactly_its_tree (Rendered == Rendered, equal lock text, no two folded-equal paths, protected never covers a tree path, lock files == tree paths).
Whole-tree fixtures: 01-determinism (AGENTS.md, CLAUDE.md, settings.json, .mcp.json, hook script Executable, agents and skills; all paths, bytes, modes against the goldens), 02-instructions (expected/AGENTS.md, CLAUDE.md), 03-agent-overlay (expected agent byte for byte; agents and skills paths against GOLDEN_03; no whole-tree golden exists for its AGENTS.md, CLAUDE.md or settings.json), 15-protected-paths (paths and lock). Only 02 and 03 have an expected/ folder in test/fixtures. Also tested: target codex error, budget error, '-->' in a header source makes no AGENTS.md and an error, to_write None on errors.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, 1146 alz-core lib tests plus the rest, doc, msrv, deny, audit); mise run lint ok. Not run: acceptance suite (no Rust binary yet; R5.8). Open: the writer (R5.8) must call to_write() and pass managed_folders() and protected() to plan_writes; the plan's 'protected' field name is the Protected patterns.

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged. Whole-tree goldens for fixtures 03 and 15 (and the rest) come with R5.8's golden trees.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
render(&Merged, &Manifest) -> Rendered assembles AGENTS.md, CLAUDE.md, the Claude files and settings, and passthrough files into one FileTree; a passthrough path that folds onto a generated one is error passthrough with the generated file kept; protected paths are dropped with one warning each and kept out of the lock; codex is error target; to_write() is None when any error exists.
<!-- SECTION:FINAL_SUMMARY:END -->
