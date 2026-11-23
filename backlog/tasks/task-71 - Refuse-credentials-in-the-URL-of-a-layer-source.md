---
id: TASK-71
title: Refuse credentials in the URL of a layer source
status: To Do
assignee: []
created_date: '2026-11-23T04:51:00Z'
updated_date: '2026-11-23T05:19:00Z'
labels:
  - discovered
  - area:rust
  - area:security
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
priority: medium
type: bug
ordinal: 88390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in the TASK-65 review. A git: source such as https://user:token@host/repo.git is accepted, so its token reaches diagnostics (§6 'A failed fetch' prints the source as written), layer.lock (source) and allowed.yaml. Spec §5.2 'Credentials in a source' now refuses userinfo in an http or https URL, and a password in an ssh:// URL. The message never prints the source. A user name without a password stays allowed for ssh:// and the scp-like form (git@host:owner/repo.git). The TypeScript reference accepts all of these (reference drift). Fixed only in Rust, in R3 (TASK-55.4), per decision-30.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz build with a git:https://user:token@host/x.git@v1 source exits 1 with error [source] and the §5.2 message, and neither stderr nor any written file holds the token (acceptance test, referenceDrift)
- [ ] #2 git:https://token@host/x.git@v1 and git:ssh://git:secret@host/x.git@v1 are refused the same way; git:ssh://git@host/x.git@v1 and git:git@host:o/x.git@v1 are not refused for it (acceptance or Rust tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23): the rule in §5.2 'Credentials in a source' is accepted as written.

Architect (2026-11-23, R2 plan in TASK-55.3): the parse rule of spec §5.2 'Credentials in a source' is built into the Rust GitUrl type in R2.6 (TASK-55.3.6) with unit tests; the CLI cases still need alz build, so this task stays R3's.
<!-- SECTION:NOTES:END -->
