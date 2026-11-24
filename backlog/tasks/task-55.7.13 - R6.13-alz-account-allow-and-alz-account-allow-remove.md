---
id: TASK-55.7.13
title: 'R6.13: alz account allow and alz account allow --remove'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T13:46:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.10
  - TASK-55.7.11
  - TASK-55.7.12
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 172390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 `alz account allow`, §7.13).
- alz-core account/allow.rs: from the declaration read with R6.8's reader (as resolution reads it), the allowed state and the folder listing: AlreadyAllowed | NothingDeclared (names --remove when this root has an entry) | CacheMiss (run alz build) | Conflict | Reserved | Ask(Offer). The Offer shows the repository, the account per target and whether its folder exists, the declaring layer and its position, source and ref as written, revision, the source to record and the current entry when it differs; repository text escaped (class 2).
- One prompt per distinct name in target order; an answer without its surrounding spaces and tabs must equal the name; after the last answer the declaration is read again and a changed map, source, revision or layer is refused; then StateDir writes.
- --remove reads no stack: shows the entry, asks for its names, removes it; with no file or entry, `nothing to remove`, exit 0, nothing created.
- clap accepts only --remove; --yes, -y or anything else is a usage error (exit 2). The gate (R6.11) runs before any file system call.
- Deletes the entries of the cases it makes pass from test/ci/r6-pending.json in the same change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every case of test 32 that allow or --remove reaches, and every pty case R6.1 brought back, passes against ALZ_BIN, and their entries are gone from test/ci/r6-pending.json (vitest run; check-r6-pending exits 0)
- [x] #2 Under the pty driver a repository whose real root holds U+0085, U+2028 and U+2029 is allowed, and alz account which then applies the layer's account (acceptance case, referenceDrift TASK-73)
- [x] #3 A declaration changed between the offer and the last answer is refused and allowed.yaml stays byte-identical (pty acceptance case)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-24), branch feat/rust-r6-task-55.7, uncommitted. alz-core: account/allow.rs (allowance(): AlreadyAllowed | NothingDeclared{has_entry} | CacheMiss | Conflict | Reserved | Ask(Offer); Offer::is_still_declared_by compares map, recorded source, revision, layer and position, not the file's entry), account/allow_text.rs (every line of section 10.3 for allow and --remove as Message; refusals; not_a_person from NotAPerson with markers by name and the CLAUDECODE line; prompt and offer quoted as class 2 Escaped), account/shower.rs (the ~ for HOME, moved out of which_report), Code::AccountAllow (same text as R6.14's), AccountName::default_account, AccountChoice::to_name. alz-io: StateDir::check_state() (reader-level refusals only; --remove runs it before looking for its entry, so a link to somebody else's file with no entry for this root is 'nothing to remove'), StateDir::check() now refuses a file the reader ignores with the reader's own reason first (acceptance 41: 'allowed.yaml is ignored (it lies inside a git work tree ...)'), IgnoredEntries text matches the spec. alz: cli.rs AllowArgs{--remove} (clap rejects --yes, -y, anything else with exit 2), command/account/allow.rs (gate before any file system call, HOME, repository, repair layer.yaml, StateDir::check, declaration, already allowed, offer on stdout, one prompt per distinct name on stderr, declaration read again with a fresh gather, StateDir::update, exit codes 0/1), Gathered gains files. AC1: acceptance 32 (44 entries), 40 (34), 41 (77) and 3 of the 16 in 42 pass against target/release/alz with pty-run; their entries are deleted from test/ci/r6-pending.json (44+34+77+3); the 13 left in 42 are R6.14's hint cases (owner: TASK-55.7.14, commit 803a33a on feat/rust-r6.14-hint, which also deletes entries: expect a merge conflict in r6-pending.json to resolve by deleting the union). Full acceptance project against the Rust binary: 1202 passed, 13 failed (all listed), 5 skipped; check-r6-pending.ts exit 0 (1220 cases, floor raised 1219 to 1220). AC2: acceptance case in 40 (plain it, not referenceDrift: the TypeScript reference passes it) allows a repository whose real root holds U+0085, U+2028, U+2029 under the pty driver and then alz account which applies the layer's account. This needed the reader of allowed.yaml alone to accept U+007F-U+009F, U+2028, U+2029 once decoded (yaml DecodedText::Paths, parse_paths, parse_path_file, used for Which::AllowState); the file still holds them only as escapes; any other control character still does not read back and allow refuses to write. Unit test in allow_state/tests.rs. AC3: acceptance 40 'refuses when the declaration changes while the person confirms' (map change; also layer.yaml unreadable; declaring layer dropped) and the byte-identical snapshot. Rust tests: crates/alz/tests/account_allow.rs (8 pty and pipe cases, isolated HOME, no XDG_*: remove asks per recorded name and removes, wrong answer, y, name then Ctrl-D, Ctrl-C, end of input each not confirmed and file unchanged, equal names asked once, no entry creates nothing, piped stdin refused, each marker with and without value refused by name only, a group-writable file refused before any question, usage errors exit 2); alz-core allow and allow_text unit tests; alz-io state_dir check_state test. test/acceptance/41: the usage-error case asserted 'usage: alz' text, now decision-26 form (exit 2, empty stdout, stderr not empty). Spec (docs/LAYERING-SPEC.md): section 7.13 Allow state says the reader accepts those characters in root and source; section 10.2's 'refuses to write' sentence now says allow writes them; section 7.13 prompt says text typed and ended by Ctrl-D or Ctrl-C is the end of input (R6.11's request). Checks: mise run rust:ci exit 0 (twice, last after the final edit), mise run lint exit 0, mise run test exit 0 (1308 passed, 295 expected fail, 5 skipped). Left: the 13 hint cases of test 42 (TASK-55.7.14); merge conflicts expected with 803a33a in account.rs (mod lists), facts.rs (Gathered: my files and its allow_problem) and r6-pending.json. Open: allow prints only the root walk warnings and the ignored-declaration warnings, not accounts.yaml's; a cargo-level test of the confirmed allow write (needs a cached layer) is acceptance 40's only.

Verified by the lead (2026-11-24): rust:ci and lint exit 0, src/ unchanged. AC #2's case is a plain it(), not referenceDrift: the reference passes it. Accepted spec change: allowed.yaml's reader alone accepts U+007F-U+009F, U+2028 and U+2029 once decoded (written only as escapes), so allow writes such a root as AC #2 requires; this replaces the R2 sentence that allow refuses it. Flag for the R6 security review and the PR. The 13 hint cases of test 42 go to R6.14's merge.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz account allow and --remove: refuse before asking when not a person at a terminal (reasons and markers by name), check the state first, ask for each name, read the declaration again after the last answer, write or remove through StateDir, print the §10.3 lines. Tests 32, 40 and 41 pass against the Rust binary; 158 entries leave the R6 list.
<!-- SECTION:FINAL_SUMMARY:END -->
