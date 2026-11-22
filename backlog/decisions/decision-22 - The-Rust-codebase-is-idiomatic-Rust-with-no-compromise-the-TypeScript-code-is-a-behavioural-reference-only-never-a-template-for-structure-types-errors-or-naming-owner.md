---
id: decision-22
title: >-
  The Rust codebase is idiomatic Rust with no compromise: the TypeScript code is
  a behavioural reference only, never a template for structure, types, errors or
  naming (owner)
date: '2026-11-22T13:04:00Z'
status: accepted
---
## Context

decision-21 moves alz to Rust and keeps the TypeScript implementation as the reference until
cutover. A port that follows the existing code would carry TypeScript habits into Rust: its module
layout, loosely typed records, exceptions used for control flow, string kinds and test seams that
mock the file system.

## Decision

The Rust codebase follows idiomatic Rust and the best current Rust conventions, with no
compromise. The TypeScript code is a reference for behaviour only (what the spec and the suite
say alz does), never a template for crate or module structure, types, error handling, naming or
patterns. The standard is the one in doc-5 ("Rust standard"): edition 2024, rustfmt, clippy with
`-D warnings` and `pedantic`, `forbid(unsafe_code)`, validated newtypes built by parsing, typed
errors with `thiserror`, `clap`, documented public items, Rust tests (unit, integration,
`assert_cmd`, `proptest`), and `cargo deny` and `cargo audit` in CI.

## Consequences

The crate layout is designed from the spec in R0 and recorded as a decision before domain code.
Generated files stay byte-identical because users commit them; user-facing messages follow the
spec, and where a Rust convention changes them the spec is updated on purpose. The TypeScript
acceptance suite is only a temporary oracle and is replaced by Rust tests before cutover (R8). The
repository rules for TypeScript stop applying to new code.
