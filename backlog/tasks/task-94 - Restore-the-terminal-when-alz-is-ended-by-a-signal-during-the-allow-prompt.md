---
id: TASK-94
title: Restore the terminal when alz is ended by a signal during the allow prompt
status: To Do
assignee: []
created_date: '2026-11-24T14:52:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies: []
priority: low
ordinal: 181390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the R6 security review (a note, terminal integrity only). While account allow reads an answer it turns ISIG off and sets VEOL to the interrupt key; returns and panics restore the settings, but SIGTERM or SIGHUP during the prompt leave the shell's Ctrl-C, Ctrl-Z and Ctrl-\ without effect until stty sane. Either restore the saved settings on SIGTERM and SIGHUP (blocking them around the read and restoring before re-raising; a new dependency needs a decision) or document the limit in the spec.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Ending alz with SIGTERM or SIGHUP at the prompt leaves the terminal settings as they were before alz started, or the spec states the limit (pty test or spec diff)
<!-- AC:END -->
