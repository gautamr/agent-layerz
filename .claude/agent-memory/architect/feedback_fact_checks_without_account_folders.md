---
name: fact-checks-without-account-folders
description: How to fetch vendor docs, review big diffs and smoke-test Claude/Codex account facts without touching real account folders (WebFetch and large Bash output spill, no `timeout`, env -i with fake HOME)
metadata:
  type: feedback
---

When verifying target facts, download vendor doc pages with `curl` into the session scratchpad and grep them, rather than using WebFetch on large pages. The same applies to any large Bash output, such as `git diff` of the spec.

**Why:** WebFetch, and any Bash output over about 30 KB, is saved as a file inside the active Claude account folder (`$CLAUDE_CONFIG_DIR/projects/.../tool-results/`). The owner's rule forbids reading any file inside a real account folder, so that output can't be read back. It happened again on 2026-11-22 with a full `git diff docs/LAYERING-SPEC.md`, and with `sed` of `docs/target-facts.md` §12 (about 34 KB on its own).

**How to apply:**
- Redirect big outputs (diffs, test logs) to a scratchpad file and read slices with Read or `sed -n`. Review an edited spec by reading the file itself, not the whole diff.
- Docs: `curl -sSfL https://code.claude.com/docs/en/<page>.md` into a fresh scratchpad dir, then grep or sed. The Codex source is at tag `rust-v<version>`, and a shallow sparse clone of `codex-rs` works.
- For one or two Codex source files, `curl -sSfL https://raw.githubusercontent.com/openai/codex/rust-v<version>/codex-rs/<crate>/src/<file>.rs` is faster than a clone. A Rust dependency's source comes from `https://static.crates.io/crates/<name>/<name>-<version>.crate` (a tarball), with the version read from `codex-rs/Cargo.lock`. This is how the wildmatch case-folding fact was checked on 2026-11-22.
- The Bash tool runs zsh, which does not word-split `$var`, so `set -- $pair` loops silently compare empty strings. Wrap such loops in `bash -c '...'`.
- macOS here has no `timeout`; use `perl -e 'alarm shift; exec @ARGV' <secs> cmd`.
- Smoke runs: `env -i HOME=<tmp>/home PATH=... CLAUDE_CONFIG_DIR=<tmp>/...` with stdin from `/dev/null`. Never run Claude with `CLAUDE_CONFIG_DIR` unset or empty: it would read the real default Keychain login. The session's own shell carries a real `CLAUDE_CONFIG_DIR`, so always override it.
- Keep account folder names seen in tool output out of every file and report. See [[backlog-decisions-title-only]] for other board conventions.
