---
id: TASK-41
title: Close the gaps in the personal layer's hooks and notifications
status: To Do
assignee: []
created_date: '2026-11-21T13:07:00Z'
updated_date: '2026-11-21T17:03:00Z'
labels:
  - area:security
dependencies: []
priority: high
ordinal: 7390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by a fresh-context review of a client hub's agent setup on 2026-11-21 (personal layer v0.3.0). The path guard and hub guard only see Edit and Write, the path guard has shell-quoting and symlink gaps, notifications go to a public server by default, and the session brief hides board errors. Related: TASK-40 (memory gives write tools to read-only agents). Out of scope: client-layer configuration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 hooks/hooks.yaml matches NotebookEdit as well as Edit and Write for both guards, and a NotebookEdit outside the allowed paths is blocked (test with sample hook JSON piped into each script)
- [ ] #2 guard-paths.sh quotes the allowed-path list (a * in ownership.conf is not expanded) and resolves symlinks before comparing paths; tests cover a glob character, a symlink that points outside the repository, and a legitimate name containing '..'
- [ ] #3 notify.sh sends nothing unless the owner has configured a destination; there is no public ntfy.sh default (test: with no configuration it exits 0 and makes no network call)
- [ ] #4 The session brief shows an error line when a backlog command fails, instead of reporting 0, and reads a hub board when BACKLOG_CWD is set in the project settings (test with a failing command)
- [ ] #5 Each change is released in a new personal-layer tag with a changelog entry
- [ ] #6 notify.sh passes the message with curl --data-raw, not -d, so a message that starts with @ is sent as text and no file is read (test: a message '@/etc/hosts' sent to a local listener arrives as that literal text)
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @security-reviewer
created: 2026-11-21T17:03:00Z

---

2026-11-21: a second fresh-context review of the same client hub (security-reviewer) found that notify.sh:10 sends the body with curl -d "$msg", which reads a file when the message starts with @. Added as criterion #6.

---
<!-- COMMENTS:END -->
