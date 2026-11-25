---
name: ci-gotchas
description: Verified gotchas for mise/GitHub Actions CI in agent-layerz (action pinning, mise auto-install, cargo-audit arch)
metadata:
  type: project
---

- Annotated tags (e.g. Swatinem/rust-cache v2.9.3): `git ls-remote` shows the tag object; pin the `^{}` commit SHA. Confirm with `gh api repos/<r>/commits/<sha>`.
- `mise run` auto-installs every missing tool in mise.toml (verified 2026-11-22), defeating mise-action `install_args`; CI sets `MISE_AUTO_INSTALL=false`.
- cargo-audit's aqua release is x86_64-apple-darwin only; run supply-chain on ubuntu.
- mise's rust is a symlink into rustup, so mise-action cache is off for Rust jobs.
- Required checks + workflow-level `paths:` leave checks Pending and block merges; chose no path filter (TASK-55.1.5).
- actionlint and zizmor are pinned in mise.toml; `mise run workflows:lint`.
- mise-action v5.3.0: `version` + `sha256` inputs pin mise; sha256 is of the extracted binary (per OS, one value per step). Token is NOT persisted unless `persist_github_token` is true. The action auto-adds `--locked` only when a lockfile exists; `MISE_LOCKED=1` makes absence an error (verified locally 2026-11-22). Lockfile needs `mise lock --platform linux-x64,macos-arm64`. Read dist sources via raw.githubusercontent (contents API is empty for the big dist file).
- Matrix cannot read the `env` context; use a ternary in the step for per-OS values.
- Scheduled runs need their own concurrency group or they can displace a pending push run.
