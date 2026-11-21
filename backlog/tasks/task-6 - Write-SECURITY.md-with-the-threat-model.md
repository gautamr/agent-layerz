---
id: TASK-6
title: Write SECURITY.md with the threat model
status: To Do
assignee:
  - '@docs-writer'
created_date: '2026-11-19T17:13:00Z'
labels:
  - area:docs
  - area:security
milestone: m-1
dependencies: []
priority: high
ordinal: 180
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Four issues with untrusted repository content were fixed before 0.1.0. Reporters need a private channel, and users need to know what alz trusts. The threat-model skill holds the current model.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Explains private reporting through GitHub security advisories
- [ ] #2 States the trust model: repository content is untrusted, layers are trusted code, account folders are read for names only
- [ ] #3 Lists supported versions and a 72-hour acknowledgement target
- [ ] #4 Reviewed by the security-reviewer with findings resolved
<!-- AC:END -->
