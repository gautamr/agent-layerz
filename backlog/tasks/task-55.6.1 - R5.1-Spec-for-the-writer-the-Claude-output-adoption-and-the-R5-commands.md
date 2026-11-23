---
id: TASK-55.6.1
title: 'R5.1: Spec for the writer, the Claude output, adoption and the R5 commands'
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-23T18:08:00Z'
labels:
  - area:rust
  - area:spec
  - needs:owner-approval
milestone: m-7
dependencies: []
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 133390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: the spec text R5 builds against, written once the owner has answered the R5 questions in TASK-55.6's notes. No code.
- §4.3: the writer. A file is written or deleted only when alz can show it generated it (owner question 1); never through a link (the entry or any parent folder); paths compared with ASCII case folding of the NFC form; the lock written last; the managed .gitignore block (current and legacy markers); the manifest edit of --adopt (owner question 2).
- §4.5: protected paths match with ASCII case folding.
- §7.1 and §4.3: the generated header, its source refs, and the rule that a ref holds no control or format character and cannot close its comment (owner question 6).
- §8.1: the Claude output per item, including the three permission floor keys once TASK-76 records them, and what is written for each neutral tool and mode.
- §10 and §10.1: each R5 command's output lines and exit codes, the diff format (owner question 3), and the codes collision, check, budget, protected, adopt, target, claude-agent, tool-scope, hook-on-fail and ls.
- §13: entries for the writer rules and the parity goldens. CHANGELOG entry (user-facing, minor bump at cutover).
Out of scope: Codex output (TASK-18), §9.4 identity behaviour (R6).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/LAYERING-SPEC.md states every rule listed in this task's description, each owner answer of TASK-55.6 included (spec diff, reviewed)
- [ ] #2 §10.1 lists each new diagnostic code with its level and what raises it, and §10 quotes every R5 message the acceptance suite asserts (spec diff checked against grep of test/acceptance)
- [ ] #3 CHANGELOG.md names the user-facing changes and the minor bump at cutover; mise run lint exits 0
<!-- AC:END -->
