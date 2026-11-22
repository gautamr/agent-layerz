---
id: decision-16
title: >-
  Account allow is per repo, account and layer source, by a person only;
  accounts that cannot apply fail closed; init shares config only; amends
  decision-15 (TASK-50.1)
date: '2026-11-22T01:19:00Z'
status: accepted
---
## Context

The first review of the account spec (TASK-50.1) found that repository content is untrusted: a
manifest can name an external layer that names one of the owner's accounts, an approval per
repository can be reused after the stack changes, and falling back to the default account sends
client work through the personal login.

## Decision

- `alz account allow` records the repository's real root, the account per target and the
  declaring layer's source. Any change skips the layer's account, warns with the old and new
  values and asks for allow again. It is accepted only by a person on a terminal, typing the
  account name: it refuses without a TTY, inside an agent session, and has no `--yes`.
- A named account that cannot apply fails closed: `alz run` and `alz use` stop before the tool
  starts; the hook sets the variable to the named folder, the recorded account (cache miss with
  the same source) or a blocked folder that never exists, never the default account.
- `alz account init` shares configuration only (later narrowed to nothing by decision-17).

## Consequences

Amends decision-15. §7.13 and TASK-50.2 to TASK-50.6 carry these rules; the allow state lives in
an alz-only `allowed.yaml` under `XDG_STATE_HOME`. Agents that can write outside the repository
can still edit that file, so deny rules and a sandbox are recommended as the real protection.
