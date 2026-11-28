---
id: TASK-114
title: Verify the Claude Code facts the autonomy mapping needs
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-27T11:51:00Z'
labels:
  - area:autonomy
  - area:facts
milestone: m-8
dependencies: []
references:
  - docs/target-facts.md
priority: high
ordinal: 215390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-7 §12 lists the Claude facts behind the autonomy mapping; most are UNVERIFIED. Adapters may write nothing on them until docs/target-facts.md records them. Use the verify-target-fact skill; never write to account folders.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/target-facts.md has a row with a status for each Claude item CF1, CF3, CF4, CF5, CF6, CF9, CF10 and CF12 of doc-7 §12 (grep the IDs)
- [x] #2 Each row says how it was checked (SMOKE command or DOCS URL), and the header date and Claude Code version are updated (grep)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Record the installed Claude Code version (claude --version).
2. DOCS: fetch code.claude.com/docs/en/permissions.md, permission-modes (or iam/settings) and the AskUserQuestion/tools reference; quote the lines for CF1, CF3, CF4, CF5, CF6, CF9, CF10, CF12.
3. SMOKE without login or model call, as in target-facts §12/§16: a throwaway project, HOME and CLAUDE_CONFIG_DIR set to empty throwaway folders (nothing under ~/.claude* read or written), claude -p hi --max-turns 1 --output-format stream-json --verbose. Read the init event (permissionMode, tools, agents) and startup messages for: project defaultMode values (CF1, CF12), whether a project defaultMode is ignored in an untrusted folder like allow rules (CF3), whether a bare deny AskUserQuestion removes the tool and whether it is listed in -p runs (CF9, CF10), and an agent with disallowedTools: AskUserQuestion is accepted.
4. Behaviour that needs a model turn (what a mode prompts for or refuses: CF1 behaviour, CF4, CF5, CF6, CF12 exit, CF10 in subagents) stays DOCS, with the reason; no login or account folder is used.
5. Add a new target-facts section for the doc-7 §12 Claude items, one row per ID with status and method; update the header date and version.
6. Checks: grep the IDs; cargo test -p alz-test-support --test spec_coverage and mise run rust:check (the facts harness parses target-facts).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-27, Claude Code 2.1.323 (claude --version), macOS Darwin 27.2.0.
DOCS fetched with curl the same day: permission-modes.md, permissions.md, sandboxing.md, tools-reference.md, sub-agents.md, headless.md, settings-reference.md (code.claude.com/docs/en/).
SMOKE without login or model call (HOME and CLAUDE_CONFIG_DIR = empty throwaway folders, nothing under ~/.claude* read or written; throwaway git project never trusted): claude -p hi --max-turns 1 --output-format stream-json --verbose, reading the init event and stderr.
- project defaultMode plan/default/acceptEdits/dontAsk: init permissionMode equals the value; none: default.
- defaultMode applies in the untrusted folder with no warning; an allow rule in the same file prints 'Ignoring 1 permissions.allow entry ... this workspace has not been trusted'.
- deny WebFetch removes WebFetch from tools; deny AskUserQuestion and NoSuchToolXyz: no warning, no change (AskUserQuestion is never listed in -p).
- tools in -p never include AskUserQuestion, EnterPlanMode, ExitPlanMode, in any mode, with or without --permission-prompts none.
- an agent with disallowedTools: AskUserQuestion loads.
Recorded as docs/target-facts.md §17; §17.2 lists what this changes for TASK-116. Header date and version updated.
Checks: grep -c -E '^\| CF(1|3|4|5|6|9|10|12) ' docs/target-facts.md = 9 rows (CF1 has two); mise run rust:check exit 0 (81 suites, 2412 passed, 0 failed); mise run alz:check passed.
Open: behaviour rows stay DOCS until a logged-in model turn is allowed; whether -p lists AskUserQuestion when logged in is open (CF10). The board has no In Review status, so the task stays In Progress until a fresh review; nothing committed yet (owner asked to hold commits).

2026-11-27, fresh-context review (reviewer agent): AC met, every DOCS quote found in the cited page; requested changes: rows that mixed -p SMOKE with interactive DOCS or inference were labelled SMOKE, and qualifiers the docs state were missing (VS Code never reads a project defaultMode, project value outranks ~/.claude/settings.json, cloud ignores dontAsk, plan lets the classifier approve commands when auto mode is available, forks keep AskUserQuestion, sandboxed commands that skip auto-allow, 'most' allow-approved calls). Fixed: §17.1 split into 13 rows by surface (CF1 x3, CF3 x2, CF10 x2, CF12 x2); CF3 (interactive) and CF10 (-p) are now UNVERIFIED; qualifiers added; §17.2 rewritten (VS Code, plan with auto mode, forks, cloud dontAsk); Host header updated. Re-checked: rust:check exit 0 (81 suites, 2412 passed, 0 failed), alz:check passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Recorded the eight Claude facts behind the doc-7 autonomy mapping (CF1, CF3, CF4, CF5, CF6, CF9, CF10, CF12) in docs/target-facts.md §17, one row each (CF1 split into takes-effect and behaviour), with status and method, on Claude Code 2.1.323. SMOKE without login shows a project defaultMode applies in -p, even in an untrusted folder; that plan starts in plan; that a bare deny removes a tool; and that -p lists no AskUserQuestion/EnterPlanMode/ExitPlanMode. What each mode prompts for or refuses (CF1 behaviour, CF4, CF5, CF6, leaving plan mode) is DOCS, because it needs a logged-in model turn, which was not done. Findings for TASK-116: write defaultMode at every level (interactive default is auto); acceptEdits also auto-approves rm/mv/sed in working folders; subagents never have AskUserQuestion; under dontAsk a content-scoped ask rule becomes a refusal. Checks: rust:check exit 0 (2412 passed), alz:check passed.
<!-- SECTION:FINAL_SUMMARY:END -->
