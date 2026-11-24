---
id: TASK-50.4
title: Report account problems in alz doctor
status: To Do
assignee: []
created_date: '2026-11-21T17:57:00Z'
updated_date: '2026-11-24T11:14:00Z'
labels:
  - area:accounts
  - area:cli
milestone: m-3
dependencies:
  - TASK-50.2
parent_task_id: TASK-50
priority: medium
ordinal: 20390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Covers the account folder checks. Workspace trust lives in per-account state (.claude.json, Codex config.toml) that alz never opens, so alz never determines whether a repository is trusted. TASK-22 (per-account trust) folds into this work as a static notice in alz doctor that trust is per account (LAYERING-SPEC §7.13 'Trust is per account', §9.1 'names only').
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor reports, per repository, a named account folder that does not exist and an account variable set outside alz, adding 'mise or direnv is active here' when __MISE_DIFF or DIRENV_DIFF exists, without ever reading their values (decision-17); it ignores the blocked folders ~/.claude-alz-blocked and ~/.codex-alz-blocked as accounts
- [ ] #2 It never reports an item as not linked, because alz account init shares nothing by default (decision-17); at most it prints one informational line per account folder naming the items that are links, found by lstat and readlink of the fixed item names only, and that line is not a warning and does not change the exit code (test: an account folder with no links gets no finding)
- [ ] #3 Every check is tested with a fake HOME and never opens a file inside an account folder
- [ ] #4 alz doctor prints a static notice that workspace trust is per account and never reads trust state (test: no file under the account folders or ~/.claude.json is opened)
- [ ] #5 alz doctor does not count ~/.claude-alz-blocked or ~/.codex-alz-blocked as account folders, but warns when one exists: Claude creates a missing folder (mode 0755) at a blocked start, and a login there would make it usable (LAYERING-SPEC §7.13, §12 item 18)
- [ ] #6 alz doctor warns when an account folder is readable or writable by group or others (Claude creates a missing folder with mode 0755), naming the folder by account name only
- [ ] #7 alz doctor prints a notice naming, never showing, the credential environment variables from LAYERING-SPEC §7.13 'Limit' (C3) that are set, since they apply to every account
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): revised to the owner's shared-items decision (config only by default; settings.json, projects/, sessions/, memories/ per account unless --share) and to the names-only rule in LAYERING-SPEC §9.1; the TASK-22 part becomes a static notice.

2026-11-22 (@architect, final pass): criterion 1 ignores the blocked folders (owner decision 7 in TASK-50.1).

2026-11-22 (@architect, from TASK-50.7 facts and decision-17): criterion 2 no longer reports shared items that are not linked, since nothing is shared by default; an informational line is the most it prints. Criterion 1 gives the presence-only mise/direnv wording (C11). Criterion 5 cites the item 18 fact (Claude creates the folder; Codex refuses to start and creates nothing). No doctor notice for C2 or C3 (C2 deferred by the owner; C3 is a spec limit only). Spec: LAYERING-SPEC §7.13, §10 doctor row, §13 test 23.

2026-11-22: paused by decision-21 (Rust port, doc-5). Built in Rust after cutover (TASK-55.9) against the same spec; the hook is built as a fast path from the start.

2026-11-24 (TASK-55.7.15): AC #5 is implemented in Rust on branch feat/rust-r6.15-doctor: doctor leaves ~/.claude-alz-blocked and ~/.codex-alz-blocked out of the account folders and prints one warning per existing blocked folder (lstat only, nothing opened). Evidence: crates/alz-core doctor unit tests, crates/alz-io/tests/account_names.rs, crates/alz/tests/doctor_live.rs, acceptance 12-account-independence (referenceDrift TASK-55.7). The AC stays unchecked here; TASK-50.4 remains in m-3 for the other criteria.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T02:56:00Z

---

decision-17 (accepted 2026-11-22, amends decision-16): alz account init shares nothing by default and every shared item is opt-in with --share; warnings about values set outside alz say only that mise or direnv is active, from the presence of __MISE_DIFF or DIRENV_DIFF, never decoding them. Owner decisions recorded in TASK-50.7.

---
<!-- COMMENTS:END -->
