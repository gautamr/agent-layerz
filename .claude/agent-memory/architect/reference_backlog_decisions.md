---
name: backlog-decision-cli-title-only
description: backlog decision create/edit only set title and status; standing exception to hand-fill bodies of decisions created here; how to amend an accepted decision; titles under ~150 chars; ### headings in plans
metadata:
  type: reference
---

`backlog decision create <title> [-s status]` and `backlog decision edit` (status only) cannot fill the
Context/Decision/Consequences body.

The file name is built from the title, so a title of about 250 characters or more fails with ENAMETOOLONG
and creates nothing (seen 2026-11-22, decision-18). Keep titles under about 150 characters.

Standing exception (owner, commit d1a7c15, written into `.layer/instructions/50-repo.md` "Work"): fill the
Context, Decision and Consequences of a decision created in this session by hand, committed with its task.
Never change a decision's title or status, or any other backlog file, by hand. Used for decision-20
(TASK-50.11, 2026-11-22).

Amending an accepted decision (owner-approved tightenings, 2026-11-22: decision-24, -26, -27, -28 after
the R0 security review) is the one case where its body changes after acceptance: add an "**Amended <date>
(owner, <reason>).**" note inside each affected section, mark replaced bullets "(amended below)" or
replace a now-wrong bullet and say in the note what was removed; title and status stay. A changed
*direction* is still a new superseding decision (record-decision skill). Check the caller's premises
against the code first: the brief claimed the TS reference keeps `!!binary` as a string, a smoke run
showed it decodes to bytes.

A decision the owner did not state is `proposed`; the record-decision skill then wants the task labelled
`needs:owner-approval` plus a comment with options and a default.

Plans set with `backlog task edit --plan` sit inside `## Implementation Plan`; use `###` headings in the plan
text so they are not mistaken for task sections.

Creating many subtasks from one script: macOS `/bin/bash` 3.2 fails with "syntax error near unexpected
token `('" on a `VAR=$(cat <<'EOF' … EOF)` heredoc whose body holds parentheses (TASK-55.3, 2026-11-23).
Use `read -r -d '' VAR <<'EOF' || true` instead, run `bash -n` first, and take each new ID from
`backlog task create … --plain | grep -o 'TASK-[0-9.]*'` to chain `--dep`. `backlog task edit --dep`
replaces the dependency list; it does not add to it.

Crate sources the YAML spike downloaded (serde-saphyr 2.2.0, granit-parser 2.2.0, sha2 0.10 and 0.11,
regex) are in `~/.cargo/registry/src/index.crates.io-*/`: read API facts there before fetching. Reading
the source makes a fact read-only knowledge, not a run, so it still needs a pinning test.

**How to apply:** write a self-contained decision title (what, where, the boundary, the task ID), fill the
body by hand, and point to the decision ID from the task plan or notes. See [[spec-edits-while-planning]].

The board's statuses are only To Do, In Progress and Done (seen 2026-11-23, R3.1). `-s "In Review"` fails with "Invalid status" and applies none of the edit's other changes, so check ACs and notes in a separate call or drop the `-s`. A finished task with no review yet stays In Progress, with the final summary written.
`backlog task edit --acceptance-criteria` (repeatable) replaces every criterion. Rewording one criterion means passing all the others with their text unchanged. A Python script calling `subprocess.run(["backlog", ...])` avoids shell quoting of `$HOME` and apostrophes.

Titles (2026-11-24, TASK-55.9 replan): a `§` in a title goes into the task's file name (git shows it
quoted), and `backlog task edit -t` changes the title but does not rename the file. Write "sections 4 to 6"
in titles, not "§4 to §6".
