---
id: TASK-71
title: Refuse credentials in the URL of a layer source
status: In Progress
assignee: []
created_date: '2026-11-23T04:51:00Z'
updated_date: '2026-11-23T11:58:00Z'
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
- [x] #1 alz lint in a repository with a git:https://user:token@host/x.git@v1 source exits 1 with error [source] and the §5.2 message, and neither stderr nor any written file holds the token (acceptance lint twin, referenceDrift, R3.11)
- [x] #2 git:https://token@host/x.git@v1 and git:ssh://git:secret@host/x.git@v1 are refused the same way; git:ssh://git@host/x.git@v1 and git:git@host:o/x.git@v1 are not refused for it (acceptance or Rust tests)
- [ ] #3 The alz build form of AC #1 is in the acceptance suite (written in R3.11 where it is missing) and is checked again against the Rust alz in R5 (TASK-55.6)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23): the rule in §5.2 'Credentials in a source' is accepted as written.

Architect (2026-11-23, R2 plan in TASK-55.3): the parse rule of spec §5.2 'Credentials in a source' is built into the Rust GitUrl type in R2.6 (TASK-55.3.6) with unit tests; the CLI cases still need alz build, so this task stays R3's.

Architect (2026-11-23, owner answer 7 in TASK-55.4, delegated by the lead): criteria reworded to the lint twins that R3.11 adds; the build, check and account-resolution forms are checked again in R5 (TASK-55.6) or R6 (TASK-55.7), as the last criterion says. The other criteria are unchanged.

R3.11 (2026-11-23): AC1 and AC2 checked. Lint twins in test/acceptance/24-untrusted-repo.test.ts pass against the Rust binary: four refused forms (https user:token, https token, ssh password, https token without a ref) give error [source] with the section 5.2 message, no token in output and no file written; ssh://git@ and git@host:o/x.git are not refused for credentials. The four refused forms are referenceDrift TASK-71. AC3 left unchecked: build forms are written and fail until R5. Finding: the whole-text credentials rule also refuses scp-like sources with no slash in the path and a ref, e.g. git:git@host:repo.git@v1, which needs a decision.
<!-- SECTION:NOTES:END -->
