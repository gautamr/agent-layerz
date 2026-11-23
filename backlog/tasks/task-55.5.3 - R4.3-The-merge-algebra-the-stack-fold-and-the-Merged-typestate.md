---
id: TASK-55.5.3
title: 'R4.3: The merge algebra, the stack fold and the Merged typestate'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T12:53:00Z'
labels:
  - area:rust
  - area:merge
milestone: m-7
dependencies:
  - TASK-55.5.2
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 116390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (TASK-55.5 plan, "The merge algebra").
- alz-core `merge/algebra.rs`: Last<T> (most specific wins), OrderedSet<T> (union keeping the first occurrence), ListEdit<T> (`+[..]`, `-[..]`, `[..]`), Replace<K, V> (most specific wins per key), DeepMerge over alz's ordered tree, and the floor lattice.
- DeepMerge (owner answer 5 in TASK-55.5): the reference's left fold. Maps merge key by key; two lists concatenate and are deduplicated by structural equality of the ordered tree (so mappings that differ only in key order are one item); in every other case, a change of kind included, the more specific value replaces. Its laws are stated for kind-consistent trees, where it is associative.
- `merge/stack.rs`: merge(&ResolvedStack, &Manifest, &Placements) -> Merging { merged: Option<Merged>, diagnostics }, a left fold of each kind's absorb over the layers in stack order. Every kind is merged and every problem reported in one run; merged is None when any error was found. Merged has private fields and no public constructor, so R5 renders only a merged stack. Provenance (layer name and action per contribution) on every item, for alz explain in R5. Later subtasks add one kind each.
- alz-core clippy.toml: disallowed-types for std HashMap and HashSet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Proptest laws pass: associativity and identity of Last and Replace, associativity and idempotence of OrderedSet (commutative as sets), associative composition of ListEdit, associativity of DeepMerge over kind-consistent trees (cargo test -p alz-core)
- [x] #2 DeepMerge keeps the left fold: a later kind replaces an earlier one, and list items equal as ordered trees appear once (unit tests)
- [x] #3 merge is a pure alz-core function from &ResolvedStack to Merging, Merged has no public constructor, and merging the same input twice gives equal results (unit test and proptest)
- [x] #4 alz-core's clippy.toml bans std HashMap and HashSet, and mise run rust:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-23), R4.3 in the working tree, not committed. Files: crates/alz-core/src/merge.rs, merge/algebra.rs, merge/algebra/{last,ordered_set,list_edit,replace,floor,deep_merge}.rs, merge/{provenance,placement,stack}.rs, merge/stack/tests.rs; lib.rs (pub mod merge); crates/alz-core/clippy.toml and crates/alz/clippy.toml (HashMap/HashSet in disallowed-types, twins kept equal).
AC1 (cargo test -p alz-core merge, 49 pass; proptest 1.12.0): Last associative + unset identity (merging_is_associative, unset_is_the_identity_on_both_sides); Replace associative + empty identity (replace::tests); OrderedSet associative, idempotent, commutative as a set (same_members), empty identity, no repeats (ordered_set::tests); ListEdit composition associative, applying a composition equals applying in turn, no-change edit is identity (list_edit::tests; ListEdit is Replace | Adjust{remove,add}, merge = composition); Floor associative, commutative, idempotent, identity, monotone (merged >= both), admits never loosened (floor::tests); Tree::deep_merge associative over kind-consistent trees (generator: a Shape fixed per triple, any mapping key may be missing) and settled-tree idempotence (deep_merge::tests).
AC2 unit tests in deep_merge::tests: a_later_kind_replaces_an_earlier_one, a_later_scalar_replaces..., mappings_merge_key_by_key..., two_lists_concatenate_and_keep_the_first_occurrence..., list_items_equal_as_ordered_trees_appear_once_whatever_the_order_of_their_keys (Tree::Map is a BTreeMap, so key order cannot matter), NaN item deduplicated (FloatBits), and a_path_that_changes_kind_is_not_associative (documents owner answer 5: left fold gives the last list, right grouping gives both).
AC3: merge(&ResolvedStack, &Manifest, &Placements) -> Merging, a pure fold over the layers in stack order with LayerContext per layer; Merging::of keeps merged only when no error, so the two cannot disagree (tests an_error_leaves_no_merged_model..., a_warning_alone_keeps...). Merged has private fields and no constructor: two compile_fail doctests (Merged::new, struct literal) pass. merging_the_same_input_twice_gives_equal_results and proptest merging_is_deterministic_and_keeps_the_stack_order (random stack sizes and placements).
AC4: clippy ban verified to fire (probe with std HashMap gave 'use of a disallowed type' and was reverted). mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doctests, doc, MSRV, deny, audit); mise run lint exit 0.
Design notes for R4.4 to R4.13: add a kind by a module merge/<kind>.rs with absorb, a field in Accumulator (merge/stack.rs) and in Merged (steps in the stack.rs module docs). Provenance = Provenance/Traced<T>/Action(Define, Extend, Replace, Remove; kinds add variants). Placements/Placement live in alz-core (alz-io fills them; a layer with no entry counts as Inside, the stricter answer). The narrowing preorder is left to R4.6 and Declared<T> to R4.13, where their rules are specified; Tree has no conversion from yaml/json yet (R4.10 owns key typing: Tree keys are String, SettingsTree will validate them). Idempotence of deep_merge holds only for lists without repeats, as the reference's fold does, so it is tested on settled trees.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (739 tests; merge 49), lint passes, src/ unchanged. Placements default to Inside for a layer with no entry (stricter, so a missing entry can never let a layer declare): accepted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The merge frame is in alz-core: the algebra (Last, OrderedSet, ListEdit, Replace, Floor, DeepMerge with the owner's left fold) with each law as a property test, Provenance, Placements, and merge(&ResolvedStack, &Manifest, &Placements) -> Merging as a pure left fold, where Merged has private fields and no constructor (compile_fail doctests). HashMap and HashSet are banned in alz-core and alz.
<!-- SECTION:FINAL_SUMMARY:END -->
