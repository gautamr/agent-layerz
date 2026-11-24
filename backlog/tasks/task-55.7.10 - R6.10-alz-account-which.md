---
id: TASK-55.7.10
title: 'R6.10: alz account which'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T13:14:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.2
  - TASK-55.7.9
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 169390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 `alz account which`, §7.13).
- cli.rs: `alz account which [--use <target>=<name>]...`; an unknown target, an invalid name, a target given twice or a positional argument is a usage error (exit 2); an unknown account subcommand exits 2, stripped.
- Flow: the §7.13 variables by value and the markers and mise or direnv by presence only, Home, Place (once), the root walk, the two files, the folder listing, the cached declaration only for an allowed root, the resolution, stdout lines and warnings on stderr. The command is handed no GitRunner, RepoRoot or StateDir.
- Rewrites test 24's escape case so the folder name comes from allowed.yaml (the Rust alz refuses the layer.yaml the old case wrote, §5.2).
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every case of tests 30 and 31, test 29's who-wins cases and test 24's three cases passes against ALZ_BIN, and their entries are gone from test/ci/r6-pending.json (vitest run; check-r6-pending exits 0)
- [x] #2 In an allowed repository alz account which runs no git and changes no file (test 31's gitProxy and snapshot cases pass against ALZ_BIN)
- [x] #3 An assert_cmd test shows exit codes 0, 1 and 2, and that warnings go to standard error only
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R6.10 (implementer, 2026-11-24), uncommitted on feat/rust-r6-task-55.7. Code: crates/alz cli.rs (account which [--use target=name]..., unknown target / invalid name / missing = / repeated target / positional / unknown subcommand exit 2), command/account/{facts.rs (gather: Home, AccountFiles, Place once, find_root, files, folder listing, env markers by value, mise/direnv by presence -> Facts), which.rs (resolve, which_report, stdout report, warnings on stderr, exit 0/1)}; alz-io CachedLayers (the Stack impl: reads the cache lazily and only when step 4 asks; holds no GitRunner, RepoRoot or StateDir), recorded_sources() via a Locator split out of CachedDeclarationReader (the 'source still in the stack' check needs no cache or lock), AccountFiles::{home,real_home,shown,in_repository}. Fixes to R6.8/R6.9 found by the acceptance run: lock_miss: any lock that does not parse is 'layer.lock cannot be read' (a short, upper case or empty sha, tests 30; LockShaInvalid stays for an entry that parses but has no usable sha); a working directory in a configured folder now fails closed for an allowed root that is the folder, above it or INSIDE it (test 31 'a parent of the repository'; this reverses R6.7 lead decision 1, which said a root strictly below is not matched: the working directory is known only as the folder, so a root inside it may be where the person is; the spec sentence for 7.13 should say so). AC1: acceptance 24 (279), 29 (59), 30 (74), 31 (113) = 525 pass against target/release/alz, and 468 pass + 57 expected fail against the reference; all entries of 24, 29, 30, 31 are gone from test/ci/r6-pending.json (3, 5, 63, 103) plus 11 of 32 that pass now (they only use account which): 55 -> 44. Full acceptance project with the JSON reporter: 1219 cases, 171 failing, 171 listed, check-r6-pending exit 0; floor raised 1216 -> 1219 (3 cases added). Remaining pending: 32 (44), 40 (34), 41 (77), 42 (16) = 171, owners R6.13/R6.14/R6.16 (alz account allow, the hint). AC2: crates/alz/tests/account_which.rs it_runs_no_git_and_changes_no_file_in_an_allowed_repository (stand-in git on PATH records no call, read_tree before/after equal, isolated HOME/XDG_*), and test 31's gitProxy and snapshot cases pass against ALZ_BIN. AC3: the same file: exit 0, 1 and 2 (six usage errors, empty stdout) and a_warning_goes_to_standard_error_only. Test edits (spec 5.2: a control character is refused in YAML after escapes, also in allowed.yaml and as C1; so none can reach the report through an account file): 24 hub-ESC case now reaches the folder through a link and asserts the report and no control character, plus referenceDrift(TASK-55.3) 'ignores an allowed.yaml whose source is an escape for a control character'; 31 'stale approval' case uses a quote in the root name, 'changed entry source' and 'ignored declaration' cases use U+202E/U+200F (format characters, stripped) with a referenceDrift(TASK-55.3) twin for the ESC escape; the link-account case uses the short key ~/link-account because a key prints cut after 64 code points. Checks: mise run rust:ci exit 0; mise run lint exit 0; mise run test exit 0 (1307 passed, 295 expected fail, 5 skipped). src/ unchanged. NOT done / open: the working-directory-once count is asserted by alz-io tests/account_walk.rs (the lookup closures count 1 and 1); the alz binary has no counting seam of its own, main captures once and gather places once; spec edits for 7.13 (configured-folder sentence above, lock sha reads as cannot be read) are not made.

Verified by the lead (2026-11-24): rust:ci and lint exit 0, src/ unchanged. Accepted: the ESC cases of tests 24 and 31 use stripped characters, with referenceDrift('TASK-55.3') twins for an ESC escape in allowed.yaml (refused as not valid YAML, decision-28); an allowed root inside a configured folder fails closed (reverses the R6.7 lead decision, test 31 requires it, stricter); a lock with a short, upper-case or empty sha is 'layer.lock cannot be read' (test 30). Pending 356 -> 171, floor 1219.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz account which reads the environment and places the working directory once, reads both account files, finds the root, lists account folders, reads the cached declaration lazily without git or writes, resolves and prints the §10.3 report (stdout) and warnings (stderr), exit 0 or 1, usage errors 2. Tests 24, 29, 30 and 31 pass against the Rust binary.
<!-- SECTION:FINAL_SUMMARY:END -->
