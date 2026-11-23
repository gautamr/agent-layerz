---
id: TASK-55.3.6
title: >-
  R2.6: The repo stack manifest: sources, refs, targets, workspace, budgets,
  options
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T05:18:00Z'
labels:
  - area:rust
  - area:resolve
milestone: m-7
dependencies:
  - TASK-55.3.5
parent_task_id: TASK-55.3
priority: high
type: task
ordinal: 95390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Spec §5.2, §6 and R2.1; fixes TASK-61 in the model. Modules `crates/alz-core/src/stack.rs` with `stack/{source,git_ref,manifest}.rs`, and `path.rs` with `SafeRelPath` (shared with R2.7 and alz-io).

Types:
- `GitRef`: no leading `-`, no `..`, only `[A-Za-z0-9._/+-]`; YAML text only, so `ref: [x]` and `ref: 1.10` are errors naming `stack[i].ref` (TASK-61), never printing the value.
- `Source = Path { dir } | Git { url: GitUrl, git_ref } | GitHub { owner, repo, git_ref }`, parsed from `path:`, `git:` and `github:`. `GitUrl` refuses credentials as §5.2 states (TASK-71's rule; its CLI cases need `build` and close in R3).
- `StackEntry { source, pin: Option<GitRef> }`, with R2.1's rule for which sources take `ref:`.
- `Manifest { stack: Stack<Declared>, targets: BTreeSet<TargetName> (default claude), workspace: Workspace { hub, add_dirs } (relative dirs, hub first, no duplicates), protected_paths, mcp_enable, budgets (whole numbers, spec defaults), options }`. `Stack<Declared>` is the first state of doc-6 §3.3; `Resolved` is R3's.
- `SafeRelPath`: `/`-separated, valid UTF-8, no empty, `.`, `..` or `.git` segment, no backslash, no NUL. Its `Ord` is `str`'s, which is spec §6 name order.

Laws (proptest): a valid source prints back to its input byte for byte and parses again to the same value (the lock and allowed.yaml store that text); `SafeRelPath` order equals the order of UTF-8 bytes and of code points (§6).

Untrusted input: the whole manifest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ref: [x], ref: 1.10, a ref starting with - and a ref holding .. are refused with an error that names the entry and not the value, and ref: "1.10" is accepted (unit tests, TASK-61 in the model)
- [ ] #2 Every source form of spec §5.2 parses to its variant and prints back byte for byte; credentialed git URLs are refused as §5.2 states, and a user name alone in ssh:// and the scp-like form is accepted (unit tests)
- [ ] #3 The name-order property test for SafeRelPath passes, and unknown fields, an empty stack and a file without stack are refused with the field path (unit tests)
<!-- AC:END -->
