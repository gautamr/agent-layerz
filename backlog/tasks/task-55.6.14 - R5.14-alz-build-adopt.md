---
id: TASK-55.6.14
title: 'R5.14: alz build --adopt'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T05:19:00Z'
labels:
  - area:rust
  - area:adopt
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.6.13
  - TASK-55.6.8
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 146390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§4.3, §10).
- RepoRoot applies an accepted AdoptionPlan: layer files written create-only, moves as renameat inside the repository, removals, empty folders pruned, then the manifest edit as owner question 2 decides (keeping its comments and layout).
- build --adopt: notes `note [adopt]`, problems `error [adopt]` and the line that nothing was adopted, exit 1 with the repository unchanged; else the adopted lines, a second pipeline run from the manifest, and any collision left is an error.
- The repo layer must be a path: entry inside the repository and last in the stack, else the error adopt.
- Setting the local git identity on --adopt is R6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 20 passes against ALZ_BIN: the adopted repository builds the same settings and agents, the manifest keeps its comments, and check passes (vitest run)
- [x] #2 When any file cannot be adopted the repository tree is byte-identical before and after (test 20 against ALZ_BIN and an alz-io test)
- [x] #3 Nothing adoption writes, moves or deletes lies outside the repository or goes through a link (alz-io tests)
- [x] #4 Moved from R5.13: a property test over generated native settings adopts them, builds, and finds the rendered native files equal to the adopted ones
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.8 (2026-11-24): the build --adopt rows left are test 14 (1 row), test 23 (2 rows) and test 24 (2 rows: a lock outside the repository, a lock sha).

Lead decisions from R5.13 (2026-11-24): (1) an agent with native claude.hooks (fixture 20's architect) is refused for adoption with a problem naming the agent and the field (moving agent-scoped hooks into hooks/hooks.yaml would make them run everywhere); test 20's success case becomes referenceDrift('TASK-55.6.13', ...), and a twin fixture without native hooks shows the success path against the Rust binary. (2) Adoption notes print at Level::Note (R5.9 added it), not as warnings. (3) AdoptionInput.natives holds the collisions and adopt-always files alz did not generate (the planner does not know ownership); agents_md is the on-disk AGENTS.md bytes. (4) R5.13's AC #3 (property over generated native settings through the renderer) is done here, where render is wired: adopt, then build, then the rendered native files equal the adopted ones for generated inputs.

R5.14 implemented (2026-11-24). Evidence per AC (nothing checked off; the lead checks).
AC1: acceptance 14 (adopt row), 23 (2 rows) and 24 (the two build --adopt rows) pass against the release binary; test 20: the success case now runs on a twin fixture without native agent hooks (test/fixtures/20-adopt-native-plain) and passes against both binaries (reference and Rust); referenceDrift(TASK-55.6.13) covers the refusal of fixture 20 (architect has claude.hooks), referenceDrift(TASK-55.6.14) covers hook scripts moving to .layer/hooks/scripts/ with run: scripts/<path> (spec §4.3), and the secret-env case keeps passing with a wording-neutral assertion (the Rust message quotes the variable). The two lock rows of test 24 became referenceDrift(TASK-55.6.15): the Rust lock messages follow the spec (the lock's <field> is missing or not valid), the reference prints 'unsafe entry'. check passes after adoption in the success tests. Results: ALZ_BIN=release: files 14, 20, 23 pass; 24 has 16 failures, none of them adopt (mcpServers frontmatter, JSON depth, account, control character); the whole acceptance suite: 737 passed, 286 failed (account which/allow, R6 identity, R5.11, R5.15 parity).
AC2: a refused plan writes nothing: acceptance 20 (secret fixture, readTree before/after), crates/alz/tests/adopt.rs (a_plan_with_a_problem_writes_nothing_and_prints_every_problem_stripped, a_manifest_without_a_repo_layer..., a_link_is_never_followed...), crates/alz-io/tests/repo_adopt.rs (a changed native file, a link, a layer file that appeared: tree equal before and after).
AC3: crates/alz-io/tests/repo_adopt.rs: native file replaced by a link, a layer folder that became a link (nothing written outside); RepoRoot::adopt checks every path with no-follow probes before the first write, moves with linkat then unlinkat (never replaces), writes through folder handles opened with O_NOFOLLOW; crates/alz/tests/adopt.rs: link collisions stop adoption, a settings.json link the build does not write is never read.
AC4: crates/alz/tests/adopt.rs rendered_native_settings_equal_the_adopted_ones (proptest, 12 cases of generated env and allow/deny/ask lists; adopt, build, check; permissions and env of the rendered .claude/settings.json equal the adopted ones).
Design: alz-core adopt/natives.rs picks the natives (NotGenerated collisions that are regular files, plus .claude/settings.json and .mcp.json when alz did not generate them; other collisions are unadoptable and reported as collision errors before planning); AdoptionPlan notes now Level::Note; alz-io RepoRoot::adopt(&AdoptionPlan, &AdoptionBasis); alz command/build/adopt.rs; Computed gained stack, placements and manifest_text. Spec §4.3 gained two bullets (unadoptable collisions, the apply rule). Open: the Rust problem text says 'Write the variable as a reference to itself and export it' where the spec lists 'Use "${NAME}" and export it' (R5.13 wording); setting the git identity on --adopt is R6.

Verified by the lead (2026-11-24): merged into the R5 branch (51bb23c); mise run rust:ci and mise run lint exit 0. Test 24's two lock rows assert the spec's wording as reference drift owned by R5.15.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz build --adopt picks the native files alz did not generate, plans them with adopt::plan, checks every path with no-follow reads before writing anything, applies layer files, moves (link then unlink, never replacing), removals and the layer.yaml edit through RepoRoot, then builds; a refused plan leaves the repository byte-identical.
<!-- SECTION:FINAL_SUMMARY:END -->
