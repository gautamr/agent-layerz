---
id: TASK-50.7
title: Verify the account facts in LAYERING-SPEC §12 items 10 to 21
status: Done
assignee:
  - '@claude'
created_date: '2026-11-22T02:17:00Z'
updated_date: '2026-11-22T02:58:00Z'
labels:
  - area:accounts
  - area:spec
  - spike
dependencies: []
parent_task_id: TASK-50
priority: high
ordinal: 24390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
LAYERING-SPEC §12 says items 10 to 21 must be recorded in docs/target-facts.md before TASK-50.2. They decide whether account selection isolates anything at all (item 12: is the login keyed by CLAUDE_CONFIG_DIR / CODEX_HOME on macOS and on Linux) and how the hook behaves (item 18: a missing folder). Verified on the installed Claude Code and Codex with the verify-target-fact skill; never with a real login and never by reading credentials.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/target-facts.md has a section for each of §12 items 10 to 21 with the tool version, the date, how it was checked (DOCS, SMOKE or SOURCE) and the result
- [x] #2 Facts that need a real login or a second machine are marked owner-only, with the exact commands for the owner to run and what to look for
- [x] #3 Any fact that contradicts §7.13 is listed with the spec change it needs, and the owner is told before TASK-50.2 starts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Branch spike/account-facts from main (done before any edit).
2. For each LAYERING-SPEC §12 item 10 to 21, gather evidence in this order: DOCS (code.claude.com, Codex docs), SOURCE (github.com/openai/codex), SMOKE (installed Claude Code 2.1.323, codex-cli 0.169.0, mise 2026.9.1) with CLAUDE_CONFIG_DIR / CODEX_HOME pointed only at fresh dirs under the session scratchpad. No login, no logout, no credential reads; commands run with a timeout and stdin from /dev/null.
3. Add docs/target-facts.md §12 (one table row or subsection per item, in the §10/§11 style): spec assumption, observed behaviour, status DOCS/SMOKE/SOURCE with version and date 2026-11-22, agreement with §7.13. Update the header date and versions.
4. Mark owner-only items (real login, second account, Linux) with exact commands and what to look for.
5. List every fact that contradicts or changes §7.13 with the spec change it needs (spec not edited in this task).
6. mise run lint; tick the ACs that were verified; append notes. No commit; task stays In Progress.
Untrusted inputs: none processed by alz code; the spike only reads vendor docs/source (treated as data) and tool output from scratch dirs.

7. (session 3) Apply decision-17, the C2 deferral and C1-C13 to LAYERING-SPEC §7.9, §7.13, §8.4, §9.1, §10, §12, §13 and to TASK-50, 50.2-50.6; run lint, tests, alz:check and the confidentiality grep.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (branch spike/account-facts, not committed): added docs/target-facts.md §12 for LAYERING-SPEC §12 items 10-21 (Claude Code 2.1.323, Codex 0.169.0 at tag rust-v0.169.0, mise 2026.9.1; DOCS/SOURCE/SMOKE per row). SMOKE used env -i, a fake HOME and the variable under a throwaway dir; no login, no credential read, no security command, Claude never run with CLAUDE_CONFIG_DIR unset.
Verified: 10 (folders, trust in .claude.json; 'read only at start' still unverified), 11, 12 (macOS DOCS/SOURCE; Linux unverified), 13 (no fallback, both SMOKE), 16, 17, 18, 19 (Claude only).
Unverified (stopped at the turn limit): 14 symlinks (method in the row), 15 Codex half, 19 Codex half, 20 mise decoding and direnv, 21 wrappers and Codex.
Contradictions with §7.13 listed as C1-C9 in target-facts §12.1. The main ones: C2, Codex MCP OAuth logins are shared across CODEX_HOME folders in the default auto keyring mode; C3, Claude Anthropic profiles and env credentials are shared across folders; C4, sharing skills/ and plugins/ also shares the claude.ai synced folders; C6, user or managed settings env can override CLAUDE_CONFIG_DIR. Owner-only checks O1-O7 are in §12.2.
New probe: the Claude init event and SessionStart hooks run without a login (updates target-facts §0.1 and §8 item 11).
Checks: mise run lint, exit 0 (biome check + tsc --noEmit).
AC #1 not ticked: items 14, 20 and 21 and the Codex halves of 15 and 19 have no result yet. AC #3 not ticked: the contradictions are listed, but the owner has not been told yet.

2026-11-22 owner decisions: C4 - alz account init does not share Claude skills/ or plugins/ by default and creates no symlinks for them; each account keeps real folders (claude.ai synced/ and .trash/ stay per account); --share skills and --share plugins remain opt-in. Personal skills reach alz repos through alz build at project scope. C2 (Codex MCP OAuth logins shared across CODEX_HOME by default) is deferred by the owner, to be decided later; record it as a known limit until then.

