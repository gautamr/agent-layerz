---
id: TASK-55.1.4
title: Supply-chain and quality tooling for the Rust workspace
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T15:53:00Z'
updated_date: '2026-11-22T16:50:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.1.3
parent_task_id: TASK-55.1
priority: medium
ordinal: 62390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add deny.toml (licences, bans including duplicate versions, advisories, sources) and mise tasks for cargo deny, cargo audit, docs with -D warnings, doc tests, and an MSRV check (cargo hack --rust-version or a pinned toolchain). Optional: cargo-nextest. Record any tool added as a dev dependency decision if it is a crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 mise tasks run cargo deny check, cargo audit, the docs build with -D warnings and the MSRV check, all passing on the skeleton
- [x] #2 rust:check (or a new rust:ci task) runs the full local set in one command
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pin cargo-deny and cargo-audit through mise's aqua backend (prebuilt binaries). 2. Write deny.toml from what cargo deny reports on the real graph. 3. Add mise tasks rust:deny, rust:audit, rust:doc, rust:doctest, rust:msrv and a sequential rust:ci; leave rust:check fast. 4. Prove each gate fails when it should (broken doc link, rust-version mismatch).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Tools (not crate dependencies, so no decision needed): mise.toml pins aqua:EmbarkStudios/cargo-deny 0.22.0 and aqua:rustsec/rustsec/cargo-audit 0.23.0, both prebuilt binaries, no compiling. The aqua registry ships cargo-audit only as an x86_64 macOS binary, so it runs under Rosetta on Apple silicon. Tasks call cargo-deny and cargo-audit directly, not 'cargo deny': cargo searches ~/.cargo/bin before PATH and found an older global cargo-deny 0.21.0 there.

deny.toml: licences allow MIT, Apache-2.0 and Unicode-3.0 (the set the current graph needs: clap family, assert_cmd, predicates, unicode-ident); every licence added later needs a comment naming the crate. Advisories: version 2, yanked, unmaintained and unsound all denied, no ignores. Sources: crates.io only, no git. Bans: wildcards denied. Duplicate versions: DENY. The graph is small (38 crates, no duplicates today), so deny costs nothing now and keeps it that way; when a duplicate is unavoidable it goes into bans.skip with the reason and the forcing crate, which makes each duplicate a reviewed decision instead of drift. Revisit as warn only if skip entries start to churn.
Version bans work without the crates being in the graph: cargo-deny's { name, version = "<req>" } form takes a real semver requirement (verified against clap). Added rustix <1.1.6 (decision-27) and serde-saphyr <2.2.0 and >2.2.0 (decision-28 exact pin; the exact requirement in Cargo.toml is the primary guard, this catches a lockfile edit). Do not use the { crate = "name@<req>" } form: it ignores the operator and treats the version as a caret match (verified: clap@<4.6.9 banned 4.6.9). The tasks adding rustix and serde-saphyr should confirm these lines still match their entry and keep the exact-pin bump in two places.

MSRV: MSRV equals the mise pin (decision-25), so a second toolchain adds nothing. rust:msrv fails if rust-version in Cargo.toml differs from rustc --version of the pinned toolchain (the real way the two could drift), then runs cargo check --workspace --all-targets --all-features --locked on it. cargo-hack not added: no feature matrix and no older toolchain to test. cargo-nextest not added: tests run in well under a second; revisit when they do not. If MSRV is ever lowered below the pin, replace rust:msrv with a check on a second pinned toolchain.

Tasks: rust:deny, rust:audit (--deny warnings), rust:doc (RUSTDOCFLAGS=-D warnings, --no-deps, --locked), rust:doctest, rust:msrv, rust:ci (sequential: rust:check, rust:doc, rust:msrv, rust:deny, rust:audit). rust:check unchanged and fast. rust:doctest is standalone: cargo test --workspace in rust:check already runs doc tests, so rust:ci does not repeat it.

Evidence: all five new tasks and rust:ci exit 0 on the skeleton; the advisory database fetched from the network (1560 advisories). Negative checks: a broken intra-doc link made rust:doc fail; rust-version 1.99.0 made rust:msrv fail with the explanatory message (both reverted). Not done: the AGENTS.md command table is generated from .layer/, so the new tasks are not listed there; add them with the CI task (TASK-55.1.5) if wanted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
deny.toml (licences MIT/Apache-2.0/Unicode-3.0, advisories with yanked/unmaintained/unsound denied, crates.io only, duplicates and wildcards denied, version bans for rustix <1.1.6 and serde-saphyr != 2.2.0 per decisions 27/28). cargo-deny 0.22.0 and cargo-audit 0.23.0 pinned through mise's aqua backend (tools, not crates, so no decision); tasks call the binaries directly because a global cargo-deny in ~/.cargo/bin shadowed the pin. New mise tasks rust:deny, rust:audit, rust:doc (-D warnings), rust:doctest, rust:msrv (rustc must equal rust-version, then cargo check --locked) and rust:ci running the full set. Orchestrator reran mise run rust:ci: exit 0. Negative checks (broken doc link, rust-version 1.99.0) failed as expected and were reverted. Open: cargo-audit has only an x86_64 macOS binary in aqua (Rosetta on Apple silicon); tasks adding rustix and serde-saphyr confirm the ban lines.
<!-- SECTION:FINAL_SUMMARY:END -->
