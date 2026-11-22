---
id: TASK-56
title: Configurable review policy in layers (review:)
status: To Do
assignee: []
created_date: '2026-11-22T13:49:00Z'
labels:
  - area:process
  - area:spec
milestone: m-3
dependencies: []
priority: medium
ordinal: 42390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today the review rule is one sentence in step 5 of .layer/commands/next-task.md: run the reviewer after every implemented task, and the security-reviewer when the task has area:security or touches files, subprocesses, hooks, permissions or secrets. Nothing reports or validates it, a personal default cannot be overridden per client or repository in a controlled way, and nothing enforces it.

Goal: a first-class review section that layers declare and alz merges, validates, renders and optionally enforces, so the owner chooses when reviews run (per subtask, per task, per parent task or per milestone), which reviewers run under which conditions, and what the review gates. Example:

review:
  unit: parent            # subtask | task | parent | milestone
  reviewers:
    - agent: reviewer
      when: always
    - agent: security-reviewer
      when: { labels: [area:security], touches: [files, subprocesses, hooks, permissions, secrets] }
      locked: true        # a later layer cannot remove or weaken it
  gate: done              # done | commit

Built in Rust after the cutover (decision-21, decision-22); the spec part can start now. Backlog.md definition-of-done defaults alone cannot express this (they are project-wide and know nothing of agents), so they are at most the visible checklist.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every subtask is Done
- [ ] #2 In a repository whose layers set unit: parent, a /next-task run on a subtask does not start a reviewer, and finishing the parent does; with the security-reviewer locked in the personal layer, a repository layer cannot remove it (recorded end-to-end run)
<!-- AC:END -->
