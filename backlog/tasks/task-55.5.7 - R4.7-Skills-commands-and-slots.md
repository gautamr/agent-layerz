---
id: TASK-55.5.7
title: 'R4.7: Skills, commands and slots'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T13:49:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.6
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 120390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.3, §7.4).
- item/skill.rs: a skill is its directory with all its files from LayerContents; the SKILL.md name equals the directory name (kebab-case) and a description is required. commands/<name>.md becomes a skill that only explicit invocation reaches; a command and a skill with one name in one layer is an error.
- merge: Replace per name, the most specific layer wins with the whole directory.
- Slots: every name in a merged agent's skills or in a layer's requires_slots must exist; a provides_slots entry must name a skill of that layer. strict_slots true gives a `slot-missing` error; false gives a stub skill and a warning. The user-directory collision report is doctor's (R5).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 05's merge verdict has a Rust test: the repo skill replaces the personal one with its whole directory (unit test)
- [x] #2 Test 06's cases have Rust tests with the same verdict: a missing slot fails with strict_slots true and gives a stub and a warning with false (unit tests)
- [x] #3 A SKILL.md name that differs from its directory, a command and a skill sharing a name in one layer, and a provides_slots entry without its skill are errors (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (R4.7). Files: crates/alz-core/src/item/skill.rs (+ item/skill/tests.rs): SkillItems::read reads skills/<name>/ whole directories (SkillDirectory with every SkillFile and its bytes), commands/<name>.md (Command), the skill-command clash and the layer's requires_slots as SkillName; SkillItems::check_provided is the per-layer provides_slots check. crates/alz-core/src/merge/skills.rs (+ tests): SkillState folds with Replace per name (Traced, Define then Replace), finish fills slots (strict -> slot-missing error, else warning + stub in Merged::slot_stubs()), slot-command for an agent that preloads a command. Merged::skills() and Merged::slot_stubs() added; Code gained skill, command, skill-command-clash, slot, slot-missing, slot-command; schema::Refusal gained SkillDescription; MergedAgent::file() added (where an agent was defined, for the location of slot diagnostics); merge::testing gained layer_from_yaml, merged_with, merged_without_slots, lines_but_slots (agents tests use made-up skill names, so they use the last two).
AC1 (test 05 verdict): merge::skills::tests::case_05_the_repo_skill_replaces_the_personal_one (reads the fixture's two SKILL.md and layer.yaml via include_str), case_05_the_provenance_lists_the_overridden_layer_first, the_whole_directory_is_replaced_and_files_of_the_older_one_are_dropped (extra files of the general directory are gone), proptest the_most_specific_layer_that_has_a_name_wins_with_exactly_its_files. The doctor half of test 05 (user-dir collision, exit 1) is doctor's, R5.
AC2 (test 06): case_06_a_missing_slot_fails_with_strict_slots (error [slot-missing] personal:layer.yaml, merged() is None), case_06_a_missing_slot_gives_a_stub_and_a_warning_without_strict_slots (warning, merged is Some, slot_stubs holds domain-glossary, no skill); also strict_slots_is_the_default_of_the_manifest, a_slot_a_more_specific_layer_fills_is_not_missing, agent-preload cases (a_skill_a_merged_agent_preloads_is_a_slot_that_must_exist, a_skill_an_overlay_adds_to_an_agent_is_a_slot_too), a_command_fills_a_slot_a_layer_requires. The files on disk (no domain-glossary/SKILL.md when strict; stub text 'not defined for this repo' when loose) are the renderer's, R5; the stub text is not produced in R4.7, only the stub entry.
AC3: item::skill::tests::a_skill_md_name_that_differs_from_its_directory_is_an_error, a_command_and_a_skill_with_one_name_in_one_layer_are_an_error (skill-command-clash), a_provides_slots_entry_without_its_skill_is_an_error (slot), plus no SKILL.md, non-kebab directory, missing name/description, unclosed frontmatter, command schema, requires_slots not a skill name, untrusted directory name stripped.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint exit 0. test/ and src/ untouched. Tests were written with the code in one pass, not shown failing first.
Spec gaps and choices: (1) §10.1 'agent' lists 'too many preloaded skills' but §7.2 gives no limit; the reference's 32 (target-facts) is not implemented in R4.5 or here; the spec needs the number, or the check moves to the budgets. Not done. (2) §7.4 gives no frontmatter fields for a command; I took the reference's (description required, argument_hint optional, any other field an error code command); the spec should list them. (3) §7.3 does not say whether alz build checks provides_slots; the reference checks it only in alz lint, so SkillItems::check_provided is for R4.15 (lint) and the merge does not call it. A requires_slots entry that is not a skill name is a slot error in the reader (build and lint), because a stub would be a directory of that name. (4) The slot-missing location is not in the spec: I use <layer>:layer.yaml for a layer's requires_slots and the agent's defining file for a preload; the first demander is named (layers' requires_slots before agents). (5) A command and a skill with one name in one layer: the command is dropped, the skill kept (as the reference), so one error comes out. (6) SKILL.md frontmatter other than name and description is not checked (spec silent; the reference does not either).

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ and test/ unchanged. Choices accepted; lead: 'too many preloaded skills' is dropped from §10.1's agent code (no limit in the spec, none in the reference).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Skills (whole directories) and commands merge in one namespace with Replace per name and provenance; slots that layers require or agents preload must exist (strict: slot-missing error; loose: warning and a stub entry for R5); SKILL.md name, provides_slots and command schemas are checked; tests 05 and 06 have Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
