---
id: TASK-55.7.5
title: 'R6.5: Identity steps of init, build, build --adopt and check'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T10:51:00Z'
labels:
  - area:rust
  - area:identity
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.2
  - TASK-55.7.4
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 164390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.4, §10, §10.3).
- init (repo kind): after the scaffold, resolve the stack as build does (fetching); stack and declaration problems are warnings, nothing more is written, exit 0; a refused account declaration also skips the identity step; with a declaration, set the identity (R6.4) and report.
- build warns about a mismatch; build --adopt also sets the identity; check reports it as an error after its own comparison unless --skip-identity.
- Outside a git repository a note; git missing or broken: check error, build and init warning. No declaration: no git command and no line.
- golden_trees.rs stops leaving out the [identity] lines.
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every case of tests 26 and 28 and test 29's init case passes against ALZ_BIN, and their entries are gone from test/ci/r6-pending.json (vitest run; check-r6-pending exits 0)
- [x] #2 With no commit_identity declared, init, build and check run no git command for the identity and print no identity line (acceptance case with gitProxy, or an alz integration test)
- [x] #3 golden_trees.rs compares the [identity] lines by level and code for every fixture (cargo test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R6.4 (2026-11-24): turn RepositoryGit's SetupResult (Blocked, Planned { plan, written, stopped }), Stopped (Refused, Tampered { field, changed }) and the IdentityCheck pieces into the report lines of §10.3; Tampered is error [identity] and stops init and build --adopt with exit 1.

R6.5 (implementer, 2026-11-24). Wired the identity step: crates/alz/src/command/identity.rs (Step: check for build and check, set_up for init and build --adopt; declared_of takes the merged model's winning declarer and its source), init.rs (resolves the stack with pipeline::compute after the scaffold; a Code::Source error gives the note, other errors become warnings through the new Diagnostics::errors_as_warnings, a refused declaration leaves no computed build so no step; exit 1 only for a failed check after a write), build.rs (step after the summary), check.rs (identity as errors after the comparison, --skip-identity skips it with no git call, last line 'check failed: the git identity differs...' when only the identity failed). AC1: acceptance 26, 28 and 29 against target/release/alz: all 24+2 cases and 29's init case pass; deleted from test/ci/r6-pending.json 24 (26), 2 (28), 1 (29's init case); 256 -> 229 entries; full acceptance project with JSON reporter: 1083 cases (floor 1083 kept), 229 failing, 229 listed, check-r6-pending exit 0. 3 build --adopt schema cases of 26 (control, C1, line separator in the name) were rewritten in test/acceptance/26 the way the lint cases already are: the Rust alz reports a decoded control character as error [yaml] (spec 7.13), so they are now referenceDrift TASK-55.3 cases; their old names left the pending list. AC2: crates/alz/tests/identity_steps.rs 'without_a_declared_identity_init_build_and_check_run_no_git_and_print_no_identity_line' (stand-in git on PATH records every call; none) and 'skip_identity_passes_and_starts_no_git'. AC3: golden_trees.rs no longer filters [identity] lines (README line removed); passes (the fixture with a declaration prints the not-a-repository note). Other tests: identity_steps.rs has 15 cases with real git, isolated HOME, GIT_CONFIG_GLOBAL, GIT_CONFIG_NOSYSTEM (build warns, adopt writes and check then passes, a different local value kept, env variable named without value, not a repository, config link refused, link swapped during the write = error and exit 1, check fails, check with a failed comparison, git failing, init set / note / ineligible), each asserting the wrong global value never appears. init.rs tests updated for the new init stderr. Changed in alz-io: a link at .git/config is always Blocker::ConfigNotRegular (test 26 'does not write through a .git/config that is a symlink' expects 'is not a regular file' for a link with an absolute target; R6.4 had reported ConfigOutside); Blocker::ConfigOutside is now unreachable from alz-io. Checks: mise run rust:ci exit 0; mise run lint ok; mise run test 53 files passed, 1178 passed, 288 expected fail. Open: spec 9.4/10.3 still list 'resolves outside the repository' (unreachable now: drop it or give it a case); the post-write re-read after a refused-before-write stop prints findings (spec silent).

Verified by the lead (2026-11-24): mise run rust:ci and lint exit 0, src/ unchanged. Accepted: test 26's three control-character schema cases are referenceDrift('TASK-55.3') expecting error [yaml] (spec §7.13); a link at .git/config is always 'not a regular file' (test 26, reference).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The identity step runs in init, build, build --adopt and check as §10.3 orders it: build warns, check fails, init and --adopt add only what is missing between no-follow checks, a change caught after a write is error [identity] with exit 1; no git runs without a declared identity or with --skip-identity. 27 R6 cases now pass (256 to 229 pending).
<!-- SECTION:FINAL_SUMMARY:END -->
