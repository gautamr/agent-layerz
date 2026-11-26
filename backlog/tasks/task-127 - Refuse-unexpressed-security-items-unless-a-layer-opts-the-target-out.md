---
id: TASK-127
title: Refuse unexpressed security items unless a layer opts the target out
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-25T19:30:00Z'
updated_date: '2026-11-26T03:59:00Z'
labels:
  - area:adapter
  - discovered
milestone: m-8
dependencies:
  - TASK-109
references:
  - decision-40
  - docs/LAYERING-SPEC.md
priority: high
type: feature
ordinal: 228390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner decision on 2026-11-26 (TASK-104 notes): when an adapter cannot express a security item, alz fails the build with an error naming the item and the target, never a warning, and a layer may opt that target out of the item explicitly. TASK-109 adds the coverage report and refuses a report that leaves an item out; this task adds the refusal of an item reported as not expressed and the opt-out field unenforced of permissions.yaml, as spec §7.6 'Targets that cannot enforce an item' and §8.0 'Security items a target cannot enforce' state (TASK-104). User-facing: a new layer field, so a minor version bump and a CHANGELOG entry. Out of scope: autonomy items (doc-7) and the Codex adapter (TASK-126), which is the first adapter that needs it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 permissions.yaml accepts unenforced.<target id> with deny, floor_fields, default_mode and hooks as spec §7.6 says; another shape, a rule this layer does not deny, a floor field it does not set to true, default_mode: true without its own floor in force, or a hook id that is not its own locked hook is the error 'permissions', also in alz lint (tests)
- [ ] #2 A test adapter that reports a deny rule, a floor field, the default_mode floor or a locked hook as not expressed fails the build with the error 'target' at <layer>:permissions.yaml and the message of spec §8.0 for that kind, naming the item and the target, and nothing is written (one test per kind)
- [ ] #3 When every layer that set the item opts the target out of it the build passes with no diagnostic for it; an entry of a more specific layer never covers a more general layer's item (tests, one per kind)
- [ ] #4 docs/spec-coverage.md rows §7.6 Targets that cannot enforce an item and §8.0 Security items a target cannot enforce are covered; the acceptance suite, mise run rust:check and mise run alz:check pass with unchanged output
- [ ] #5 A locked hook with on_fail: deny on stop, subagent_stop or post_tool fails a claude build with the error 'target' naming the hook and the target, and none of the warning hook-on-fail; with unenforced.claude.hooks listing it in its own layer the build passes with the warning hook-on-fail no longer printed for it; an unlocked hook keeps the warning (tests)
- [ ] #6 The Claude adapter reports every locked hook and the default_mode floor as expressed, next to the deny rules and floor fields of TASK-109, except a locked hook with on_fail: deny on stop, subagent_stop or post_tool, which it reports as not expressed (test)
<!-- AC:END -->
