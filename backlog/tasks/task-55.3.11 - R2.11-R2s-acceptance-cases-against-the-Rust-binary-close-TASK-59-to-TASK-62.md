---
id: TASK-55.3.11
title: 'R2.11: R2''s acceptance cases against the Rust binary; close TASK-59 to TASK-62'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T06:55:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.3.9
  - TASK-55.3.10
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 100390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. TASK-55.3 AC #1: "The suite's model and lint cases pass against the Rust binary". The cases:
- 29 "§13.24 account layer field: schema": the 6 accept and 20 refuse cases (`alz lint --layer`). Its build case stays for R5.
- 26: "accepts an apostrophe inside a name" and "lints a valid declaration in a single layer", plus a new `alz lint --layer` twin of "rejects %s as a schema error, before build --adopt writes anything" (the same table; both binaries).
- New `test/acceptance/34-yaml-limits.test.ts` with fixture `test/fixtures/34-yaml-limits` (spec §13 entry from R2.1), through `alz lint --layer`, `referenceDrift` where the reference fails: an unresolved alias whose name is a marker that is never printed (TASK-59); a cyclic alias, and nesting 65 deep refused while 64 parses (TASK-60); 1 MiB + 1 byte, invalid UTF-8, 101 aliases over three anchors, 20001 nodes and 10001 replayed events refused, each limit itself accepted (TASK-62); `!foo` and `!!binary` refused with `yaml-tag`; `"a\x07b"` refused; two documents refused.
- Stack mode, in the same file or `35-manifest-schema`: `ref: [x]` and `ref: 1.10` refused by `alz lint` in the repository with an error naming the entry (TASK-61, drift); their `alz build` twins are written now (drift) and pass against Rust from R5.
- 10 "a clean public layer passes lint" passes too, but the leak guard stays R5's.

Run: `mise run rust:build`, then `ALZ_BIN=$PWD/target/release/alz mise run test:acceptance` on the files and case names above, and `mise run test` with ALZ_BIN unset (drift cases are expected failures there).

This pull request also closes TASK-59, TASK-60, TASK-61 and TASK-62, which depend on this task: their criteria are checked with this run as evidence. TASK-61 closes here through its lint case (owner answer 5): its build half is checked by the shared parser's unit tests (R2.6), and its build case turns green in R5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every case listed in the description passes against ALZ_BIN=target/release/alz; the list with counts is in the notes
- [x] #2 mise run test passes with the reference, each new drift case is an expected failure there and names its task
- [x] #3 TASK-59, TASK-60, TASK-61 and TASK-62 have every criterion checked with the evidence of this run and go to In Review with this pull request
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R2.5 (2026-11-23): test 29 asserts stderr contains `'account` (leading quote) and the reference's "'account' keys must be target names"; spec §10.1 prints key paths unquoted and the Rust alz reports a bad target key as "unknown field 'cursor' in account". Assert the spec form (e.g. contains `account`), with referenceDrift where the reference fails it. Test 26's `commit_identity` assertion still holds. Open for R5: LayerVersion, Description, SlotName and ForbidPatternText have no rule beyond being YAML text; decide before they reach the AGENTS.md header.

Implementer (2026-11-23), uncommitted on feat/rust-r2-task-55.3. Changed: test/acceptance/29-account-layer.test.ts (schema assertion is now the unquoted 'account', the spec form of 10.1); 26-commit-identity.test.ts (the refusal table is shared as REFUSED_IDENTITIES; new 'lint --layer rejects %s as a schema error' twin: 30 rows with error [layer] and commit_identity, 3 rows that decode to a control character as error [yaml] under referenceDrift TASK-55.3); new 34-yaml-limits.test.ts and 35-manifest-schema.test.ts with fixtures; docs/LAYERING-SPEC.md 7.13 names the warning codes account-config and account-allowed.
Against ALZ_BIN=target/release/alz (cargo build --release --locked --package alz): 29 'accepts' + 'lint refuses' = 26 passed; 26 lint cases (33 table twins + apostrophe + valid declaration) = 34 passed; 10 'a clean public layer passes lint' = 1 passed; 34-yaml-limits = 18 of 18 passed; 35-manifest-schema lint, version and text cases = 5 passed. The two 35 'alz build' twins fail against the R2 binary (usage error, exit 2): they turn green in R5 (TASK-55.6).
With ALZ_BIN unset (reference): 34 has 7 passed and 11 expected failures; 35 has 2 passed and 5 expected failures; 26 has 3 more expected failures. Each drift case's failure reason was checked against the bundled reference by running it as ALZ_BIN (dist/cli.js): 1 MiB+1, invalid UTF-8, 101 aliases, 20001 nodes, 10001 replays accepted (TASK-62); 65 levels and cyclic alias accepted (TASK-60); unresolved alias name printed (TASK-59); local tag, !!binary, control-character escape (TASK-55.3); ref list and number, as error [source] not [manifest] (TASK-61); version: 1.10 accepted (TASK-55.3).
Left for later phases: 29's build and 'who wins' and init cases (R5, R6), 26's build/adopt cases, 10's other cases (leak guard, R5), 35's two build twins (R5). 
Code vs spec grep (R2.1 AC1): codes emitted in crates/: yaml, yaml-tag, layer, manifest, no-manifest, lock, source, account-config, account-allowed, not-ported. All named in the spec except not-ported (deliberately temporary) and account-config and account-allowed, which R2.11 added to 7.13 (User config and Allow state bullets). Observation for the R5/R2 unit review: summary and where use the directory as given (../personal) even where the layer name is known and parsed; the spec says <layer> is the name; the cases do not assert it.
Checks: mise run rust:ci exit 0; mise run lint passes; mise run test (reference) 48 files, 1119 passed, 23 expected fail, 1 skipped.

Correction to the counts above: the 26 lint cases are 32 table twins (29 error [layer] and 3 error [yaml] drift rows) + apostrophe + valid declaration = 34 passed.

Verified by the lead (2026-11-23): ALZ_BIN=target/release/alz: 34+35 give 23 passed and 2 failed (the two alz build twins in 35, R5); 29 -t schema 26 passed, 1 failed (its build case, R5); 26 -t lint 33 passed. mise run test (reference): 48 files, 1119 passed, 23 expected fail, 1 skipped. rust:ci exit 0, lint passes, src/ unchanged. AC #3: the board has no In Review status, so TASK-59 to TASK-62 stay In Progress with every AC checked until the R2 unit review; the lead sets them Done when the R2 PR merges. Open for the unit review: lint prints the layer directory as given in <where> and the summary even when the name parsed; §10.1 says <layer> is the name once known.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
R2's acceptance cases run against the Rust binary: test 29's account schema cases, a lint twin of test 26's identity table, the new 34-yaml-limits (cap, UTF-8, alias, depth, node and replay limits, tags, controls) and 35-manifest-schema (TASK-61 refs, version as text), asserting the spec form; cases the TypeScript reference fails are marked reference drift with their task. The two alz build twins wait for R5.
<!-- SECTION:FINAL_SUMMARY:END -->
