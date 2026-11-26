---
id: TASK-109
title: Check every adapter output in the core before the writer
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T07:32:00Z'
labels:
  - area:adapter
  - area:security
milestone: m-8
dependencies:
  - TASK-108
references:
  - decision-40
priority: high
ordinal: 210390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-40 point 4: the core refuses adapter output that leaves the adapter's territory, touches core-owned paths, breaks limits or uses a foreign code, and it requires a coverage report of every security item, so no adapter can weaken §4.3, §4.5 or §9. Needs a security review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test adapter writing outside its territory, or AGENTS.md, layer.yaml, layer.lock, .layer/x or .git/x, gets the error 'target' and the build writes nothing (one test per path)
- [ ] #2 Output beyond the file-count or byte limits is the error 'target' (tests)
- [ ] #3 A code outside the shared adapter codes and the <id>- namespace is the error 'target', and a message with control characters is stripped (tests)
- [ ] #4 A security item of any of the four kinds of spec §8.0 Model. (a deny rule, a floor field, the default_mode floor, a locked hook) missing from the coverage report fails the build; the Claude adapter reports every one as expressed, until TASK-127 changes the locked hook with on_fail: deny on stop, subagent_stop or post_tool (tests, one per kind)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-26): decision-40 accepted. When an adapter cannot express a security item (for example a decision class's rules on a target without allow/ask/deny lists), alz fails the build with an error naming the item and the target, never a warning; a layer may opt that target out of the item explicitly.

Implementation (implementer, 2026-11-26). Checks live in alz-core: crates/alz-core/src/render/adapter_output.rs (emit(descriptor, security, output), codes) with submodules paths.rs (territory, core-only paths), limits.rs (10000 files, 16 MiB, 256 MiB), coverage.rs (coverage and not-expressed). Registry::render (target/registry.rs) runs adapter then emit; rendered.rs uses it. Evidence: AC1 registry-level toy adapter tests in crates/alz-core/src/target/registry/output_checks.rs, one per path (outside territory, AGENTS.md, layer.yaml, layer.lock, .layer/x, .git/x, plus case-folded agents.MD); AC2 output_checks.rs a_file_of_more_than_16_mib / more_files_than_the_limit (real limits) and adapter_output/tests.rs limits with small values, incl. the 256 MiB total; AC3 output_checks.rs a_diagnostic_code_of_another_namespace / a_message_with_control_characters_is_stripped and adapter_output/tests.rs own-code and core-code tests; AC4 output_checks.rs one test per kind for missing, doubled and unknown items, the four not-expressed messages, and the_claude_adapter_reports_every_kind_of_item_as_expressed. Claude output byte-identical: all golden-tree and fixture tests, mise run rust:check, rust:ci and alz:check pass unchanged. Notes: an adapter's own code must be in the code tables (Code is a closed enum), so only claude-agent exists today; a not-expressed item is always the error until TASK-127 adds the opt-out; spec-coverage rows for Output checks, Coverage report and Security items a target cannot enforce stay planned with the remaining sentence named (TASK-110 key rules, TASK-127 opt-out and locked-hook case).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-25T19:52:00Z

---

From TASK-104 (spec, @architect): §8.0 'Output checks.' and 'Coverage report.' and §10.1 'Adapter codes.' state what this task makes true, with the limits 10000 files, 16 MiB per file and 256 MiB in all, and the shared adapter codes models, tool-scope and hook-on-fail. The refusal of an item reported not expressed, and the opt-out field unenforced, are TASK-127 (owner decision of 2026-11-26), which depends on this task.

---

author: @architect
created: 2026-11-26T03:23:00Z

---

From TASK-104, after the owner's answers of 2026-11-26 (@architect): model 1.0 has four kinds of security item (spec §8.0 Model.): deny rules, floor fields that are set, the default_mode floor, and locked hooks that run on the target. The coverage check covers every item of the model, so the Claude adapter must report all four kinds as expressed, although AC #4 names only deny rules and floor fields. TASK-127 AC #4 re-checks the two new kinds.

---
<!-- COMMENTS:END -->
