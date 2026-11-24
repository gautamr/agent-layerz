---
id: TASK-55.7.16
title: 'R6.16: R6 acceptance, parity and the drift list'
status: Done
assignee: []
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T14:36:00Z'
labels:
  - area:rust
  - area:test
  - needs:owner-approval
milestone: m-7
dependencies:
  - TASK-55.7.5
  - TASK-55.7.13
  - TASK-55.7.14
  - TASK-55.7.15
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 175390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (doc-5 R6 gate).
- Run the whole acceptance suite against the release binary with the JSON reporter; test/ci/r6-pending.json holds no entry. Per owner question 6, remove the file and the R6 allowance of test/ci/check-r6-pending.ts so the acceptance-rust job fails on any failing case (a CI change, by the release-engineer); raise case-floor.json.
- Record the counts per file in TASK-55.7's notes; complete §13's drift list; golden trees with identity lines.
- Carried criteria: TASK-72 #1, TASK-52 #4 and TASK-69 #5 (account forms), TASK-50.4 #5, TASK-73's escape case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 test/ci/r6-pending.json lists no case and every acceptance case passes against ALZ_BIN (JSON report run; counts per file in TASK-55.7's notes)
- [x] #2 Spec §13's drift list names every reference-drift case R6 added, each with a Rust case that passes (grep and vitest run)
- [x] #3 mise run rust:ci, mise run lint, mise run test and mise run alz:check exit 0 on the R6 branch
- [x] #4 TASK-72 #1 and the account forms of TASK-52 #4 and TASK-69 #5 are checked off with the evidence in their notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R6.5 (2026-11-24) for the spec: (1) drop '.git/config resolves outside the repository' from §9.4 and §10.3 and the Blocker enum — a link at .git/config is always 'not a regular file' now; (2) state that after a Refused stop (the place stopped being writable between two keys) the refusal line is printed, then the findings; (3) init skips the identity step whenever the build has any error (render-time ones included).

From R6.11 (2026-11-24) for the spec: §7.13 says that bytes typed without a newline and ended by Ctrl-D or Ctrl-C are the end of input, not an answer, so a name followed by Ctrl-D does not confirm (lead accepted, stricter). R6.13 renders §10.3's refusal from NotAPerson (markers, includes_claude_code()); R6.14's hint uses PersonAtTerminal::check(..).is_ok().

From R6.8 (2026-11-24) for the spec: (1) a cache entry another user owns, or any open error other than missing or link, is the miss unreadable; (2) cache-in-repo applies whatever sets the cache location; (3) an absent cache root reads as no entries (index-missing for a pinned layer, cache-entry-missing for a git layer); (4) a lock with a malformed sha is lock-sha-invalid, any other bad lock lock-unreadable (the reference reads the lock first: drift if a case shows it); (5) for an unpinned in-place layer a layer.yaml that is not a regular file is layer-invalid before the hash, in a cache entry a link there is a content-hash miss; (6) an index entry of the wrong length is index-invalid.

From R6.9 (2026-11-24), for the spec and drift pass: (1) reference drift: the reference prints the kind label of an ignored account declaration unstripped (its kind is '<kind>', not 'client'); the Rust alz strips it. (2) spec sentence to add to 7.13: a working directory inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names is placed by that folder's real path and nothing below it is looked at, so an allowed.yaml root strictly below that folder is not matched (stricter); the user config folders rule matches that real path too. (3) spec 7.13: invalid YAML in accounts.yaml or allowed.yaml is an ignored file with the reason 'it is not valid YAML' (account-file-ignored), so alz account allow refuses it as an ignored allowed.yaml, not as 'not a list'.

