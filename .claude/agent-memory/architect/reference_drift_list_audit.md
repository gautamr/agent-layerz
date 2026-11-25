---
name: drift-list-audit
description: How to audit spec §13's reference drift table against the referenceDrift marks in test/ (script, base commit, the stale-row trap, running only drift cases)
metadata:
  type: reference
---

Audit of §13's drift table (R6.16, 2026-11-24):

- Compare the marks, not the table: a Python script runs `git ls-tree`/`git show <rev>:<file>` over
  `test/**/*.ts` and matches `referenceDrift\(\s*["'`](TASK-[0-9.]+)\1\s*,\s*(name)`. Diff the base commit
  against HEAD to get the marks a milestone added. Local `main` can lag (R5 was not in `main..HEAD`'s base),
  so take the base from the board commit that started the milestone (R6: ec7c190).
- Trap: planning tasks write table rows with "(R6)" ahead of the cases. Later the case lands as a plain
  `it` (the reference passes it) or under another task's mark, so the row goes stale. Check every row's
  files against the marks.
- Differences only a file-system trace shows, or that no case asserts, are spec notes with their task.
  They are not table rows (a paragraph under the table names them).
- Run only the drift cases: `ALZ_BIN=<abs> npm exec --no -- vitest run --project acceptance -t 'reference drift'
  --reporter=json --outputFile=<scratch>`. Titles end in `[reference drift: TASK]`, so the filter catches
  every one, loops included (294 at R6).

Related: [[acceptance-baseline-run]], [[spec-edits-while-planning]].
