---
id: TASK-55.5.12
title: 'R4.12: Passthrough files and their protected-variable checks'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T14:52:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.11
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 125390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.11 and the passthrough part of §7.9).
- item/passthrough.rs: targets/<target>/<path> per target; the most specific layer wins per path; a passthrough warning per use. A path with a non-ASCII character is refused for every target.
- Checks for every target, paths compared with ASCII case folding: paths ending in .claude/settings.json or .claude/settings.local.json, every .mcp.json and every .claude-plugin/*.json are parsed with alz's JSON tree, and the root env, the env of each mcpServers entry and any nested env mapping are checked; .claude/agents/**/*.md frontmatter mcpServers is checked; a *.toml file is refused by the text rule of §7.9 (shell_environment_policy, a whole-name mention, \u, \U or \x escapes, a trailing backslash). A covered file that does not parse is refused.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each passthrough case of §7.9 is refused with protected-variable and no value, for every target, with case-folded path matching (table tests)
- [ ] #2 A non-ASCII passthrough path is refused, and a covered JSON file or agent frontmatter that does not parse is refused (unit tests)
- [ ] #3 The most specific layer wins per path and each use warns (unit test), and the per-path merge is associative (proptest)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-23, R4.1): spec §4.1 and §7.11 decide the case of a folder in targets/ whose name is not a known target. It is an error, not ignored: each file below such a folder is a passthrough error that names the file. The reference merges files under any folder name, warns and writes none of them, so the Rust behaviour is reference drift (TASK-55.5). A file directly in targets/ is no item. R4.2's ItemPath gives Other for both; R4.12 turns the first into the error. Add a unit test: targets/claud/x.json gives a passthrough error naming the file, and targets/README.md gives no diagnostic.
<!-- SECTION:NOTES:END -->