From R6.10 (2026-11-24) for the spec: (1) §7.13: a working directory in a configured folder is known only as that folder, and an allowed root that is the folder, above it or inside it fails closed (replaces R6.7's 'not matched' sentence); (2) §7.13: a lock whose sha is short, upper-case or empty is 'layer.lock cannot be read'; LockShaInvalid stays for an entry that parses without a usable sha.

R6.16 evidence (architect, 2026-11-24, branch feat/rust-r6-task-55.7 at b757f43). Spec (docs/LAYERING-SPEC.md, uncommitted, for the owner): s9.4 a link at .git/config is 'not a regular file', no real-path check; s10.3 'resolves outside' dropped from the reasons (named as reference-only); s10.3 a Refused stop between two keys prints the first key's line, the may-not-write line, then the findings; s10.3 init skips the identity step on any build error, render-time ones included; s7.12 the kind of an ignored declaration is stripped (drift note); s7.13 invalid YAML in an account file is 'it is not valid YAML' and allow refuses it as ignored; s7.13 the configured folder is known only as its real path, an allowed root that is it, above it or inside it fails closed, folders match that path; s7.13 a short, upper-case or empty lock sha is 'layer.lock cannot be read', lock-sha-invalid is an entry without a usable sha; s7.13 the cache misses of R6.8 (cache-in-repo whatever sets the location, absent root = no entries, unreadable entry, index-invalid, in-place layer.yaml not regular = layer-invalid before the hash, link at a cached layer.yaml = content-hash); s13 item 32 names test/ci/check-acceptance-report.ts. Already in the spec, skipped: the end-of-input rule and the allowed.yaml characters (R6.13). AC #2: drift cases added since R6 started (python script over git show ec7c190 vs HEAD, test/**/*.ts): 8 marks: 12 TASK-55.7 x2, 24 TASK-55.3, 26 TASK-55.3, 31 TASK-55.3 x2, 42 TASK-55.7 x2; none removed. s13 table now: TASK-55.3 files 24, 26, 29, 31, 34, 35 (allowed.yaml escapes, commit_identity before build --adopt writes); TASK-55.7 files 12, 42 (ALZ_ACCOUNT_DIRS, blocked folders for TASK-50.4, no note at $HOME or in the CLAUDE_CONFIG_DIR folder); rows that named cases with no mark corrected: TASK-52 30, 31 (R6), TASK-55.7 24, 26, TASK-73 32 (R6, a plain it in 40 the reference passes), TASK-50.4 12 (marked TASK-55.7); the pre-R6 TASK-55.6.7 row (file 39, 7 marks) added; a paragraph names the spec-only drift notes. Run: ALZ_BIN=target/release/alz vitest run --project acceptance -t 'reference drift' --reporter=json: 294 drift cases (loops expanded) pass, 0 fail, over files 04 12 20 22 24 26 28 29 31 33 34 35 36 37 38 39 42. Not checked: waits for the owner to accept the spec edits. Golden trees: crates/alz/tests/golden_trees.rs passes (cargo test -p alz --test golden_trees: 1 passed); tests/fixtures/golden/26-commit-identity/repo holds AGENTS.md with the Commits identity lines, stdout and stderr (the note [identity] outside git). Nothing missing for the plan; problem_lines compares only level and code, so identity wording is pinned by acceptance 26 and crates/alz/tests/identity_steps.rs, not by the goldens. Code left for the implementer: alz_core::identity::Blocker::ConfigOutside is never constructed (R6.5 said to drop it).

Owner approved the R6.16 spec changes 1-10 (2026-11-24): §9.4 a link at .git/config is not a regular file wherever it points; §10.3 drop 'resolves outside', the second-write output order, init skips the identity step on any build error; §7.12 the kind is stripped; §7.13 invalid YAML is 'it is not valid YAML', the configured-folder rule, the lock sha, the cache misses; §13 the renamed check.

R6.16 follow-ups (implementer, 2026-11-24, uncommitted on b757f43): (1) TASK-72 #1 / TASK-55.7.6 #1: seam gather_with(env, directory, choices, home) and which::run_at, new test which.rs tests::with_home_unset_and_no_user_database_entry_every_target_resolves_to_default passes (exit 0, one [account-home] warning, claude: default and codex: default on stdout; isolated scratch HOME-less env). (2) Blocker::ConfigOutside removed from alz-core and alz-io (never constructed). (3) TASK-55.7.1 AC #2: per-case list appended to TASK-55.7.1: 72 R1.6-pending cases, 62 with an acceptance twin (22 in 40, 21 in 41, 18 in 42 incl. 2 referenceDrift('TASK-55.7') expected failures on the TypeScript bundle, 1 in acceptance 32), 10 without (7 TS-only, 3 file-24 allow cases not ported), all 72 reference copies kept. Checks: mise run rust:ci exit 0; mise run lint ok; mise run test 56 files passed, 1307 passed | 295 expected fail | 5 skipped.

R6.16 (implementer, 2026-11-24): test/acceptance/43-untrusted-repo-terminal.test.ts adds 3 terminal cases (TASK-55.7.1 AC #2 list); case floor 1223; full acceptance against target/release/alz: 1223 cases, 0 failing, check-acceptance-report.ts exit 0; mise run rust:ci exit 0; mise run lint ok; mise run test 57 files, 1309 passed | 296 expected fail | 5 skipped.

Lead (2026-11-24): AC #1 full acceptance run 1223 cases, 0 failing, counts per file in TASK-55.7's notes; AC #2 the TASK-55.3 drift row now names file 43 (checked against every referenceDrift in test/acceptance: 24 26 29 31 34 35 43), owner approved the spec changes; AC #3 rust:ci, lint, test and alz:check exit 0 (a parallel-spawn descriptor leak in the pty-run tests on macOS was fixed by serialising the driver's start); AC #4 TASK-72 #1 proven by which::tests::with_home_unset_and_no_user_database_entry_every_target_resolves_to_default, TASK-52 #4 and TASK-69 #5 already checked with evidence (TASK-69 #5's 0xFF name runs only on Linux CI).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
R6's gate: every acceptance case passes against the Rust binary and the R6 pending list and its CI allowance are gone; the spec states R6's final points (owner approved) and the drift list names every R6 drift case; the carried criteria are proven.
<!-- SECTION:FINAL_SUMMARY:END -->
