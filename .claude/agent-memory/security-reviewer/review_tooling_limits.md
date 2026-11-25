---
name: review-tooling-limits
description: security-reviewer runs with Read/Grep/Glob only (no shell), so git diff, cargo test and binary probes are unavailable; crate sources live in ~/.cargo/registry/src/index.crates.io-*/
metadata:
  type: reference
---

The security-reviewer agent has no Bash tool: it cannot run `git diff`, `cargo test` or the release
binary, even when the brief allows it. Review by reading files and say in the report which claims
are unverified by execution.

Dependency sources (serde-saphyr, granit-parser, rustix) can be read under
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/<crate>-<version>/`. Glob with a path
rooted there; a Glob over all of `~` times out.
