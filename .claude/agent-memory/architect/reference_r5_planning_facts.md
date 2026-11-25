---
name: r5-planning-facts
description: Non-obvious facts found planning R5 (TASK-55.6): no Codex adapter in the reference, adopt keeps manifest comments, lock ownership hazard, file names reach headers, zsh counting pitfalls
metadata:
  type: reference
---

Found while planning R5 (TASK-55.6, 2026-11-23). Check against current code before relying on them.

- The TypeScript reference has **no Codex adapter** (src/adapters/registry.ts lists only claude; codex is
  "planned", and `targets: [codex]` is error [target]). Codex output, and its TOML quoting of dotted keys, is
  TASK-18, not a port phase.
- `alz build --adopt` edits layer.yaml with `YAML.parseDocument` and keeps comments and flow style; test 20
  asserts `# the repo layer` and `targets: [claude]` survive. alz's emitter cannot do that, so it needs an
  owner choice (R5 question 8).
- The reference treats a path as owned when the previous layer.lock lists it OR it has the generated mark
  (src/render/workspace.ts). The lock is untrusted, so a lock entry alone could make build delete a
  gitignored file. Planned as owner question 1 in neutral words; the specifics went only to the handback.
- SafeRelPath refuses only NUL among control characters, so layer file names can carry line feeds into
  generated headers (AGENTS.md sources, frontmatter `# ...` comments). Question 6.
- `diff` uses jsdiff's createTwoFilesPatch; `doctor` and `ls --live` run the `claude` binary
  (ALZ_CLAUDE_BIN); `init --hub` runs `git tag --list layer-v*`. No glob matcher existed in Rust.
- A full acceptance run against ALZ_BIN takes about 23 s (981 cases). Tally per file with a Python
  script over the verbose output; zsh one-liners with `[` quoting and `grep -c` over many files failed twice.

**How to apply:** for R6 and R8 planning, start from TASK-55.6's notes (baseline counts, owner answers) and
the R5 subtasks TASK-55.6.1 to .15. See [[acceptance-baseline-run]] and [[private-security-tasks]].
