---
id: TASK-70
title: Run git for a remote layer source so that ssh cannot prompt on the terminal
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
  - src/resolve/git.ts
priority: medium
type: bug
ordinal: 87390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in the TASK-65 review. alz runs git with GIT_TERMINAL_PROMPT=0 and stdin closed, and captures stdout and stderr (src/resolve/git.ts). A program git starts, such as ssh, can still open /dev/tty and prompt there: a host key confirmation or a key passphrase, outside alz's output and its sanitising. Spec §6 'A failed fetch' names this limit. Options: (a) ssh -o BatchMode=yes through GIT_SSH_COMMAND, which replaces a user's own core.sshCommand or GIT_SSH_COMMAND; (b) start git without a controlling terminal (setsid), so neither git nor ssh can open /dev/tty, while an ssh agent keeps working; (c) keep prompts. Both (a) and (b) break a passphrase-protected key that is not in an agent, so the choice is the owner's (needs:owner-approval). Fixed only in Rust, in R3 (TASK-55.4), per decision-30. Out of scope: credential helpers, which git runs itself.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The spec states whether git and the programs it starts may prompt during a build, and how the Rust alz prevents it if not (spec diff)
- [x] #2 A Rust test with a fake ssh that opens /dev/tty shows the chosen behaviour
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-23): never prompt. The spec (§6 'A failed fetch') now says git and the programs it starts never reach the terminal; a passphrase key must be in an agent and an unknown host key fails the fetch. The mechanism is R3's choice: prefer one that does not replace the user's core.sshCommand or GIT_SSH_COMMAND (option b, no controlling terminal) if it can be done without unsafe code (decision-27); otherwise record why (a) was chosen.

R3.11 (2026-11-23): AC1 spec 'Git subprocesses / No terminal' in section 6 (R3.1); AC2 crates/alz/tests/git_runner.rs fake ssh opening /dev/tty under script(1) (R3.3). rust:ci passes.
<!-- SECTION:NOTES:END -->
