---
id: TASK-113
title: Write the guide for adapter authors
status: To Do
assignee:
  - '@docs-writer'
created_date: '2026-11-25T19:06:00Z'
labels:
  - area:adapter
  - area:docs
milestone: m-8
dependencies:
  - TASK-112
references:
  - decision-40
priority: low
ordinal: 214390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Third parties need one place that explains the contract of decision-40: model v1, territory, the core checks, structured output, the facts rule and the harness, versioning. The WASM parts are added when the host exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/ADAPTERS.md has sections for the contract, model v1, territory and checks, output kinds, facts and the harness, and versioning (grep)
- [ ] #2 The guide's example adapter compiles as a doc test against alz-adapter (cargo test --doc)
<!-- AC:END -->
