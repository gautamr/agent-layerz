---
id: TASK-12
title: Keep private forbid patterns out of public layers
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-19T17:13:00Z'
updated_date: '2026-11-21T04:02:00Z'
labels:
  - area:security
  - area:spec
milestone: m-1
dependencies: []
priority: high
ordinal: 170
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The leak guard reads forbid_patterns from layer.yaml. In a public layer that file is published, so the patterns themselves reveal the private names they protect. The owner's personal layer lists such names today, which blocks publishing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The spec defines a private source of forbid patterns that is never committed (for example an environment variable or a file outside any repository) and how it combines with layer.yaml patterns
- [ ] #2 alz lint warns when a public layer's forbid_patterns contain literal names instead of generic patterns
- [ ] #3 An acceptance test shows a public layer failing lint for a privately configured pattern that appears nowhere in the repository
- [ ] #4 The README Safety section is updated
- [ ] #5 alz check applies the private patterns to every tracked file in the repository (including backlog/ and docs/), and to author, committer and message of commits not yet on the remote
<!-- AC:END -->
