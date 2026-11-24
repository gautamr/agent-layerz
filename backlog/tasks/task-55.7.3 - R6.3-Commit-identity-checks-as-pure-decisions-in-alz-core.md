---
id: TASK-55.7.3
title: 'R6.3: Commit identity checks as pure decisions in alz-core'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:39:00Z'
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
- [x] #1 Property: for any configured or environment value that differs from the declaration, no report line contains it (cargo test, proptest)
- [x] #2 Property: a fix command is printed exactly when the declared value is ShellSafe, and findings print name before email and author before committer whatever order they came in (proptest)
- [x] #3 Property: a SetupPlan never names a key that .git/config already holds, and the git var and origin parsers never panic on arbitrary bytes (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-24). Files: crates/alz-core/src/identity.rs plus identity/{slot,effective,origin,finding,check,setup,shell_safe,declared,report,tests}.rs. Types: Role, Field, Slot (orders field first, then role), ConfigKey; Effective::compare(git var output, declared) keeps two verdicts and drops the value at parse; ConfigOrigin::parse drops the third field; ConfigAnswer, Presence (environment by presence only), Source, Finding, locate (lazy git config lookups through a closure); RepoState::classify, Verdicts, IdentityCheck::decide, Command::level; KeyState, SetupPlan, GitConfigPlace/Blocker, SetupDecision; ShellSafe (letters, marks, digits, space . - _ @ +; the circled-letter symbols are excluded), FixCommand; DeclaredBy/DeclaredIdentity::from_winner; Report (check_lines, finding_lines, blocked_line, written_line, kept_line, unreadable_line, write_failed_line), Home. AC1: proptest identity::tests::no_report_line_holds_a_value_git_reported (wrong name, email and config value across all scopes, env presence, 3 commands, with and without fix; also checks the Debug of the check) and origin::the_value_never_appears_in_the_parsed_origin. AC2: a_fix_command_is_printed_exactly_when_the_declared_value_is_shell_safe (command line iff ShellSafe, else the sentence), findings_print_name_before_email_and_author_before_committer_whatever_the_input_order (shuffled, repeated), shell_safe proptests (no hostile character in a ShellSafe; one hostile character refuses). AC3: a_setup_plan_adds_exactly_the_keys_that_are_missing, the_parsers_are_total_over_arbitrary_bytes (Effective, ConfigOrigin, ConfigAnswer, KeyState, RepoState), origin parse proptest. Parity: ran the reference bundle (esbuild into the scratchpad, src/ untouched) against the 26 fixture for: global-scope name and email, GIT_AUTHOR_NAME plus EMAIL, command scope, local different key (kept line), linked worktree blocker, not top level, not a repository, broken config (unavailable); the Rust lines equal the reference's text, with one deliberate difference: the blocker line prints the by-hand commands after 'Run by hand: ' on ONE line separated by '; ' (the reference prints them on indented lines; Message strips line feeds, one-line rule of spec 10.1). Checks: cargo clippy -p alz-core --all-targets -D warnings clean; mise run rust:ci exit 0 (fmt, clippy, tests, doc, msrv, deny, audit); mise run lint exit 0. Spec needs for the architect (R6.2): (1) blocked-line commands on one line separated by '; '; (2) GitConfigPlace is defined in alz-core as Writable | Blocked(Blocker) with Blocker = NotTopLevel | GitNotADirectory | ConfigNotRegular | ConfigOutside | Uninspectable, so R6.4 returns it; (3) git var output that is not valid UTF-8 counts as differing in both parts; (4) the init line 'warning [identity] <reason>, so alz did not write ...' text is unchanged from the reference. ACs left unchecked as instructed.

Verified by the lead (2026-11-24): fmt and clippy -D warnings clean, alz-core tests pass, src/ unchanged. Deliberate difference: the blocker line's by-hand commands on one line (passed to R6.2 for the drift list).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz-core decides commit identity purely: git's output is parsed into findings with the reported value dropped, the check and the setup plan add only what is missing, the report lines match the reference, and a fix command is printed only for a ShellSafe value.
<!-- SECTION:FINAL_SUMMARY:END -->
