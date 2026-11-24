---
id: TASK-55.6.15
title: 'R5.15: R5 acceptance, output parity and the drift list'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T05:48:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.6.9
  - TASK-55.6.11
  - TASK-55.6.12
  - TASK-55.6.14
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 147390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: close R5.
- Run the whole acceptance suite against ALZ_BIN; record pass and fail per file in TASK-55.6's notes; every file listed in TASK-55.6's plan passes; every case left for R6 is named in TASK-55.7's notes.
- The output parity list (TASK-73 AC #4): each reference drift case of spec §10.2 reachable through adoption has an acceptance case checking alz's own bytes, marked referenceDrift TASK-73; the golden trees list every drift sample.
- Acceptance cases for the writer rules of R5.1 (a passthrough .claude/Settings.json collides; a protected path in another case is left alone; a generated path under a link is refused; a lock entry alone does not make a file deletable), each referenceDrift where the reference fails it.
- test 37: every row, including the rows for Codex keys that duplicate first-class items, also through alz build, writing nothing; test 33's name-order drift cases.
- Carried criteria: TASK-52 #4, TASK-53 #4, TASK-63 #4, TASK-71 #3, TASK-66 #2, TASK-73 #4, and TASK-69 #5 where a Linux run exists; tick each with its evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every acceptance file the TASK-55.6 plan lists passes against ALZ_BIN=target/release/alz, and the suite against the reference has only the expected drift failures (mise run test and the ALZ_BIN run, counts in the notes)
- [ ] #2 The writer-rule and parity drift cases are in the suite and pass against the Rust binary (vitest run)
- [ ] #3 mise run rust:ci, mise run lint and mise run alz:check exit 0, and src/ is identical to main (commands in the notes)
- [ ] #4 The carried criteria listed in this task's description are checked with evidence, and the R6 remainder is in TASK-55.7's notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.16 (2026-11-24): remove continue-on-error from the acceptance-rust job in .github/workflows/tests.yml and mark the R6 cases (identity, account) as expected failures there, which also closes R5.16's AC #2.

From R5.8 (2026-11-24), to settle in R5.15: (1) parity: the build summary line prints targets in sorted order, the reference in declared order — follow the reference; (2) the AGENTS.override.md walk skips a folder whose entries have names that are not UTF-8 — it must still find an override there (or refuse the build); (3) test 24's 12 rows 'agent frontmatter, claude.mcpServers list/map' expect error protected-variable while the Rust alz reports error agent first (both refuse) — decide by the spec and mark reference drift or reorder the checks; (4) test 24's 'JSON nested deeper than 32 levels' row: the protected-variable scanner needs the depth limit; (5) 3 rows of test 24 need account which/allow (R6).

From R5.10 (2026-11-24): alz explain strips tabs and carriage returns from the merged file it prints (class 1 guard). Tabs are harmless in a terminal; keep tabs and strip only what can move the cursor or hide text (CR, escapes, bidi and format characters), which also narrows the drift from the reference.

From R5.13 (2026-11-24): (1) hook scripts adopt to .layer/hooks/scripts/<path> with run: scripts/<path>, the layout the spec and the reader require; test 20's expectation of .layer/hooks/guard.sh becomes reference drift. (2) Floats and nulls in an adopted settings.json or .mcp.json: the YAML emitter writes none, so adoption refuses them, yet typed Number keys exist (e.g. skillListingBudgetFraction); if the reference writes such floats, teach the YAML emitter to write floats exactly (round trip) and drop the refusal, else state it in §4.3. (3) Model alias tie-break: follow the reference's insertion order rather than name order. (4) The reader check runs after conversions succeed, so problems come per phase; say so in §4.3.

From R5.14 (2026-11-24): (1) align the adoption problem for a literal env value in .mcp.json with the spec's text (Use "${<NAME>}" and export it); (2) a link at .claude/settings.json or .mcp.json that the build does not write is skipped silently — print a note naming the path; (3) test 24's two lock rows assert the spec's wording as referenceDrift('TASK-55.6.15', ...) — keep or retarget.

Baseline (2026-11-24, ALZ_BIN=target/release/alz, full acceptance project): 742 passed, 281 failed. Failing files: 24 (261/16), 26 (97/24), 28 (65/12), 29 (48/6), 30 (11/63), 31 (13/103), 32 (6/55), 33 (2/2); all others pass whole. 30-32 and the identity parts of 26 and 29 are R6. Owner decision for ls --live (2026-11-24): pass AWS_* only when CLAUDE_CODE_USE_BEDROCK is set, and GOOGLE_*/CLOUD_ML_REGION/ANTHROPIC_VERTEX_PROJECT_ID only when CLAUDE_CODE_USE_VERTEX is set, recorded in target-facts first; everything else stays cleared.

Implementer (2026-11-24), partial: tree green, parity items 1-3 largely done, item 4 done. DONE: (1) build summary targets in declared order (Manifest.targets is an OrderedSet; unit test; spec 10.3 sentence); (2) AGENTS.override.md walk: a folder whose name is not valid Unicode or not a safe path makes the build refuse (error write, 'holds an entry whose name alz cannot read'), a file with such a name is skipped, .git/.DS_Store/node_modules are passed over before parsing the name; alz-io tests in crates/alz-io/tests/repo_root.rs; spec section 4.3 'The search for AGENTS.override.md'; (3) test 24 agent claude.mcpServers rows (12): the spec says the error agent (7.2), so the rows are referenceDrift TASK-55.5.16 expecting error [agent], spec 7.9 says so; (4) protected-variable depth: the spec already says 128 (drift TASK-55.5.12); test 24 now has a 40-level drift case and a 200-level case that both binaries refuse; explain keeps tabs (terminal_text::strip_untrusted_keeping_layout, unit test, spec 10.3); adoption hint for a literal env value now reads Use "${NAME}" and export it (no new test, mcp_json.rs); spec 4.3 says problems come in two phases. Tests 28 and 33: 33 uses sandbox.ignoreViolations (the settings key table of TASK-79 refuses unknown top-level keys, so whole-number keys can only sit in a free-keyed mapping); 28 message assertions follow the spec wording and are referenceDrift TASK-55.6.15 (lock field errors, cache entry link, sha not a commit id, control-character file names, Unicode-form clash). 24: JSON depth rows as above. LEFT (not done): link at .claude/settings.json or .mcp.json that the build skips needs a printed note naming the path; model alias tie-break (insertion order); floats in adopted settings (check whether the reference writes them; else state in 4.3); carried criteria and AC ticks. CI: continue-on-error removed from acceptance-rust; the step runs vitest with the JSON reporter and test/ci/check-r6-pending.ts compares failing cases with test/ci/r6-pending.json (256 R6 cases, each with its reason); it fails on any other failing case and on a listed case that now passes. actionlint (shellcheck 0.11.0) and zizmor --offline: no findings. DRIFT LIST: spec 13 'The reference drift list' (grouped by task, all referenceDrift cases). RESULTS: baseline 742 passed / 281 failed; now 770 passed / 256 failed / 5 skipped (1031). Remaining failures, all R6 (TASK-55.7): 24: 3 (two use alz account which; the ESC hub-folder case also needs a rewrite because the Rust alz refuses a layer.yaml with a control character after escapes), 26: 24 (commit identity setup, warnings, check, schema rows), 28: 2 (init sets identity), 29: 6 (account which, init account warning), 30: 63, 31: 103, 32: 55 (account which/allow). Checks: mise run rust:ci exit 0, mise run lint exit 0, mise run test exit 0 (1169 passed, 237 expected fail, 5 skipped), mise run alz:check exit 0, src/ unchanged (git diff src empty), ALZ_BIN run plus check-r6-pending exit 0. Not committed; ACs not ticked.
<!-- SECTION:NOTES:END -->
