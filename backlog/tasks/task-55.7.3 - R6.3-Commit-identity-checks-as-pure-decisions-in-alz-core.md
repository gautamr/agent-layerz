---
id: TASK-55.7.3
title: 'R6.3: Commit identity checks as pure decisions in alz-core'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:20:00Z'
labels:
  - area:rust
  - area:identity
milestone: m-7
dependencies: []
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 162390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.4, doc-6 §3.1 and §3.2).
- identity/effective.rs: Role (Author, Committer), Field (Name, Email), GitIdent parsed from `git var` output (`Name <email> <epoch> <tz>`), total over any bytes.
- identity/origin.rs: ConfigOrigin { scope, origin } from `--show-scope --show-origin --get`; the value is dropped at parse, so no later code holds a wrong value. Scope = System | Global | Local | Worktree | Command | Environment, else Unknown.
- identity/finding.rs: Source = Environment(variable) | Config(key, origin) | EmailVariable | Guessed | Unknown, in git's order.
- identity/check.rs: IdentityCheck = Matches | Differs(findings) | NotARepository | Unavailable; the level per command (build warning, check error, init warning).
- identity/setup.rs: SetupPlan from what .git/config holds per key: add a missing key; leave an equal one; report a different one; never replace.
- identity/report.rs: the lines of R6.2; ShellSafe (letters, marks, digits, space and . - _ @ +) is the only way to a fix command; home as ~; origins stripped.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Property: for any configured or environment value that differs from the declaration, no report line contains it (cargo test, proptest)
- [ ] #2 Property: a fix command is printed exactly when the declared value is ShellSafe, and findings print name before email and author before committer whatever order they came in (proptest)
- [ ] #3 Property: a SetupPlan never names a key that .git/config already holds, and the git var and origin parsers never panic on arbitrary bytes (proptest)
<!-- AC:END -->
