---
id: doc-7
title: 'Autonomy levels: proposal for a neutral, configurable autonomy setting'
type: specification
created_date: '2026-11-25T19:01:00Z'
updated_date: '2026-11-25T19:10:00Z'
---
# Autonomy levels: proposal for a neutral, configurable autonomy setting

**Status:** proposal for the owner's review (2026-11-26). Nothing here is in `docs/LAYERING-SPEC.md` yet. TASK-116 writes it into the spec after the owner answers §15. Milestone m-8; it depends on decision-40 (proposed) for the coverage report and the adapter mappings.

## 1. Scope

This proposal covers one setting that runs from the strictest level to fully autonomous. It is neutral across tools, each adapter maps it, and layers configure it the way they configure every other item.

It covers two kinds of user input:
- **(a) choice questions** that the model asks, such as Claude Code's AskUserQuestion: several options and a recommendation;
- **(b) yes/no permission prompts** that come from the tool's permission settings, such as editing a file in another folder or committing.

Out of scope:
- managed or user-scope settings, which alz never writes (§8.4);
- branch protection on the hosting service, which enforces part of the Never row but is not alz output.

## 2. What exists today

- **Delivery-protocol §4 (personal layer skill).** It has four rows: decide alone and note it in the task; decide, then notify; stop and ask the owner; never (enforced by permissions, hooks and branch protection). "When unsure, use the stricter row", and profiles may override rows explicitly. §5 says how to escalate: label the task, comment with the question, two or three options and a recommendation, move on to another task, and never wait idle. The skill is static text, the same in every repository.
- **Spec §7.6.** It covers `permissions.yaml`: allow, ask and deny lists combined across layers, where deny always wins (the permission floor). It has a `default_mode` with an optional `floor: true`, and the three boolean floor fields (`disable_bypass_mode`, `block_reads_outside`, `block_writes_outside`). Modes from loosest to strictest: `bypass_permissions`, `auto`, `accept_edits`, `default`, `dont_ask`, `plan`.
- **No item expresses "how autonomous is this repository"**, and nothing ties a decision row to the permission prompt for the same action.

## 3. The two kinds of input are governed differently

| | (a) Choice questions | (b) Permission prompts |
| --- | --- | --- |
| Who triggers it | The model decides to ask | The tool, when an action matches no allow rule (or matches an ask rule) |
| What governs it | Instructions; at most a switch that removes the tool | Settings: permission mode, allow/ask/deny rules, sandbox, additional directories (Claude); approval policy and sandbox mode (Codex) |
| What alz writes | A decision table and the "how to ask" text for the level, in `AGENTS.md` (and the tool name in `CLAUDE.md`) | Native settings keys, through each adapter |

The proposal ties both together through **decision classes** (§5). A class names a kind of decision and, optionally, the neutral permission rules of the actions it covers. The row a class ends up in at the effective level decides two things: what the instructions tell the model, and which permission list its rules go into.

## 4. The levels

From strictest to most autonomous:

| Level | Who decides | (a) Choice questions | (b) Permission prompts |
| --- | --- | --- | --- |
| `advisory` | The human decides everything; the agent changes nothing | Every choice is a question with options and a recommendation | The session is read-only or in plan mode |
| `supervised` | The human approves each change | Ask before each change that is not purely mechanical | Each edit and each command outside the allow list prompts |
| `collaborative` | delivery-protocol §4 as written (the reference level) | Ask only for the "stop and ask" row | Edits inside the working folders run without a prompt; commands outside the allow list prompt |
| `delegated` | The agent decides more and the human is sometimes there | Shiftable "ask" classes become "notify" | Sandboxed commands run without a prompt; writes outside the working folders are blocked, not prompted |
| `autonomous` | Nobody is watching the session | Never wait for an answer; questions go to the task as comments (delivery-protocol §5) | Nothing prompts; anything not allowed is refused; sandbox required |

The Never row is the same at every level, and no level writes a bypass mode.

## 5. Decision classes and how levels shift them

Layers define the classes. alz ships none, because the content belongs to layers; the personal layer would port delivery-protocol §4 into its `autonomy.yaml`. A class has:
- `text`: the decision, as the table prints it;
- `row`: `alone`, `notify`, `ask` or `never` at the reference level (`collaborative`);
- `fixed: true` (optional): an `ask` class that no level shifts, such as accepting a decision record, a roadmap gate, payments or production;
- `rules` (optional): neutral permission rules of §7.6 for the actions the class covers, such as `bash:git commit *`.

