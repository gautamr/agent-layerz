---
id: TASK-50.11
title: Add alz account allow and alz account allow --remove
status: To Do
assignee: []
created_date: '2026-11-22T03:34:00Z'
labels:
  - area:accounts
  - area:cli
  - area:security
milestone: m-3
dependencies:
  - TASK-50.10
references:
  - docs/LAYERING-SPEC.md
  - .layer/instructions/50-repo.md
  - .layer/skills/threat-model/SKILL.md
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 28390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Fourth and last slice of TASK-50.2. The only command that writes outside the repository and the cache: alz account allow, which a person runs to accept the account a repository's layers name (owner approval in TASK-50.2's notes; decision-16), and alz account allow --remove. It also adds the allow hint to alz build and alz check, and updates the repository invariant and the threat model, because the invariant 'outside the repo, only the cache is written' stops being true with this slice.

Spec: LAYERING-SPEC §7.13 'Allow state', 'Both files', 'Trust and alz account allow', 'Agent-session markers', 'Limit', §9.1 'Planned writes outside the repo and the cache', §10 'alz account allow', §13 test 20. Facts: docs/target-facts.md §12 item 19 (markers), item 21 (deny rules are not a guard).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz account allow shows the account per target, the declaring layer, its source and its revision, and asks the person to type the account name to confirm; it refuses and changes nothing when stdin is not a TTY or inside an agent session (any of the six markers in LAYERING-SPEC §7.13 'Agent-session markers' set, even to an empty value), and it has no --yes; it holds an exclusive lock file next to allowed.yaml while reading and writing, keeps entries sorted by root and replaces the entry for the same root; alz account allow --remove removes this root's entry under the same lock file, with the same terminal and agent-session checks; on a cache miss allow changes nothing and says only to run alz build (a test for each refusal and each marker, for --remove and for two concurrent runs)
- [ ] #2 alz account allow records the repository's real root, the whole per-target map and the declaring layer's source (a remote URL without its ref, or a local layer's real path) in allowed.yaml, and only then does an account named by a layer take effect; alz build and alz check print the allow command only on a terminal with no agent-session marker set, and the hook and alz run never print it (tests: a fresh clone starts on the default account; after alz account allow it uses the named one; a changed map or source skips it again; a ref bump keeps it)
- [ ] #3 The repository invariant 'outside the repo, only the cache is written' in .layer/instructions/50-repo.md names allowed.yaml as the one other file alz writes, the threat-model skill's trust table gains rows for accounts.yaml and allowed.yaml, and mise run alz:build then mise run alz:check pass
<!-- AC:END -->
