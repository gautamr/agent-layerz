---
id: TASK-55.5.3
title: 'R4.3: The merge algebra, the stack fold and the Merged typestate'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T12:45:00Z'
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
- [ ] #1 Proptest laws pass: associativity and identity of Last and Replace, associativity and idempotence of OrderedSet (commutative as sets), associative composition of ListEdit, associativity of DeepMerge over kind-consistent trees (cargo test -p alz-core)
- [ ] #2 DeepMerge keeps the left fold: a later kind replaces an earlier one, and list items equal as ordered trees appear once (unit tests)
- [ ] #3 merge is a pure alz-core function from &ResolvedStack to Merging, Merged has no public constructor, and merging the same input twice gives equal results (unit test and proptest)
- [ ] #4 alz-core's clippy.toml bans std HashMap and HashSet, and mise run rust:check passes
<!-- AC:END -->
