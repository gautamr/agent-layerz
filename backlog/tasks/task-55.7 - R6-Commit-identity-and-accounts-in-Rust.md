---
id: TASK-55.7
title: 'R6: Commit identity and accounts in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T08:19:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6
  - TASK-72
parent_task_id: TASK-55
priority: medium
ordinal: 39390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R6: commit identity, alz account which and alz account allow, with the same security rules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests 26, 30, 31 and 32 pass against the Rust binary
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Scope
doc-5 R6: commit identity (§7.12 Commits section is R5's; R6 adds §9.4: setting it in `alz init` and `alz build --adopt`, the warning of `alz build`, the error of `alz check` unless `--skip-identity`, and `init`'s stack and declaration warnings after the scaffold), accounts (`alz account which`, `alz account allow` and `--remove`, the `note [account-allow]` hint of build and check, the allowed.yaml writer), doctor's blocked account folders (TASK-50.4 AC #5 only) and the ALZ_ACCOUNT_DIRS answer, TASK-72, and the account-resolution forms of TASK-52 #4 and TASK-69 #5. Out of scope: `alz run`, `alz use`, the shell hook, `alz account init` (TASK-50.3, 50.5, 50.6), the rest of TASK-50.4 (m-3). src/ is never changed (decision-30); the reference is read for behaviour only (decision-22). Subtasks R6.1 (pty driver, exists) to R6.16, one pull request, as in R5.

### What exists (main e40b2f8)
- alz-core: identity/commit_identity (validated name and email), merge/declarers (who may declare, the winner in Merged), account/{name, declaration, folders (account-folder relation), folder_key, user_config and allow_state (shapes only, and the allow_state text), protected, reading}, layer/scan with Purpose::Resolution budgets, cache/location, git::child_environment (source calls only).
- alz-io: EnvSnapshot, WorkingDirectory::capture, the component walk (R3.5), ReadOnlyDir, CacheDir, RepoRoot (writer, probes), GitRunner (source calls), pinned_tree writes the pathref@ index, account_names (doctor).
- alz: build, check (`--skip-identity` parsed, nothing to skip), init (scaffold only, no stack resolution after it), doctor; golden_trees.rs filters `[identity]` lines.
- Missing: every §9.4 effect, the cache-only declaration reader, the root walk, the two files' trust checks, the resolver, which/allow, the terminal, StateDir.

