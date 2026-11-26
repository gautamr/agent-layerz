---
id: TASK-107
title: Render the Claude adapter from model v1 instead of Merged
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-25T19:06:00Z'
updated_date: '2026-11-26T05:51:00Z'
labels:
  - area:adapter
milestone: m-8
dependencies:
  - TASK-106
references:
  - decision-40
priority: medium
ordinal: 208390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Claude adapter reads alz-core's internal Merged today. decision-40 point 2 has the core convert Merged into the published model and the adapter read only that, so merge types can change without breaking adapters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz-core converts a Merged stack into Model, and one test per item kind of spec §7 shows each neutral field reaches the model
- [ ] #2 Nothing under crates/alz-core/src/target/claude imports crate::merge (grep)
- [ ] #3 Output is byte-identical: the acceptance suite and mise run alz:check pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (commit f83b171 on work-107). AC1: crates/alz-core/src/adapter_model.rs (model_for(&Merged,&TargetId)->Model; one file per kind: agents, skills, rules, hooks, permissions, mcp, models, settings, security). Tests in adapter_model/tests.rs, one per item kind (agents incl. native fields and other-target narrowing, model problems, skills/commands/slot stubs, skill sources, rules, hooks, permissions, MCP, models, settings) plus one per security kind (deny rules, floor fields, default_mode floor, locked hooks) with set_by layers. Merge change to supply the layers: MergedPermissions.deny_declared_by/flag_set_by/floor_set_by (each layer that declares/sets, in stack order). core LayerName now wraps alz_adapter::LayerName (to_contract is total). AC2: grep -rn 'crate::merge' crates/alz-core/src/target/ prints nothing (exit 1). The adapter tests stay at their spec-named paths (target/claude/tests*) and reach the merge only through adapter_model::testing. AC3: existing fixture/golden tests unchanged and green (1469 alz-core tests); mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run alz:check: 'check passed: 67 generated files match the layers and the lock'. Model additions, version stays 1.0 (no adapter exists outside this repo): Agent.model_problem: Option<ModelProblem> (NotAnAlias/NoModelForTarget, needed for the models error and warning of spec 8.1); McpTransport::Stdio.args changed from Vec<String> to Option<Vec<String>> (args: [] and no args differ in .mcp.json; this changes a field's type, so arguably a 1.1 or a re-statement of 1.0 before first use). Behaviour notes: non-finite floats in settings/native become JsonValue::Null (emitter already writes null; agent writer already refuses null), identical output; the Where of the skill-SKILL.md-without-frontmatter error is now the winning definition's file instead of the first contribution's. Spec left unclear: 'each layer whose floor is in force' read as each layer whose floor:true was accepted; 'each layer that declares a deny rule' read as each layer whose deny list names it (add or replace). Spec 8.1 names test paths under target/claude/tests*, which is why they were not moved.
<!-- SECTION:NOTES:END -->