How each level shifts the reference row:

| Reference row | advisory | supervised | collaborative | delegated | autonomous |
| --- | --- | --- | --- | --- | --- |
| alone | ask | notify | alone | alone | alone |
| notify | ask | ask | notify | notify | notify |
| ask | ask | ask | ask | notify | notify |
| ask, `fixed` | ask | ask | ask | ask | ask, escalated without waiting |
| never | never | never | never | never | never |

Where a class's `rules` go at the effective level: `alone` and `notify` put them in `allow`, `ask` puts them in `ask`, and `never` puts them in `deny`. A deny from any layer still wins (§7.6), and a rule that is both allowed and denied keeps the `permission-shadowed` warning.

## 6. Where the setting lives

A new item file in each layer, `autonomy.yaml` (§4.1 layout). The repository chooses its level in `.layer/autonomy.yaml`, like every other item.

```yaml
level: delegated          # this layer's choice; the most specific layer wins
max: delegated            # optional ceiling for every more specific layer
classes:
  dependency-intake:
    text: Adding an open-source library that passes the intake
    row: notify
  decision-records:
    text: Accepting or changing a decision record
    row: ask
    fixed: true
  commit:
    text: Committing on a feature branch
    row: alone
    rules: ["bash:git commit *"]
  push-main:
    text: Push to the main branch or force-push
    row: never
    rules: ["bash:git push --force*", "bash:git push origin main*"]
```

Alternative (owner question Q2): an `autonomy:` block inside `permissions.yaml`.

## 7. Merge rule down the stack

The levels form a total order, `advisory < supervised < collaborative < delegated < autonomous`.
- **`level`** follows last-writer-wins: the most specific layer that sets it wins.
- **`max`** is a meet. The effective ceiling is the lowest `max` of all layers, and is `autonomous` when none is set. A more specific layer may lower the ceiling but never raise it. Raising it is the error `autonomy-floor`, so nobody believes they raised it.
- **Check.** A layer's `level` above the ceiling of the layers more general than it, or above its own `max`, is the error `autonomy-floor` naming both layers: `level <x> of layer <a> is above the ceiling <y> set by layer <b>`. This fails closed, like the three floor fields of §7.6, and is not clamped (owner question Q9).
- **Classes** are merged by id (a map union).
  - `text` follows last-writer-wins.
  - `row` follows last-writer-wins for classes that are neither `never` nor `fixed`, so a repository can loosen a class that is not fixed.
  - `never` absorbs: once any layer says `never`, the class stays `never`.
  - `fixed` is sticky: once set, it stays.
  - A more specific layer may always make a class stricter.
  - Changing a `never` or `fixed` class to a looser row is the error `autonomy-floor`.
  - `rules` combine as a union. A `never` class's rules can never be removed, the same as a deny.
- **Unset.**
  - When no layer has `autonomy.yaml`, nothing is rendered and the output is byte for byte what it is today (owner question Q3).
  - When a layer has one but no layer sets `level`, the level is `collaborative`, lowered to the ceiling.

**How a repository gets more autonomy where it is allowed:**
1. A higher `level`, up to the ceiling that the personal and client layers set.
2. Loosening a class that is not fixed (`row: alone` for "test-only changes", the way delivery-protocol profiles override rows today).

A client layer that wants to forbid this sets `max`, `fixed` or `never`. Those cannot be undone further down the stack.

## 8. Interaction with permissions.yaml, settings.yaml and the workspace

- The level gives a neutral `default_mode`:

  | advisory | supervised | collaborative | delegated | autonomous |
  | --- | --- | --- | --- | --- |
  | `plan` | `default` | `accept_edits` | `accept_edits` | `dont_ask` |

  It counts as declared by the layer that set the level, so §7.6 applies unchanged: a `default_mode` floor in a more general layer refuses a looser mode. An explicit `default_mode` in the same layer or a more specific one wins, as today. The lint warns `autonomy-mode` when the explicit mode contradicts the level.
