---
id: TASK-105
title: Create the alz-adapter contract crate with TargetId and model v1
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-26T03:23:00Z'
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

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-26T03:23:00Z

---

From TASK-104 (@architect): spec §8.0 Model. defines four kinds of security item in model 1.0 (deny rules, set floor fields, the default_mode floor, locked hooks that run on the target), each holding the layers that set it. Model them from the start so the minor version does not need to add them.

---
<!-- COMMENTS:END -->