### Module layout (doc-6, decision-23)
alz-core (pure):
- identity/{effective.rs (Role, Field, GitIdent from `git var`), origin.rs (ConfigOrigin { scope, origin } parsed from `--show-scope --show-origin --get` with the value dropped at parse, so no report can echo it), finding.rs (Source = Environment | Config | EmailVariable | Guessed | Unknown), check.rs (IdentityCheck = Matches | Differs(findings) | NotARepository | Unavailable), setup.rs (SetupPlan: add a key only when .git/config has none), report.rs (lines per command; ShellSafe is the only way to a fix command; `~` for home; stripped origins)}; git/environment gains the §9.4 environment (inherited minus GIT_DIR, GIT_WORK_TREE, GIT_COMMON_DIR, GIT_CONFIG, GIT_INDEX_FILE; GIT_TERMINAL_PROMPT=0).
- account/{cached.rs (CachedDeclaration = None | Declared { map, layer, position, source, ref, revision } | Conflict | Miss(MissReason)), resolution.rs (steps 1 to 5, foreign values, Applied { Folder, Default, Kept(Foreign) }, FailsClosed reasons of doc-6 §3.2, what the hook applies, the fix), which_report.rs, allow.rs (Offer, AllowOutcome, confirmation, the next AllowState), hint.rs (the account-allow note)}.
alz-io (effects, capability types):
- git/repository.rs: RepositoryGit, the §9.4 calls only (keys a closed enum, values only IdentityName and EmailAddress, `--end-of-options`, LC_ALL=C where the message is read). fs/repo_root gains git_config_place() -> GitConfigPlace = Writable | NotTopLevel | GitNotADirectory | ConfigNotRegular | ConfigOutside, by no-follow probes.
- account/{home.rs (HOME as given; unset: std::env::home_dir's user database lookup; none: not absolute; TASK-72), files.rs (AccountFile<Located> -> Trusted | Ignored(reason): the "Both files" checks, capped no-follow O_NONBLOCK reads with fstat), place.rs (WorkingDirectory::resolve(self, &AccountFolders) -> Place, consumed once), root_walk.rs (decision-19), listing.rs (one listing of $HOME; alz-blocked never exists), cached_reader.rs (CachedDeclarationReader: read-only cache view and ReadOnlyDir, no GitRunner, RepoRoot or StateDir)}.
- terminal.rs: PersonAtTerminal, the proof value (isatty of stdin and no agent-session marker, checked before any file system call); the only reader of standard input.
- fs/state_dir.rs: StateDir, the only writer outside the repository besides the cache: allowed.yaml, its lock file and temporary file, never inside the repository, an account folder or a git work tree.
alz (wiring): command/account/{which,allow,remove}.rs, command/identity.rs (shared by init, build, check), command/account_hint.rs; cli.rs gains `account which [--use <target>=<name>]...` and `account allow [--remove]`.

### Command flows
- init (repo kind): scaffold as now; then resolve the stack as build does (fetching); stack or declaration problems are warnings, nothing more is written, exit 0; a refused account declaration also skips the identity step; with a declaration: RepoRoot::git_config_place, SetupPlan, RepositoryGit writes the missing keys, re-read, report.
- build: unchanged up to the summary; then the identity check as a warning (no declaration: no git command, no line); then the hint. build --adopt: also sets the identity as init does.
- check: comparison first; identity as an error unless `--skip-identity`; the hint only when the comparison passed, whatever the identity result.
- account which: env (the §7.13 variables by value, markers and mise/direnv by presence), Home, Place (once), root walk, the two files, the folder listing, the cached declaration only for an allowed root, resolve, print; exit 0 or 1, 2 for usage. It is handed no GitRunner, RepoRoot or StateDir, so it cannot fetch, run git or write.
- account allow: clap rejects every option but --remove (exit 2); PersonAtTerminal before any file system call; StateDir check; the declaration read with the reader resolution uses; AlreadyAllowed, NothingDeclared, CacheMiss, Conflict, Reserved or the Offer (repository text escaped, class 2); one prompt per distinct name in target order; read again after the last answer; StateDir writes under the lock. --remove reads no stack.

### Git identity through git
Reads: `rev-parse --git-dir` (LC_ALL=C: "not a git repository" is a note, any other failure Unavailable), `rev-parse --show-toplevel` (must equal the root), `var GIT_AUTHOR_IDENT` and `GIT_COMMITTER_IDENT`, `config --show-scope --show-origin --get --end-of-options <key>` for author.*, committer.*, user.* in git's order, after the four GIT_* identity variables and before EMAIL. Writes: `config --file <root>/.git/config --get` then `config --file <root>/.git/config --end-of-options <key> <value>`, only for a missing key, only when the place is Writable; never --global or --system. git follows a link at the file it writes (target-facts §11, smoke-tested 2026-11-24), so the probe runs right before the write and again after it (owner question 1). Each written key is printed with the declared value and the layer.

### Parity and drift
No generated file changes in R6; the Commits section is R5's and stays byte-identical. Byte parity: allowed.yaml written for ordinary roots equals the reference's bytes (R6.12 samples); .git/config is written by git. Messages follow the spec (decision-26); R6.2 writes into the spec every line the suite asserts. Drift, each a spec sentence and a referenceDrift case: (1) roots with C1, U+2028 or U+2029 written readable (TASK-73); (2) the working directory resolved once on Linux too (Rust test, reference/31 asserts macOS only); (3) no lstat or readlink inside an account folder through a planted link (TASK-52); (4) HOME unset with no user entry fails closed (TASK-72); (5) blocked folders not counted by doctor; (6) ALZ_ACCOUNT_DIRS per owner question 5; (7) test 24's escape case rewritten so the folder name comes from allowed.yaml.

### Untrusted input handled
layer.yaml commit_identity and account (R2 types); git output (parsed, values dropped, origins stripped, home as ~); .git, .git/config and the root's place on disk (links, gitfiles, FIFOs); GIT_* and EMAIL values (never printed); the repository's layer.yaml files on the root walk; layer.lock (sha 40 or 64 lowercase hex, one entry); cache entries and the pathref@ index; accounts.yaml and allowed.yaml (could be planted: trust checks, caps, positions only in warnings; maps keyed by validated values only); HOME, XDG_*, CLAUDE_CONFIG_DIR, CODEX_HOME; marker and mise/direnv variables by presence only; directory names (stripped in diagnostics, escaped in the offer); the person's answer (never printed).

### Tests
Unit and property tests per module. Laws: no identity report holds a value that differs from the declaration; a fix command appears exactly for ShellSafe values; findings print in a fixed order; SetupPlan never names a present key; a named account that cannot apply never becomes the default or a newly declared account; first match wins and a foreign value replaces steps 3 to 5 only; no report line holds a control character or an invalid name; an ignored file contributes nothing; the walks never touch a name inside an account folder; StateDir never writes outside its place and a refused state changes no byte; twelve writers keep every entry. alz-io integration tests with real git and temporary HOMEs. The working directory is resolved once on every platform (a counting probe). The pty driver (R6.1) for allow, the prompt, Ctrl-C and the hint. The acceptance suite against ALZ_BIN.

### How test/ci/r6-pending.json reaches zero
256 entries now (24: 3, 26: 24, 28: 2, 29: 6, 30: 63, 31: 103, 32: 55). R6.1 adds the pty cases it brings back from reference/32 as new entries (they pass on the bundle, not yet on Rust) and raises case-floor.json. Each subtask deletes the entries its change makes pass, in the same change, because the check fails on a listed case that passes: R6.5 tests 26, 28 and 29's init case; R6.10 tests 24, 29's which cases, 30, 31; R6.13 test 32's allow cases and R6.1's pty cases; R6.14 the hint cases. R6.16 ends with no entry and, per owner question 6, removes the allowance so the acceptance-rust job fails on any failing case.

### Spec changes in the same PR (R6.2)
§9.4 and §10.3 (every identity line of init, build, check; which, allow, remove and hint lines), §10.1 codes (identity warning and note, account-config, account-allowed, account-allow, account at resolution), §7.13 and §9.1 (built in Rust; the home lookup; terminal and Ctrl-C), §9.4 Limit (the write race), §13 entries and drift list. CHANGELOG: allowed.yaml readable for every root, stricter refusals; minor bump at cutover (doc-5).

### Carried criteria
TASK-52 #4 and TASK-69 #5 (account forms, R6.7, R6.8), TASK-72 #1 (R6.6), TASK-73 escape case (R6.13), TASK-50.4 #5 (R6.15).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From R5.15 (implementer, 2026-11-24): the R6 remainder. test/ci/r6-pending.json lists 256 acceptance cases that fail against the Rust alz today, grouped by file: 24-untrusted-repo 3 (alz account which; the hub-folder escape case also needs a rewrite because the Rust alz refuses a layer.yaml with a control character after escapes, spec 5.2, so the folder name must come from allowed.yaml); 26-commit-identity 24 (commit identity setup by init and build --adopt, identity warnings, the identity part of check, schema rows; spec 7.12, 9.4); 28-cache-integrity 2 (init sets the git identity from a cached layer, spec 9.4); 29-account-layer 6 (alz account which and the account warnings of init, spec 7.13); 30-account-cache-read 63 (account resolution through alz account which and the user config); 31-account-which 103 (alz account which and alz account allow, spec 7.13, 9.1); 32-account-allow 55 (alz account allow and the allow hint of build, spec 7.13). R6 deletes each entry from test/ci/r6-pending.json as its case passes; test/ci/check-r6-pending.ts (the acceptance-rust CI job) fails on any other failing case and on a listed case that now passes. Also for R6: the escaped C1/U+2028/U+2029 bytes of spec 10.2 are reachable through an allowed.yaml root path; add an acceptance case with referenceDrift TASK-73 when alz account allow exists.

From PR #14's ubuntu run (2026-11-24): the TypeScript reference resolves the working directory several times on Linux (test/reference/31 now asserts once on macOS only). R6's alz account which must resolve it once on every platform; assert it in an alz-io or alz integration test that counts the lookups.

Architect (2026-11-24, R6 plan). Subtasks R6.2 to R6.16 are TASK-55.7.2 to TASK-55.7.16, in dependency order, one pull request; R6.1 (TASK-55.7.1) is unchanged. R6.3, R6.6 and through them R6.7 to R6.9 and R6.12 can start now; R6.2, R6.4, R6.11, R6.15 and R6.16 wait for the owner answers below (labelled needs:owner-approval). New target facts the plan relies on: target-facts §11, three rows added 2026-11-24 (git follows a link at the file `git config --file` writes; `--end-of-options` with git config; a missing --file reads as unset; GIT_CONFIG_GLOBAL is scope global; useConfigOnly makes git var exit 128). These rows are uncommitted on this branch and land with R6.2.

Owner questions for R6 (options, recommendation first):
1. Writing .git/config (§9.4). git follows a link at the file it writes, so a link put in place between alz's check and git's write is followed. (A) Recommended: git config --file after alz's no-follow probe, probe again right after the write (same device and inode, still regular) and report a change; a §9.4 Limit sentence says the window is narrowed, not closed, as for the R5 writer. (B) alz writes .git/config itself through RepoRoot (temporary file and renameat) with its own small git-config writer, reading values through git: no window, but alz then writes git's syntax. (C) A, and also refuse when .git or .git/config is writable by group or others.
2. Environment of the identity calls. (A) Recommended: the inherited environment minus the five variables §9.4 names, plus GIT_TERMINAL_PROMPT=0 (what git would use for a commit, as the reference does). (B) Also drop the other repository variables the source calls drop (GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_NAMESPACE, GIT_CEILING_DIRECTORIES); this can make alz's answer differ from the person's git.
3. Home with HOME unset (TASK-72). The spec says getpwuid_r of geteuid(); std::env::home_dir uses getuid() (the same id unless alz runs set-user-id). (A) Recommended: std::env::home_dir only when HOME is unset (alz reads HOME itself first, so an empty HOME stays 'not absolute'); amend the spec to the real user id; no new crate. (B) A new crate for a geteuid lookup (nix or uzers), which needs an intake decision.
4. Ctrl-C at the allow prompt (spec: 'not confirmed; nothing changed', exit 1). (A) Recommended: rustix termios: keep the kernel's line editing and echo, switch ISIG off and make Ctrl-C a line end (VEOL) for the prompt only, restore the settings on every path; adds the rustix termios feature (amend decision-27's feature list). (B) The signal-hook crate for SIGINT (new dependency, decision). (C) Leave SIGINT alone: the process ends by the signal with no message; a spec change.
5. ALZ_ACCOUNT_DIRS, which the reference's doctor reads and the spec does not name. (A) Recommended: not read; the spec says doctor lists CLAUDE_CONFIG_DIR, ~/.claude and ~/.claude-* only; reference drift. (B) Port it and write it into the spec.
6. When test/ci/r6-pending.json is empty. (A) Recommended: R6.16 deletes it and the R6 allowance of check-r6-pending.ts, so the acceptance-rust job fails on any failing case (CI change by the release-engineer). (B) Keep the mechanism with an empty list for R7 and R8.
7. R6.1's driver (open since TASK-55.2). (a) Recommended: a Rust pty driver in alz-test-support with rustix's pty feature; needs a decision that lets alz-test-support depend on rustix (deny.toml wrappers). (b) node-pty, (c) python3's pty module, (d) no pty: those cases stay reference-only.
8. Doctor and TASK-50.4 (m-3). (A) Recommended: R6 does only AC #5 (blocked folders not counted, one warning), the rest stays in m-3. (B) All of TASK-50.4 in R6 (new behaviour before cutover, against doc-5 principle 2). (C) None; doctor keeps listing the blocked folders until m-3.
9. The cutover PR description. (A) Recommended: the owner adds to TASK-55.9 a criterion that the cutover pull request describes every security fix it ships; the details are in the handback, not here. (B) Leave TASK-55.9 as it is.

Owner answers (2026-11-24): (1) the identity write uses git config --file .git/config through the git runner, with a no-follow check of .git and .git/config right before and right after the write, and a spec Limit sentence for the window that remains (R6.4); (4) Ctrl-C at the allow prompt: rustix termios, the interrupt key off while reading so Ctrl-C answers no, settings restored (amend decision-27's rustix features; R6.11); (7) R6.1's pty driver is Rust in alz-test-support on rustix's pty feature, dev only (a decision and a deny.toml change). Lead decisions: (2) the identity git calls drop every repository variable, as decision-24 does for the other git calls, not only the five §9.4 names; (3) home when HOME is unset: std::env::home_dir, the spec says the real user id, no new crate; (5) ALZ_ACCOUNT_DIRS is not read (reference drift); (6) when test/ci/r6-pending.json is empty, R6.16 deletes it and the exception so the job fails on any failing case; (8) only TASK-50.4's AC #5 is in R6; (9) TASK-55.9 gets a criterion that the cutover PR describes every security fix it ships.
<!-- SECTION:NOTES:END -->
