---
id: TASK-97
title: >-
  Record the facts behind 11 Claude output keys that target-facts does not
  record
status: To Do
assignee: []
created_date: '2026-11-25T14:45:00Z'
updated_date: '2026-11-25T15:33:00Z'
labels:
  - bug
  - area:docs
dependencies: []
ordinal: 199390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Spec §8 says an adapter relies only on what docs/target-facts.md records. A test (target::claude::tests::facts, task-55.9.7) reads every key the Claude adapter writes and finds 11 that no verified row records: skill frontmatter name, description and argument-hint (§5 records only disable-model-invocation); permissions allow, ask and deny (§7b gives the rule syntax and order, but names none of the three keys); and the keys of a .mcp.json server: command, args, env, type, url (§7 and §13.4 say only that servers live in .mcp.json; the shape is recorded for Codex only). Verify each with the verify-target-fact skill (DOCS at least, SMOKE where possible), record them in target-facts, and shrink NO_FACT_RECORDS in crates/alz-core/src/target/claude/tests/facts.rs to empty, then mark the spec-coverage row for §8 What an adapter may rely on covered. Do not change what alz writes: these keys are the ones Claude reads today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the 11 keys has a verified row in docs/target-facts.md (DOCS at least)
- [ ] #2 NO_FACT_RECORDS in the facts test is empty and the test passes
- [ ] #3 The spec-coverage row for §8 What an adapter may rely on is covered
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence, 2026-11-25, Claude Code 2.1.323 (claude --version).
DOCS fetched 2026-11-25: code.claude.com/docs/en/skills.md (frontmatter reference: name, description, argument-hint), code.claude.com/docs/en/mcp.md (.mcp.json shape, type values, url without type, ${VAR} expansion), code.claude.com/docs/en/permissions.md (allow/ask/deny, order deny-ask-allow, trust for project allow rules), code.claude.com/docs/en/settings.md (lists such as permissions.allow merge across files; -p settings messages) and code.claude.com/docs/en/settings-reference.md (index rows for permissions.allow/ask/deny, scope any file).
SMOKE in a throwaway project with HOME and CLAUDE_CONFIG_DIR set to empty throwaway folders (nothing under ~/.claude* read or written), no login:
- skills: claude -p hi --max-turns 1 --output-format stream-json --verbose; the init event lists the skill under its folder name (probe-dir), not a different name: field. alz requires name == folder, so no difference. description and argument-hint: DOCS.
- .mcp.json: the stdio server (/bin/sh, args, env MARKER_ENV=${PROBE_VALUE}) ran and wrote MARKER-MCP-ENV; claude mcp get shows Type stdio (no type written), Command, Args, Environment unexpanded; Type http / sse with URL; a url without type is not loaded (No MCP server named ...). Matches what the adapter writes.
- permissions: claude -p hi --max-turns 1 prints (permissions.allow|ask|deny): Expected array, but received number for 12345, skips a malformed rule per key, and for a project allow rule in an untrusted folder: Ignoring 1 permissions.allow entry ... not been trusted.
Recorded: target-facts §5 (3 rows), §7 (5 rows), §7b (3 rows); header and §0 sources updated. NO_FACT_RECORDS emptied. spec-coverage: §8 What an adapter may rely on covered (the four facts tests), its known-differences line removed, summaries §8 4/1/1/0, all 155/24/9/3, TASK-55.9.7 gap rows 0.
No key differs from what the adapter writes; no bug task filed.
Checks: cargo test -p alz-core --lib target::claude::tests::facts: 4 passed. mise run rust:check: exit 0, 63 test suites, 2149 passed, 0 failed (spec_coverage guard 9 passed).
<!-- SECTION:NOTES:END -->
