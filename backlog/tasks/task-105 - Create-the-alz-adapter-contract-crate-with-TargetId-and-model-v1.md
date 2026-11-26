---
id: TASK-105
title: Create the alz-adapter contract crate with TargetId and model v1
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-26T04:18:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-104
references:
  - decision-40
priority: medium
ordinal: 206390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The contract of decision-40 point 2 lives in its own crate, so adapters depend on it and never on alz-core. This task adds the crate and its types only; nothing uses them yet. Amends decision-23's four-crate layout (accepted with decision-40).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/alz-adapter depends on no other alz crate (Cargo.toml) and its clippy.toml bans std::fs, std::process and std::env (grep)
- [ ] #2 TargetId::try_from accepts exactly the grammar of spec §5.2 (unit tests and a proptest over generated strings)
- [ ] #3 The crate exports Model with MODEL_VERSION 1.0, Descriptor, Context, Output, OutputFile with Json, Toml, Markdown and Bytes content, and the Adapter trait with descriptor() and render(&Model, &Context) -> Output (cargo doc -p alz-adapter builds)
- [ ] #4 Every public struct of the model is #[non_exhaustive] (grep) and mise run rust:ci passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (crates/alz-adapter). AC1: Cargo.toml lists only thiserror (decision-31) and dev proptest; no alz crate. clippy.toml is the twin of alz-core's without the serde-saphyr/serde_json entries and bans std::fs, std::process, std::env, the clock and the standard streams (grep 'std::fs\|std::process\|std::env' crates/alz-adapter/clippy.toml). lib.rs has #![forbid(unsafe_code)]. AC2: TargetId::try_from/FromStr parse exactly ^[a-z][a-z0-9-]{0,31}$ (byte check, length <= 32); unit tests in src/target_id.rs (spec examples and counter-examples, length edge, newline) and four proptests, one against an independently written grammar over a broad alphabet. AC3: exports Model (MODEL_VERSION 1.0, ModelVersion::runs for the registration rule), Descriptor, Context, Output, OutputFile, Content (Json, Toml, Markdown, Bytes), Coverage, and trait Adapter { descriptor(), render(&Model, &Context) -> Output }; tests/adapter_contract.rs implements a toy adapter and uses it as a trait object; cargo doc passes in rust:ci. The four security items: DenyRule, FloorFieldItem, DefaultModeFloor, LockedHook (with its on_fail), each with SettingLayers (non-empty, stack order) or defined_by; SecurityItems::ids() gives the SecurityItemId the coverage report names. AC4: every public struct with public fields is #[non_exhaustive] (grep), each with a constructor. mise run rust:ci passes (fmt, clippy -D warnings, tests, doc, msrv, deny, audit). docs/spec-coverage.md: evidence added to the 5.2 Target ids and 8.0 Model/Descriptor/Rendering/Output rows; they stay planned. Open: Model fields hold plain String for already-checked names, rules and paths (the core checks them before building the model); TASK-107 may tighten. Descriptor FloorRule/vocabulary shapes are a first cut for TASK-110.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-26T03:23:00Z

---

From TASK-104 (@architect): spec §8.0 Model. defines four kinds of security item in model 1.0 (deny rules, set floor fields, the default_mode floor, locked hooks that run on the target), each holding the layers that set it. Model them from the start so the minor version does not need to add them.

---
<!-- COMMENTS:END -->
