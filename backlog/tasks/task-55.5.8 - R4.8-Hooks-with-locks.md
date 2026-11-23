---
id: TASK-55.5.8
title: 'R4.8: Hooks with locks'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T14:01:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.7
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 121390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.5).
- item/hook.rs: hooks.yaml is a list of HookEntry = Define(Hook) | Disable(HookId), a disable holding only id and disabled: true. Hook { id, event: Neutral(NeutralEvent) | Native { target, name } with its matcher, match (tools, paths), action: Run(SafeRelPath inside the layer) | Command(CommandLine, ${PROJECT_DIR} kept for the adapter), on_fail, targets, timeout in seconds, locked }.
- merge: every hook of every layer, in stack order then file order. An id that a more general layer already defines is an error. A disable removes an unlocked hook of a more general layer; disabling a locked hook is an error naming the layer; disabling an unknown id is `unresolved`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 08's cases have Rust tests with the same verdict: disabling a locked hook fails naming the layer, and disabling an unlocked hook removes it (unit tests)
- [x] #2 Test 19's cases have Rust tests with the same verdict: command hooks are kept as written with ${PROJECT_DIR}, native events keep their matcher, and a hook with both run and command is refused (unit tests)
- [x] #3 Proptest: for any stack and any further layer the merge refuses or keeps every locked hook enabled, and hook order is stack order then file order
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R4.8 implementation notes (checks: mise run rust:ci green, mise run lint green; test/ untouched so mise run test not run).

Files: crates/alz-core/src/item/hook.rs (+ hook/vocabulary.rs, hook/tests.rs), merge/hooks.rs (+ hooks/tests.rs); wired in item.rs, merge.rs, merge/stack.rs (Merged::hooks() -> &[Traced<Hook>]), schema.rs (Refusal::Hook), diagnostic.rs (Code::Hooks "hooks", Code::HookLocked "hook-locked"; unresolved reused), item/agent/tool.rs (ToolName::named now pub(in crate::item)).

AC1 (test 08): merge::hooks::tests::case_08_disabling_a_locked_hook_fails_naming_the_layer (error [hook-locked] shop-api:hooks/hooks.yaml: hook 'block-secrets' is locked by layer personal and cannot be disabled by layer shop-api, merged() is None); case_08_disabling_an_unlocked_hook_removes_it_and_keeps_the_locked_one (format-on-save gone, block-secrets kept: run hooks/scripts/block-secrets.sh, pre_tool before, tools [read], paths [.env*], on_fail deny, locked). Both read the fixture files with include_str!.
AC2 (test 19): case_19_command_hooks_are_kept_as_written_with_the_project_dir_variable (command text byte-equal with ${PROJECT_DIR}, timeout 10, deny, edit+write filter); case_19_a_native_event_keeps_its_matcher_and_runs_on_its_target_only (claude:Notification, matcher permission_prompt, targets {claude}); case_19_a_hook_with_both_run_and_command_is_refused (error [hooks] ... needs exactly one of 'run' (a script in the layer) or 'command').
AC3: proptests in merge/hooks/tests.rs: no_later_layer_can_disable_a_locked_hook (stack + further layer: refuses, or every locked hook of the stack is still there and locked; remaining hooks keep order; errors never disappear), the_merge_follows_the_rules_of_the_spec_for_any_stack (model-based oracle written from section 7.5: error iff model errors, and stack order then file order of the survivors), a_layer_that_disables_nothing_keeps_every_hook_in_its_place. Plus unit tests: stack order then file order, duplicate id (cross-layer and in-layer), disabled id stays taken, unknown disable unresolved, a layer cannot disable its own hook, locked hook survives a distant disable, disable entry shape, all problems reported in one run. item/hook/tests.rs: 16 reader tests (every field, every refusal and its key path).

Types: Hook { id: HookId, trigger: Trigger, action: HookAction (Run(HookScript) | Command(CommandLine)), on_fail, targets: Option<OrderedSet<TargetName>>, timeout: Option<Seconds(NonZeroU32)>, lock: Lock (Open | Locked) }. Trigger = Tool{when, filter: ToolFilter::Any|Tools{tools non-empty, paths}} | Lifecycle(4 events) | Native{event: NativeEvent{target, name}, matcher}; so a match on a non-tool event, a matcher on a neutral event and paths without tools cannot be built. HookEntry = Define(Hook) | Disable(HookId).
Deviation from plan text: Trigger splits Neutral(NeutralEvent) so the filter only exists on tool events; Floor is not used for the lock (a hook's lock is set once and a disable is the only removal, so Lock is an Ord enum and the law is tested through the merge).

Spec gaps and choices (spec not edited; listing what it needs):
1. Hook id grammar: spec says only "unique id". Chosen: any text with a visible character, printed as a quoted key. Spec needs a grammar (kebab-case like other ids?).
2. timeout: spec says "in seconds". Chosen: positive whole number up to u32::MAX (reference accepted any positive number, fractions included). Spec should say whole seconds.
3. on_fail default: spec lists deny|warn|feedback, no default. Chosen warn (as the reference).
4. Native event grammar: `<target>:<Event>` with target claude or codex (reference accepted any lowercase word) and Event ^[A-Z][A-Za-z]+$ (reference regex). Spec should state both. A native event lists no target other than its own; targets: [] is an error (spec silent).
5. match.tools entries: bare neutral tool or mcp__ tool, no scope (reference same). Spec section 7.5 does not say which tools.
6. A disabled id stays taken: a more specific layer defining it again is a hooks error (reference behaviour; spec silent). Disabling a hook of the same layer, or one already disabled, is unresolved.
7. A same-layer duplicate id is a hooks error (spec says "more general layer"); message names the defining layer.
8. Diagnostics of the merge carry no line and column (the reference has none); reader errors do.
9. timeout/match/matcher: 'matcher' on a neutral event and 'match' on a native or non-tool event are errors (reference same).
No reference drift found for tests 08 and 19; no referenceDrift case added; test/ unchanged.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (920 tests), lint passes, src/ and test/ unchanged. Choices accepted; whole-second timeouts checked against fixtures and .layer (none fractional).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Hooks merge in alz-core: neutral tool and lifecycle events and native events per target, exactly one of a script below hooks/scripts/ or a command, ids unique across the stack, disables of earlier hooks, and locks that no later layer can disable (property-tested floor law and a model of §7.5); tests 08 and 19 have Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
