---
name: pre-change-baseline-binary
description: Without git diff, the installed ~/.local/bin/alz bundle (built from an earlier checkout) can show pre-change source for refactor-equivalence checks; dist/ is rebuilt from the working tree; the bundle may be too old to help
metadata:
  type: project
---

`~/.local/bin/alz` is a single-file esbuild bundle from `mise run install:local`, with `// src/<path>` section comments. It often predates the uncommitted change, so grepping it shows the old code for a "behaviour unchanged" claim. `dist/cli.js` and `dist/alz-standalone.mjs` are usually rebuilt from the working tree, so they show the new code, not the baseline.

**Why:** In the TASK-50.8 review (2026-11-22), this was how I confirmed that the commit_identity messages, codes and diagnostic order stayed byte-identical after the declarers.ts refactor. In the TASK-50.11 review the same day it was useless: the bundle had no `src/resolve/account*` sections at all (it predates TASK-50.8), so the readRepositoryDeclaration extraction could only be checked by reading the new code.

**How to apply:** Grep it for the function name before you trust it, and check that it really holds the old shape: it may have been reinstalled after the change, or be older than the code under review. Grep a specific short pattern; a Glob under `~` times out. Say in the report that it is a baseline you inferred, or that none was available. See [[review-tooling-no-shell]].