- Generated sandbox and floor keys (§9) count as declared by the layer that set the level, under the floor rows of §7.9. So a level only ever adds the stricter value of a floor key, and never removes a stricter value set by a layer.
- `workspace.add_dirs` keeps its mapping (Claude `permissions.additionalDirectories`). Codex's `sandbox_workspace_write.writable_roots` takes absolute paths only (target-facts §13.6), so a deterministic build cannot write it, and it becomes a warning (§10).

## 9. Claude mapping of (b)

| Level | `permissions.defaultMode` | sandbox | floor keys | baseline allow |
| --- | --- | --- | --- | --- |
| advisory | `plan` | none | none | none |
| supervised | `default` | none | none | none |
| collaborative | `acceptEdits` | none | none | none |
| delegated | `acceptEdits` | `sandbox.enabled: true`, `sandbox.autoAllowBashIfSandboxed: true`, `sandbox.allowUnsandboxedCommands: false` | `permissions.blockWritesOutsideWorkingDirectories: true` | none |
| autonomous | `dontAsk` | the delegated keys plus `sandbox.failIfUnavailable: true` | `permissions.disableBypassPermissionsMode: "disable"`, `permissions.blockWritesOutsideWorkingDirectories: true` | `Read`, `Grep`, `Glob`, `Edit`, `Write` (from the neutral `read`, `grep`, `glob`, `edit`, `write`) |

- At every level, the class rules go to `permissions.allow`, `ask` or `deny` by row (§5), and `workspace` dirs go to `permissions.additionalDirectories` as today.
- **Editing files in other folders:**
  - inside `workspace` dirs, it is allowed by `additionalDirectories`;
  - outside them, it prompts up to `collaborative` (CF6) and is blocked from `delegated` on (`blockWritesOutsideWorkingDirectories`, recorded).
- **Committing** is the class `commit` and its rules, so it follows the class's row.
- **Never written, at any level:** `bypassPermissions` or `auto` (no effect from project files anyway, target-facts §13.4), `skipDangerousModePermissionPrompt`, `sandbox.allowUnsandboxedCommands: true`.
- Each cell is written only once its fact is recorded (§12).

## 10. Codex mapping of (b)

| Level | `sandbox_mode` | `approval_policy` | other |
| --- | --- | --- | --- |
| advisory | `read-only` | `on-request` | none |
| supervised | `read-only` | `on-request` | none (Codex config cannot tell supervised from advisory; the instructions do) |
| collaborative | `workspace-write` | `on-request` | `sandbox_workspace_write.network_access: false` |
| delegated | `workspace-write` | `on-request` | the same |
| autonomous | `workspace-write` | `never` | the same |

- **Never written:** `danger-full-access`, `approval_policy = "untrusted"` (refused at load, target-facts §13.5), `approvals_reviewer = "auto_review"` (owner question Q11).
- **Class rules:** Codex config has no allow/ask/deny lists (target-facts §8 item 6). Rules files and permission profiles are UNVERIFIED (XF9). Until XF9 is recorded, every class rule is reported as "not expressed" in the coverage report of decision-40, with the warning `unenforced` naming the rule (owner question Q7).
- **Additional directories:** `workspace.add_dirs` cannot be written deterministically, and gives the warning `unenforced`.

## 11. Choice questions (a): what alz renders, and the choice tool

**The section in `AGENTS.md`.** Every target reads it: Codex loads `AGENTS.md` even in an untrusted project (target-facts §6.1, §6.6), and Claude reads it through `@AGENTS.md` (§3). It is shown here at `delegated`:

```markdown
## Autonomy: delegated

Set by layer `repo`. The most autonomous level allowed here is `delegated`, set by layer `client`.
Before each decision that is not purely mechanical, find its row. When unsure, use the stricter row.

| Row | Decisions |
| --- | --- |
| Decide alone, note it in the task | Committing on a feature branch; … |
| Decide, then notify (task notes and pull request) | Adding an open-source library that passes the intake; … |
| Stop and ask | Accepting or changing a decision record; … |
| Never | Push to the main branch or force-push; … |

How to ask: <the text of the level, below>
```

