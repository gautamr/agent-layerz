---
id: TASK-55.5.15
title: 'R4.15: alz lint runs the per-layer checks; R4''s acceptance cases and checks'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T13:38:00Z'
labels:
  - area:rust
  - area:lint
milestone: m-7
dependencies:
  - TASK-55.5.14
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 128390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step.
- alz lint, with --layer and in stack mode, runs every kind's per-layer reader on each layer alone and drops `unresolved`, as spec §10 and the reference do (owner answer 3 in TASK-55.5: no cross-layer merge in lint, no spec change; merge errors show in build, R5), plus the provides_slots and requires_slots checks.
- Acceptance: test/acceptance/37-settings-targets.test.ts with fixture test/fixtures/37-settings-targets and its §13 entry (R4.1): "a settings or MCP file whose top-level key is not a known target is refused, whatever the key", one row per key, through alz lint --layer and through alz build (no output written). Both groups use referenceDrift("TASK-55.5", ...); the build rows pass by R5.
- Run tests 02 to 11, 17 to 19, 26, 29 and 37 against ALZ_BIN and write into TASK-55.5's notes each merge case with the Rust test that gives its verdict (TASK-55.5 AC #1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint reports the per-layer errors of every item kind and no unresolved one, for one layer and for each layer of a stack alone (assert_cmd tests)
- [ ] #2 Test 37's lint rows pass against the Rust binary and are marked reference drift against the TypeScript reference (mise run test:acceptance, with and without ALZ_BIN)
- [ ] #3 Every case of tests 02 to 11, 17 to 19, 26, 29 and 37 that drives alz lint passes against ALZ_BIN, the 84 of the 2026-11-23 baseline included, and TASK-55.5's notes list each merge case with its Rust test
- [ ] #4 mise run rust:ci, mise run lint and mise run alz:check pass, and src/ is unchanged (git diff --stat main -- src)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Spec items collected by the lead for a spec pass before the R4 review (2026-11-23), from R4.4: (a) instructions mode values: absent or append adds, replace, remove; anything else is an instructions error; (b) replace or remove with no base is an unresolved error (add to §10.1's unresolved list), dropped by single-layer lint; (c) a replacing rule without paths is a rule error 'paths is required; always-loaded text belongs in instructions/'; (d) bodies and CLAUDE.extra.md text are trimmed of Unicode White_Space and a block empty after trimming is dropped (the reference's JS trim also removes U+FEFF: drift); (e) the layer heading capitalises the first character of the layer's kind and is printed as untrusted text; (f) duplicate globs within one rule's paths are kept once (the reference kept them on replace: drift).

Spec items from R4.5 (2026-11-23, lead accepts the choices): (a) the keys that have a neutral field inside a target map: claude name, description, tools, model, permissionMode, skills, isolation, memory; codex name, description, model, developer_instructions; (b) memory accepts none, project, user and local (the reference's set); (c) permission_mode values bypass_permissions, auto, accept_edits, default, dont_ask, plan; (d) §10.1's 'too many preloaded skills' has no defined limit: drop it from the agent code or define one; (e) overlay schema errors use agent-overlay for extends and mode and agent for the rest (the reference uses agent-overlay for all: drift), a missing extends is agent-overlay 'extends is required'; (f) removing a tool from an agent with no tool list is an agent-overlay error; (g) the list-edit syntax (+[..], -[..], [..], comma split, one pair of quotes stripped, no ] inside an item, empty means no change); (h) name equals the file stem (§4.1); (i) native fields keep source order (TASK-66, lead): first appearance keeps its place, later-layer keys appended, whole-number keys in source order (the reference hoists them: drift); settings stay sorted.

Spec items from R4.6 (2026-11-23): (a) a base path scope whose segments other than the last end in ** covers only an equal scope (keeps covers transitive); (b) the neutral tool vocabulary (the reference's 12 names) plus any name starting mcp__; anything else is an agent error, also inside -[..]; an empty scope (write:) is accepted as written; (c) 'the base' is the agent the overlay refines; a later overlay cannot re-widen what an earlier one narrowed; (d) a refused overlay is not applied; (e) allow_widen in a public layer is an error even without a tools edit or a base; (f) docs/** covers docs itself, ** covers any path without a dot segment, braces are literal.
<!-- SECTION:NOTES:END -->
