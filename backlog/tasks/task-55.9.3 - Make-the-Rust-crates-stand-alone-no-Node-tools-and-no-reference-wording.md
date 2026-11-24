---
id: TASK-55.9.3
title: 'Make the Rust crates stand alone: no Node tools and no reference wording'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
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
