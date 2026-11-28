---
name: long-decision-filenames
description: Backlog decision files with long titles break the Edit/Write tools (ENAMETOOLONG on the temp file); fill decision bodies via a shell splice
metadata:
  type: project
---

Backlog.md names a decision file after its whole title. A title of ~200 characters (decision-40) makes the
Edit tool fail with ENAMETOOLONG, because it writes `<name>.tmp.<pid>.<hex>` next to the file.

**Why:** the owner-approved exception lets the architect fill a decision's Context/Decision/Consequences by
hand, but the title must not be changed by hand, so the long name cannot be shortened afterwards.

**How to apply:** keep decision titles under ~150 characters when creating them. If one is already long,
write the body to the scratchpad and splice it under the first 10 frontmatter lines with a small bash
script that uses absolute paths (compound `cd ... &&` commands get refused in a worktree-isolated agent).
