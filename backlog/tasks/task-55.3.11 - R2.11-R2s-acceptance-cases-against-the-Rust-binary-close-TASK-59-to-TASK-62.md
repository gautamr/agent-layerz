---
id: TASK-55.3.11
title: 'R2.11: R2''s acceptance cases against the Rust binary; close TASK-59 to TASK-62'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
updated_date: '2026-11-23T05:27:00Z'
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
- [ ] #1 Every case listed in the description passes against ALZ_BIN=target/release/alz; the list with counts is in the notes
- [ ] #2 mise run test passes with the reference, each new drift case is an expected failure there and names its task
- [ ] #3 TASK-59, TASK-60, TASK-61 and TASK-62 have every criterion checked with the evidence of this run and go to In Review with this pull request
<!-- AC:END -->