**How to ask, per level:**
- `advisory`: "Change nothing. Present your plan. For each choice, ask one question with two or three options and your recommendation, as a choice question if your tool has one."
- `supervised`: "Ask before each change that is not purely mechanical, one question at a time, with two or three options and your recommendation, as a choice question if your tool has one."
- `collaborative`, `delegated`: "When a row says ask, ask one question with two or three options and your recommendation, as a choice question if your tool has one. If nobody answers, record the question, the options and your recommendation in the task, and move on to other work."
- `autonomous`: "Nobody is watching this session. Never wait for an answer and never end your turn on a question. For a 'Stop and ask' row, record the question, two or three options and your recommendation in the task, mark it for the owner, leave that work and move on to other ready work. For a 'Decide, then notify' row, take your recommended option and say so in the task and the pull request."

The table and the text count against `instructions_max_lines`. The alternative is a generated skill (owner question Q5).

**The Claude-specific block in `CLAUDE.md`,** written only once CF9 and CF10 are recorded:
- up to `delegated`: "Ask choice questions with the AskUserQuestion tool.";
- at `autonomous`: "Do not use AskUserQuestion; nobody will answer."

**Can a setting govern the choice tool?**
- Claude:
  - (1) `permissions.deny: ["AskUserQuestion"]`, which by §7b's bare-name rule would remove the tool from the context. Proposed for `autonomous` only, and only once CF9 is recorded (owner question Q6).
  - (2) `disallowedTools` in an agent's frontmatter (CF9), through a new neutral tool `ask_user` in §7.2's vocabulary, so a layer can deny it per agent.
  - (3) `askUserQuestionTimeout` is read from user or managed scope only (target-facts §14.3), so alz cannot write it.
  - (4) Nothing makes the model use the tool. That part is instructions only.
- Codex: unknown (XF7). Instructions only until it is recorded.

## 12. Facts to verify before an adapter writes them

Status is what `docs/target-facts.md` records today. A row marked UNVERIFIED gets no output until TASK-114 (Claude) or TASK-115 (Codex) records it.

| ID | Fact the mapping needs | Recorded today |
| --- | --- | --- |
| CF1 | What each `permissions.defaultMode` value (`plan`, `default`, `acceptEdits`, `dontAsk`) prompts for, allows or refuses when set in a project file | Names and type SMOKE (§13.4); behaviour **UNVERIFIED** |
| CF2 | `auto` and `bypassPermissions` have no effect from project or local files | Recorded (§13.4) |
| CF3 | Whether a project `permissions.defaultMode` needs the workspace trust dialog | **UNVERIFIED** (only `allow` is recorded as needing trust, §7b) |
| CF4 | Under `dontAsk`, an action with no allow rule is refused without a prompt, and what an `ask` rule does there | **UNVERIFIED** |
| CF5 | Under `dontAsk`, whether `sandbox.autoAllowBashIfSandboxed: true` lets sandboxed Bash run without an allow rule | **UNVERIFIED** |
| CF6 | `acceptEdits` allows edits inside the working folders without a prompt and prompts outside them | **UNVERIFIED** |
| CF7 | `sandbox.enabled`, `failIfUnavailable`, `autoAllowBashIfSandboxed` (default `true` in 2.1.323, `false` from 2.1.324), `allowUnsandboxedCommands`: names and types | Recorded SMOKE (§13.1); effects DOCS |
| CF8 | `permissions.disableBypassPermissionsMode: "disable"`, `blockWritesOutsideWorkingDirectories`, `blockReadsOutsideWorkingDirectories` | Recorded SMOKE (§13.4) |
| CF9 | `AskUserQuestion` is the tool's name, a bare deny rule removes it, and agent `disallowedTools` accepts it | **UNVERIFIED** (the bare-name rule is DOCS for tools in general, §7b) |
| CF10 | Whether AskUserQuestion is available in `-p` runs and in subagents | **UNVERIFIED** |
| CF11 | `askUserQuestionTimeout` is read from user or managed scope only | Recorded DOCS (§14.3) |
| CF12 | A project `defaultMode: plan` starts sessions in plan mode, and how leaving plan mode asks the user | **UNVERIFIED** |
| CF13 | Deny, then ask, then allow; a deny in any file wins | Recorded DOCS and SMOKE (§7b) |
| CF14 | Relative paths in `permissions.additionalDirectories` | **UNVERIFIED** (§7c, already listed in spec §8) |
| XF1 | What `approval_policy = "on-request"` and `"never"` do | Values SMOKE and DOCS (§6.6, §13.5); behaviour **UNVERIFIED** |
| XF2 | `sandbox_mode` values `read-only`, `workspace-write`, `danger-full-access` | Recorded SMOKE (§6.7, §13.5); effect DOCS |
| XF3 | Whether `approval_policy` and `sandbox_mode` take effect from a trusted project's `.codex/config.toml` | **UNVERIFIED** (trusted behaviour, §6.6) |
| XF4 | The fields of the `granular` approval table | **UNVERIFIED** |
| XF5 | `sandbox_workspace_write.network_access`; `writable_roots` takes absolute paths only | Recorded (§13.6) |
| XF6 | `approvals_reviewer` values | Recorded (§13.6) |
| XF7 | Whether Codex has a model-initiated choice tool, and any setting that switches it | **UNVERIFIED** |
| XF8 | Whether a read-only or plan collaboration mode can be set in config | **UNVERIFIED** |
| XF9 | The syntax of rules files or permission profiles for deny rules | **UNVERIFIED** (§6.6, §9) |
| XF10 | `AGENTS.md` loads in an untrusted project | Recorded SMOKE (§6.1, §6.6) |

