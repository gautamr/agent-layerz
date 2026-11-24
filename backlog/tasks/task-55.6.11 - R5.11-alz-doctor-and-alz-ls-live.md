---
id: TASK-55.6.11
title: 'R5.11: alz doctor and alz ls --live'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T18:09:00Z'
updated_date: '2026-11-24T05:16:00Z'
labels:
  - area:rust
  - area:cli
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.10
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 143390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§8.4, §4.4, §10, §9.1).
- alz-io tool.rs: ToolRunner for the claude binary (ALZ_CLAUDE_BIN or claude): argument arrays, no shell, stdout and stderr capped. Version: `--version`, parsed and compared with the verified range in target/claude/facts.rs. Probe: the stream-json init event, parsed in alz-core probe.rs into agent, skill and MCP server names and statuses only.
- alz-io account_names.rs: for each Claude account folder (~/.claude, ~/.claude-*, CLAUDE_CONFIG_DIR) the names in agents/ and skills/ and whether CLAUDE.md exists; never opens a file there.
- doctor.rs (pure): collisions with built names, the version line, client root and ancestor CLAUDE.md, AGENTS.md and .git, workspace folders, then the build diagnostics; exit 1 on any problem.
- ls --live: errors for built items Claude did not load, warnings for MCP status and Claude's warnings, the extra items line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 alz doctor reports a user-level skill or agent with a built name as a collision and a CLAUDE.md at the client root as a warning, and gives identical output under two empty account folders (tests 05, 12 and 16 against ALZ_BIN)
- [x] #2 alz ls --live passes with the fake Claude that loads everything and fails naming the agent the other fake did not load (test 22 against ALZ_BIN)
- [x] #3 No file inside an account folder is opened by doctor (alz-io test with the folder's files at mode 000)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R5.11 implementation (not committed, ACs not checked).
Files: alz-core: doctor.rs (+doctor/tests.rs; pure Report from Facts), probe.rs (+probe/tests.rs; init event via json::parse, names/status only, stripped), live.rs (+live/tests.rs; Built and Comparison), target/claude/facts.rs (Version, VERIFIED_RANGE >=2.1.311 <2.2.0), diagnostic.rs (Code::Claude), lib.rs and claude.rs module lines. alz-io: tool.rs + tool/{environment,locate,pipes}.rs (ToolRunner: absolute path on validated PATH, argument array, env_clear + purpose allowlist, own process group killed at the deadline, caps 64 KiB/4 MiB stdout and 64 KiB stderr), account_names.rs (names only: read_dir, file_type, lstat), surroundings.rs (client root, workspace dirs, ancestors by stat). alz: command/doctor.rs, command/live_probe.rs, command/ls.rs (--live), cli.rs (Ls(LsArgs), Doctor), main.rs, pipeline.rs (Pipeline.manifest kept when the build fails). Spec: LAYERING-SPEC.md 10.3 (tool lookup, environment, deadlines, caps, probe failure text, version parse).
AC1: unit tests in alz-core doctor/tests.rs (collisions per kind, CLAUDE.md note, client root, identical report under two empty accounts); crates/alz/tests/doctor_live.rs (user skill collision exit 1, two empty accounts identical, client-root CLAUDE.md warning, version found/outside range/missing/silent/failing, hostile output and hostile folder name give no ESC byte). Acceptance against release binary: 05-skill-override 1/1, 12-account-independence 1 passed 1 skipped (smoke needs ALZ_SMOKE_CLAUDE), 16-hub 7/7.
AC2: doctor_live.rs ls_live_* (passes when all loaded; fails naming agent 'architect'; missing binary; no init event; hostile output; env check that GITHUB_TOKEN does not reach claude; exact argument list). Acceptance 22-ls 7/7 (25-cli 1/1). No remaining acceptance failure for these files.
AC3: alz-io tests/account_names.rs no_file_inside_an_account_folder_is_opened (agents/*.md, skills/*/SKILL.md, CLAUDE.md, .credentials.json, ~/.claude.json all at mode 000; names and CLAUDE.md presence still found; skipped as root).
ToolRunner: alz-io tests/tool_runner.rs (found, missing, not executable, relative PATH entries ignored, deadline kills grandchild, caps, environment allowlist per purpose, cwd and null stdin). The slow-past-deadline case is covered only at alz-io level (deadline settable there); a CLI-level case would take the 10 s version deadline, so there is none.
Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint ok; mise run test: 50 files, 1191 passed, 210 expected fail, 5 skipped.
Open / spec needs: (1) environment allowlist for the probe (ANTHROPIC_*, CLAUDE_CODE_*, proxy, cert vars) is a new choice, now written in spec 10.3, owner may want it reviewed (cloud-provider variables such as AWS_* or GOOGLE_* are not passed; a Bedrock or Vertex login would need them). (2) doctor ignores ALZ_ACCOUNT_DIRS that the reference reads (spec 10.3 does not list it); the blocked account folders are not excluded (R6, TASK-50.4). (3) Version output without x.y.z is reported as 'not found' as in the reference. (4) doctor opens the repository with RepoRoot like the other commands (refuses a working directory in an account folder).

Verified by the lead (2026-11-24): mise run rust:ci and mise run lint exit 0, src/ unchanged. Open for the owner: the probe's environment allowlist (cloud-provider sign-in variables). ALZ_ACCOUNT_DIRS and the blocked account folders in doctor are R6 (TASK-50.4).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
alz doctor reports the Claude version against the verified range, skill collisions with the user's accounts (read for names only) and the client-root warning; alz ls --live runs claude through a ToolRunner (absolute path from a validated PATH, argument array, cleared environment plus an allowlist, own process group killed at the deadline, capped and stripped output) and compares what loaded with what was built.
<!-- SECTION:FINAL_SUMMARY:END -->