2026-11-22, session 2 (spike/account-facts, not committed). Finished in docs/target-facts.md §12: item 14 SMOKE for both tools (Claude 2.1.323: all six shared items work as links, and init, hooks, InstructionsLoaded, plugin marketplace add and plugin install wrote through them, keeping the links; Codex 0.169.0: AGENTS.md and skills links load, and skills/.system is written through the link; per-entry skill links load in both). Item 20: mise __MISE_DIFF SOURCE+SMOKE+DOCS (base64 no-pad, zlib, MessagePack {old,new,path}; present after hook-env, mise exec and mise run); direnv DIRENV_DIFF SOURCE v2.38.2 (base64url, zlib, JSON {p,n}). Item 15 Codex SOURCE (shell_environment_policy.set can set CODEX_HOME for shell commands, and project config may set it; hooks inherit the process env; stdio MCP servers do not). Item 19 Codex SOURCE (CODEX_THREAD_ID, CODEX_SESSION_ID, CODEX_VERSION, CODEX_TOOL_CALL_ID, CODEX_SANDBOX, CODEX_CI). Item 21 Claude DOCS (Bash deny is not a boundary), Codex partial SOURCE plus owner-only O6. Contradictions: C4 and C5 confirmed and extended; new C10 (plugin installPath names the installing account), C11 (mise/direnv detection, values hold secrets), C12 (Codex shell_environment_policy, MCP servers do not inherit), C13 (Bash deny is not a boundary). AC #1 ticked: every item has a row with version, date, status and result, or an owner-only check. Checks: mise run lint (exit code in the session report). Scratch dir deleted. AC #3 still open: the owner must be told about C1 to C13 before TASK-50.2.

Check result: mise run lint, exit 0 (biome check: 113 files, no fixes; tsc --noEmit clean).

2026-11-22 owner told about C1-C13. Decisions: (a) alz account init shares nothing by default: it only creates the account folders (0700) and every shared item, Claude agents, commands, CLAUDE.md, skills, plugins, settings.json, projects and Codex AGENTS.md, skills, sessions, memories, is opt-in with --share <item>; no symlink unless asked (amends D3 / decision-16). (b) Values set outside alz: the warning says 'set outside alz' and, when __MISE_DIFF or DIRENV_DIFF exists, adds 'mise or direnv is active here'; alz checks only that those variables exist and never reads or decodes their values (C11). (c) C2 deferred.

2026-11-22, session 3 (@architect, spike/account-facts, not committed): applied the owner decisions and C1-C13 to the spec and the board.
- LAYERING-SPEC: status line; §7.9 names Codex shell_environment_policy (set or exclude naming an account or XDG variable is a build error; C12a); §7.13 header cites decision-17 and target-facts §12; folders table and default-account note (C9, item 13); 'Both files' XDG check is presence-only (XDG_ variable away from its default while __MISE_DIFF or DIRENV_DIFF exists; decision-17); new 'Agent-session markers' (C7: six names, IDE terminals, no Codex marker in MCP servers); Limit rewritten (C13: deny rules only slow an agent down); fail closed records item 18 (C8: Claude creates the folder 0755 and starts logged out, Codex refuses; doctor warns about ~/.claude-alz-blocked); 'Values set outside alz' gives the 'set outside alz' and 'mise or direnv is active here' wording, never reading their values (D-b), plus limits C6 and C12a; new 'What a session passes on' (C12b: Codex MCP servers use the default account); shared items rewritten (D-a: nothing shared by default, --share for every item, per-entry Claude skills/ and plugins/ keeping synced/ and .trash/, limits C4, C5, C10, item 16, O7, untested projects/ and per-entry plugins/); C1 table cell; init safety; trust per account with the recorded locations; new 'Logins kept outside the account folder' (C3 limit; C2 known limit, no recommendation). §8.4, §9.1, §10 doctor and init rows, §12 items 10-21 (each says what is recorded and open, pointing to target-facts §12), §13 tests 19-24 and their intro (tests strip __MISE_DIFF, DIRENV_DIFF and markers).
- target-facts §12.1: status note on what was applied and decided (line 33 untouched).
- Board: TASK-50 description; TASK-50.2 AC 3, 6, 8; TASK-50.3 AC 5; TASK-50.4 AC 1, 2, 5; TASK-50.5 description, AC 2, 3, 4, new 6 and 7; TASK-50.6 AC 4; notes on each and a decision-17 comment on 50.2-50.6.
Checks: mise run lint -> exit 0 (biome 113 files, tsc clean); mise run test -> exit 0, 27 files, 177 passed, 1 skipped; mise run alz:check -> exit 0, 58 generated files match; confidentiality grep of all added lines (tracked diff plus the untracked TASK-50.7 and decision-17 files) -> only ~/.local/state hits, no names, emails, hosts or machine paths; git diff --stat -> 8 tracked files, 440 insertions, 110 deletions.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spike result: docs/target-facts.md §12 records LAYERING-SPEC §12 items 10-21 for Claude Code 2.1.323, Codex 0.169.0 (source tag rust-v0.169.0) and mise 2026.9.1 on macOS (DOCS, SOURCE and SMOKE per row; no login, no credential read), with contradictions C1-C13 in §12.1 and owner-only checks O1-O7 in §12.2. The owner was told and decided: decision-17 (init shares nothing by default, every shared item opt-in; mise/direnv detected only by the presence of their variables, never decoded), C2 deferred as a known limit. Follow-through in the same branch: LAYERING-SPEC §7.9, §7.13, §8.4, §9.1, §10, §12 and §13 and TASK-50, 50.2-50.6 aligned with the facts and decisions. Checks: lint, tests (177 passed, 1 skipped), alz:check and the confidentiality grep all pass. Not committed. Open: O1-O7 (owner, real login, second account or Linux); per-entry links in Claude plugins/ and a linked projects/ are untested (TASK-50.5 AC 7); owner questions on the XDG presence rule and Claude's 0755 folders are in the session report.
<!-- SECTION:FINAL_SUMMARY:END -->