## 13. Laws the tests will state

1. **Ceiling.** For every stack, the effective level is at most every declared `max` (proptest, TASK-117).
2. **Never absorbs.** A class that is `never` in any layer is `never` at every level, and its rules are in every adapter's deny output or in its coverage report as not expressed (TASK-117, TASK-119).
3. **Fixed absorbs.** A `fixed` class is `ask` at every level.
4. **Monotone.** For levels l1 ≤ l2, every class's row at l2 is at most as strict as at l1.
5. **Identity.** A stack without `autonomy.yaml` renders byte-identical output (acceptance suite, `alz:check`).
6. **No bypass.** No level makes Claude write `bypassPermissions` or `auto`, or Codex write `danger-full-access` (proptest).
7. **Facts.** Every key a level writes passes the facts harness of decision-40 (TASK-112).

## 14. User-facing changes and version

- A new item file (`autonomy.yaml`), new codes (`autonomy`, `autonomy-floor`, `autonomy-mode`, `unenforced`), and a new `AGENTS.md` section and new settings keys, only when a layer opts in.
- They need a minor version bump and a CHANGELOG entry. With no opt-in, the output does not change.

## 15. Open questions for the owner

- **Q1. Names and count.** Five levels: `advisory`, `supervised`, `collaborative`, `delegated`, `autonomous`. Recommended: yes.
- **Q2. Place.** A new `autonomy.yaml` (recommended), or an `autonomy:` block in `permissions.yaml`.
- **Q3. Unset.** Recommended: no `autonomy.yaml` means no output change. The alternative is a default of `collaborative`, which changes every build.
- **Q4. Classes.** alz ships no classes, and the personal layer ports delivery-protocol §4 into `autonomy.yaml`. That is a change to the personal layer outside this repository; afterwards the skill would point to the generated table. Recommended: yes.
- **Q5. Where the table goes.** `AGENTS.md`, always loaded and counted in the budget (recommended), or a generated skill.
- **Q6.** At `autonomous`, deny AskUserQuestion once CF9 is verified? Recommended: yes, so a run nobody watches cannot block on a question.
- **Q7.** A `never` rule that a target cannot enforce (Codex today): the warning `unenforced` (recommended, matches §8.2's best-effort rule) or a build error.
- **Q8.** `permissions.disableBypassPermissionsMode: "disable"`: at `autonomous` only (recommended), or at every level once autonomy is declared, which also stops a person from using bypass mode in the repository.
- **Q9.** A level above the ceiling: an error (recommended, fail closed) or clamped with a warning.
- **Q10.** At `autonomous`, also set `blockReadsOutsideWorkingDirectories: true`? Recommended: no. It would break reads of toolchain caches outside the repository; secrets stay protected by deny rules and the sandbox.
- **Q11.** Codex `approvals_reviewer = "auto_review"` at `delegated`: it resembles Claude's auto mode but lets a model approve. Recommended: not written.

## 16. Tasks (milestone m-8)

- TASK-114: verify the Claude facts.
- TASK-115: verify the Codex facts.
- TASK-116: the spec (needs the owner's answers).
- TASK-117: parse and merge.
- TASK-118: the `AGENTS.md` section.
- TASK-119: the Claude mapping.
- TASK-120: `alz explain`.
- TASK-126: the Codex adapter with the Codex mapping, built on decision-40's contract.
