---
id: TASK-55.9.4
title: State in the spec and target facts only what alz does
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T13:09:00Z'
labels:
  - area:spec
  - area:docs
milestone: m-7
dependencies:
  - TASK-55.9.1
parent_task_id: TASK-55.9
priority: medium
ordinal: 186390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
docs/LAYERING-SPEC.md still describes the port: §13's reference drift list and about 175 lines naming the TypeScript alz, the reference, reference drift, parity, Node or port phases. Under decision-39 the spec states only what alz does, and the later subtasks write Rust tests from it, so the text must be final before the coverage matrix is built. First the user-visible differences from the published 0.1.0 that the drift list names move to CHANGELOG.md's Unreleased section in neutral words, so the first Rust release can describe them. docs/target-facts.md is reworded the same way (its facts about Claude Code, Codex and git stay). §13's fixture path becomes fixtures/. This is an editorial change of the contract: no behaviour, layer format or generated output changes, so it needs no version bump by itself. Out of scope: new rules (a spec hole found later is its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 grep -nE 'TypeScript|[Tt]he reference|reference drift|referenceDrift|parity|Node|npm|vitest|Rust .alz' docs/LAYERING-SPEC.md docs/target-facts.md prints nothing, or only lines the notes list as not about an implementation (grep)
- [ ] #2 §13 has no drift list, each item states only behaviour, and its fixture sentence names fixtures/ (spec diff)
- [ ] #3 CHANGELOG.md Unreleased states each user-visible difference from 0.1.0 that the drift list named (diff review)
- [ ] #4 mise run rust:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence (architect, branch docs/spec-alz-only-task-55.9.4, based on fd6bfb3):
- AC1: grep -nE 'TypeScript|[Tt]he reference|reference drift|referenceDrift|parity|Node|npm|vitest|Rust .alz' docs/LAYERING-SPEC.md docs/target-facts.md prints one line: docs/target-facts.md:409 (C10), 'unverified for git and npm marketplaces'. Not about an implementation: npm is a Claude Code plugin marketplace source type. Vendor-doc mentions ('the reference page', 'in the reference') in target-facts §13 were reworded to name the vendor page; no fact changed.
- AC2: §13 has no drift list (the table, its intro and the trailer paragraph are removed). The intro reads 'The tool is done when all of these hold. Rust tests prove them (decision-39), and the fixture repositories they build are under fixtures/'. Items 13, 19, 20, 23, 24, 25 to 32 no longer name test files, test/drift.ts, traces, source scans, the import graph, R5/R6 or pending lists; 31 is 'Golden output', 32 'Commit identity and accounts', both as behaviour.
- AC3: CHANGELOG Unreleased has a new bullet 'More differences from 0.1.0' with the plain behaviour changes the drift list named: YAML tags and second documents, numbers where text is expected, schema errors, finite numbers in typed keys, nested env depth, ls order, the explain prefix, adoption of agents and hook scripts, message wording of a refused lock, cache entry or pinned tree, generated YAML quoting, the one-line identity steps and the .git/config reason. The older Unreleased entries say 0.1.0 instead of the TypeScript alz; nothing else in them changed. By the lead's decision, the remaining rows go into the 0.2.0 release notes through TASK-55.8.4, not into a committed file now. No version bump.
- AC4: mise run rust:check exit 0 (fmt, clippy -D warnings, tests), first after commit 8e3dab2 and again after the CHANGELOG follow-up; the target-facts guard every_key_of_the_tables_is_recorded_with_its_path_in_the_target_facts passes.
- Other port words removed although outside the grep: R2/R5 phase mentions in §7.6 and §7.13, JavaScript JSON.parse in §4.3. Kept: 'JavaScript's white-space set' in §9.2 (defines a character set) and Rust's std/regex-lite/OsStr names (define behaviour).
- For the owner: §10 'Implementation' now reads 'Rust (decision-21, decision-39), one binary built with the toolchain that mise pins. Dependencies are kept minimal.' §10.3 init now says 'The files have fixed contents' (follow-up noted on TASK-55.9.9). §13 item 19's untested folder ownership is noted on TASK-55.9.5. §10.2 still names samples.json; its drift fields and the .mjs generator belong to TASK-55.9.3.
<!-- SECTION:NOTES:END -->
