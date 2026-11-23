---
id: TASK-55.5.1
title: 'R4.1: Spec: merge codes, validated item keys and R4''s open merge rules'
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-23T12:27:00Z'
labels:
  - area:rust
  - area:spec
  - needs:owner-approval
milestone: m-7
dependencies: []
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 114390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: spec text only (docs/LAYERING-SPEC.md, CHANGELOG.md), on the R4 branch, so it ships in the R4 pull request with the code. It waits for the owner questions in TASK-55.5's notes.
- §7.9: the top level of settings.yaml takes only the target names claude and codex. Any other top-level key is a `settings` error that names the layer and the file, whatever the key, and that file's settings are not merged. The merged settings are ordered maps of validated keys.
- §7.7: the keys of mcp.yaml are server ids with the grammar the owner picks (question 1); any other key is an `mcp` error naming the file, printed as a quoted key. mcp.enable uses the same grammar.
- §7.8: each alias of models.yaml maps target names only.
- §10.1: every diagnostic code the merge emits, with its level, and how item names, refused keys and refused values print (decision-26 classes; protected and secret values never).
- The answers to questions 2 to 6 (leak guard phase, lint scope, test 26's phase, settings deep merge, tool scope containment), or a note that the reference behaviour stays.
- §13: a new entry for "a settings or MCP file whose top-level key is not a known target is refused, whatever the key".
- CHANGELOG: the refusals are user-facing (a layer that built before may fail now); minor version bump at cutover (doc-5).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 §7.7, §7.8 and §7.9 state which top-level keys settings.yaml, mcp.yaml and the aliases of models.yaml accept, and that any other key is an error naming the file (spec diff)
- [ ] #2 §10.1 lists every diagnostic code the merge emits with its level, and says how item names, refused keys and refused values print (spec diff)
- [ ] #3 Each answered owner question of TASK-55.5 has spec text or a note that the reference behaviour stays; §13 has the known-target entry; CHANGELOG lists the user-facing changes with the version note (spec and CHANGELOG diff)
<!-- AC:END -->
