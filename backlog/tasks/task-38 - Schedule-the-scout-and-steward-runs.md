---
id: TASK-38
title: Schedule the scout and steward runs
status: To Do
assignee: []
created_date: '2026-11-21T10:09:00Z'
labels:
  - area:process
  - needs:owner-approval
milestone: m-6
dependencies: []
priority: low
ordinal: 4390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The harness ran the scout weekly and the steward monthly from GitHub Actions. Scheduled agent runs cost money and need secrets; this repository is public, so the steward (which reads private projects) must run locally or from a private repository, never from this repository's Actions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner approves where and how often each runs, and the monthly cost
- [ ] #2 The scout runs on its schedule and opens a digest pull request
- [ ] #3 The steward runs only where its private project list is available and writes nothing private here
<!-- AC:END -->
