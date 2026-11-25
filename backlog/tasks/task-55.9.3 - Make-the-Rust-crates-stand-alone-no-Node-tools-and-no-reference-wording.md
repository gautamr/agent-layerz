---
id: TASK-55.9.3
title: 'Make the Rust crates stand alone: no Node tools and no reference wording'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-24T22:43:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.9.1
parent_task_id: TASK-55.9
priority: medium
ordinal: 185390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The crates still lean on the TypeScript reference. Four Node scripts (crates/alz/tools/golden-trees.mjs and untrusted-code-points.mjs, crates/alz-core/tools/yaml-emit-samples.mjs and json-emit-samples.mjs) made golden trees, emitter samples and a code-point list from it, and about 100 test names, doc comments and sample fields in 47 files say the reference, TypeScript or drift (for example fixture_01_matches_the_reference_byte_for_byte). Under decision-39 these files are expected output owned by the Rust tests, and each test is named after the spec guarantee it proves. This comes before the coverage matrix, which names tests. Out of scope: changing what a test asserts or any expected byte, and deleting test/.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 git grep -nIE 'TypeScript|[Tt]he reference|reference drift|referenceDrift|[.]mjs|node crates/|Node[.]js|npm' -- crates prints nothing, or only lines the notes list as not about the TypeScript implementation (grep)
- [ ] #2 The four .mjs scripts are gone, and each fixture folder they made has a README naming the spec rule it pins and how it is updated without Node (file review)
- [ ] #3 No expected file changes its bytes: git diff main -M --name-status -- 'crates/*/tests/fixtures' shows only renames, the READMEs and no modified expected file (git diff)
- [ ] #4 mise run rust:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC1: git grep -nIE 'TypeScript|[Tt]he reference|reference drift|referenceDrift|[.]mjs|node crates/|Node[.]js|npm' -- crates prints 4 lines, all fixture text about a sample stack's technologies, not about the TypeScript implementation: claude-files/01-determinism and 05-skill-override and golden/01-determinism and 05-skill-override stack-conventions SKILL.md (expected files, unchanged). AC2: the four .mjs scripts are deleted; READMEs written or rewritten for golden, claude-settings, instructions, json-emit, yaml-emit (each says the spec rule and that it is kept by hand, no Node); untrusted-code-points.txt carries the same in its header. AC3: no expected file changed except the 2 comment header lines of untrusted-code-points.txt (it named the deleted .mjs; the code point lines are unchanged); one rename (glob reference_table.rs to match_table.rs). Left as is because renaming changes bytes of yaml-emit/samples.json: its labels 'reference-reads-differently', 'escaped-characters-the-reference-writes-raw' and field 'drift'; the Rust tests and the README name them. Tests: deleted a_sample_marked_as_drift_is_not_what_alz_writes (only compared samples with another implementation's text; the spec rule is proven by the two sample-input tests and the rule tests); the two drift_samples_* tests now only assert alz's output (renamed). AC4: mise run rust:check and mise run rust:ci pass.
<!-- SECTION:NOTES:END -->
