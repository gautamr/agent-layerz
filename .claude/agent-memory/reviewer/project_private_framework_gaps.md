---
name: private-framework-gaps-file
description: FRAMEWORK-GAPS.md at the repo root is a private, git-excluded work order that contains client-identifying names; never quote it in reports or let its content reach tracked files
metadata:
  type: project
---

`FRAMEWORK-GAPS.md` at the repository root is excluded through `.git/info/exclude` (not `.gitignore`). It holds client-identifying account folder names and the owner's name.

**Why:** Seen in the TASK-50.1 review (2026-11-21). Tasks still ask for edits to it (for example, pointing to decisions), so reviewers will read it.

**How to apply:** In confidentiality checks, treat its private names as expected, since it is not tracked. Confirm it is still listed in `.git/info/exclude`. Never repeat its names in a review report. Flag any tracked file that copies text from it. See [[review-tooling-no-shell]].
