---
id: TASK-55.7.2
title: 'R6.2: Spec for the identity steps, account which and allow, and the R6 codes'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-24T08:15:00Z'
updated_date: '2026-11-24T08:46:00Z'
labels:
  - area:rust
  - area:docs
  - needs:owner-approval
milestone: m-7
dependencies: []
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 161390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (like R5.1). Write into the spec every line the acceptance suite asserts for R6, with its stream, level and code, so the Rust messages follow the spec (decision-26):
- §9.4 and §10.3: the identity steps of init, build, build --adopt and check (note outside git, warning or error per command, the finding lines, the fix commands, the `git init` and top-level texts, the written-key line).
- §10.3: `alz account which` (repository line, one line per target with its source, each skipped step, the two file paths, stale approvals, fails-closed lines and fixes), `alz account allow` and `--remove` (offer, prompts, `already allowed`, `nothing to remove`, `not confirmed; nothing changed`, refusals), and the `note [account-allow]` forms.
- §10.1 codes: identity as warning and note, account-config, account-allowed, account-allow, account at resolution.
- The owner answers to TASK-55.7's R6 questions as spec text (the .git/config write race, the identity environment, the home lookup, Ctrl-C at the prompt, ALZ_ACCOUNT_DIRS, the blocked folders).
- §7.13 and §9.1 no longer say these parts are built in TypeScript only; §13 entries and the drift list for R6.
- CHANGELOG entry: no generated file changes; allowed.yaml readable for every root; stricter refusals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Spec §9.4, §10.1 and §10.3 state every identity, account which, account allow and hint line that tests 24, 26 and 28 to 32 assert, with stream, level and code (spec diff; a grep of each asserted string finds it in the spec)
- [x] #2 The owner's answers to the R6 questions in TASK-55.7 are spec text, and §13's drift list names every R6 reference-drift case (spec diff)
- [x] #3 CHANGELOG has the user-facing R6 entry and its version note (diff)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-24, R6.2 spec pass). Edits are uncommitted on feat/rust-r6-task-55.7 (docs/LAYERING-SPEC.md, CHANGELOG.md, decision-27); they land with the R6 pull request.
Spec changes:
- 7.13: the Rust alz builds the same parts in R6 (intro). The account warnings use the reference's codes (lead decision): account-config-shape, -config-key, -config-default-name, -default-name-folder, -config-folder, -config-duplicate, account-allowed-shape, -allowed-entry, account-file-ignored, -xdg-relative, -file-in-work-tree, account-home; an entry named by its position from 1, a folders key only quoted. A folders key is ignored inside an account folder, inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names, or at or above $HOME (as written and once real); an allowed root only inside an account folder or at or above $HOME, and kept inside a configured folder. The owner checks of both files use the effective user id, as the cache does; only the home lookup uses the real user id. The terminal module is named for both implementations. Ctrl-C at the prompt: interrupt key off while reading, it ends the line and answers no, settings restored on every path, a terminal whose settings cannot be read or set is asked nothing (owner answer 4). HOME unset: the real user id as std::env::home_dir; an entry whose home is not absolute counts as none (lead decision 3).
- 9.1: account selection is built; the allowed.yaml writer is named for both implementations.
- 9.4: git var output that is not valid UTF-8 differs in both parts, a git config --get value that is not valid UTF-8 is a different value; git config --file .git/config --end-of-options through the git runner; no-follow checks of .git and .git/config right before and right after each write (.git the same device and inode, .git/config regular, no link, inside the repo); a failed check after a write is error [identity], exit 1, also in init and build --adopt, no value printed (lead decision); a Limit bullet for the window left (owner answer 1); the identity calls drop every repository variable of 6 Environment plus GIT_CONFIG and keep the -c, GIT_CONFIG_GLOBAL/SYSTEM/NOSYSTEM, identity and EMAIL variables; LC_ALL=C for rev-parse --git-dir (lead decision 2).
- 10.1: identity as error, warning or note; account also a warning in init; the identity row names the after-write error; rows account-allow, account-declaration-ignored, account-changed, account-foreign, account-manifest and one row per account file and config code.
- 10.3: check's order and its identity failure line; doctor: the blocked folders are not accounts and get one warning each, ALZ_ACCOUNT_DIRS not read (reference drift, lead decision 5); new paragraphs: the commit identity step (blocker reasons are the reference's texts; the by-hand steps on one line after 'Run by hand: ' separated by '; ', reference drift; an unknown scope or failed get is 'a source git could not report'), alz account which (a file that is not valid YAML shows ' (ignored: it is not valid YAML)'), alz account allow and --remove, the account note.
- 13: item 32 (R6) and drift rows TASK-52 (30, 31, 36), TASK-55.7 (12, 24, 26), TASK-50.4 (12), TASK-73 (32, 38). TASK-72 and the working directory resolved once on Linux are Rust tests, named in item 32. R6.16 confirms the file numbers.
- CHANGELOG: the R6 entry under Unreleased/Changed, part of the minor bump at the cutover.
- decision-27: Amended 2026-11-24 (owner, R6) paragraphs in Decision (termios in alz-io; pty dev only in alz-test-support) and Consequences (deny.toml wrappers gain alz-test-support in TASK-55.7.1; a cargo tree record; prompt tests). Title and status unchanged.
- target-facts 11: the three TASK-55.7 rows were committed with the R6 plan in 3c6aa78; nothing left to commit.
- Inputs from R6.3 and R6.6 (coordinator, same day) are folded in above.
Evidence:
- mise run lint: exit 0 (biome, 192 files; tsc).
- Asserted-string check (scratch script over the toContain, toMatch, includes and toBe literals of tests 24, 26 and 28 to 32, backticks and whitespace normalised): 137 literals were missing from the spec before, 114 after. A template match of the 109 left against the spec's placeholder spans matched 77. The 37 left are not R6 lines (24 source, agent and protected-variable; 26 schema, Commits section, visibility; 28 cache), test data (names, emails, URLs, fixture text), or R6 instances whose placeholder comes first (a root before (layer.yaml is not valid), the markers before set, a map after now), which 10.3 states as templates.
- Leak grep of the added lines in docs, CHANGELOG, the decision and this task: no match.
Lead decisions (coordinator, same day): the after-write check is an error; the finer reference codes replace account-config and account-allowed (TASK-55.7.6 has the mapping in its notes). lint after these: exit 0. Owner: the five identity variables the reference removes become the full repository list.
<!-- SECTION:NOTES:END -->
