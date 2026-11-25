---
id: TASK-102
title: The adopt problem texts differ from the spec (section 10.3)
status: To Do
assignee: []
created_date: '2026-11-25T17:22:00Z'
labels:
  - bug
dependencies: []
ordinal: 204390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Four problem lines of 'alz build --adopt' do not have the words section 10.3 gives. (1) A server env entry: the spec says 'server <id> sets <NAME> to a value', the build quotes the name ('TOKEN'). (2) A server with unsupported fields: the spec says 'uses <keys>, which mcp.yaml does not support yet', the build says 'uses fields that mcp.yaml does not support yet' with no key names. (3) A permission rule with an unknown tool: the spec says '<file>.<list>: <rule> uses a tool agent-layerz does not know', the build says '<file>: permissions.<list>: <rule> uses a tool agent-layerz does not know'. (4) Text the layer reader refuses: the spec says '<file>: adopting it would write <layer file> with text the layer reader refuses: <rule>; fix it in <file> and rerun', the build says '<file>: the repo layer that adoption plans would be refused: <full reader line>'. Decide which side is right, change the other, and add the missing texts to crates/alz/tests/adopt_output.rs. Found while closing the section 10 gaps of TASK-55.9.
<!-- SECTION:DESCRIPTION:END -->
