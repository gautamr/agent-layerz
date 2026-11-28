<!-- Title: a Conventional Commit with the task ID, for example `feat(adopt): adopt hand-written hooks (task-12)`. -->

## Task

<!-- The board task this pull request does: TASK-<n>. One task, one pull request. -->

## Summary

<!-- What changes for a user or a contributor, and why. -->

## Spec and laws

- [ ] Behaviour changes update `docs/LAYERING-SPEC.md` in this pull request, or there is no behaviour change.
- [ ] A new or changed merge structure states its laws and has a property test for each (docs/algebra.md).
- [ ] Adapter behaviour rests only on facts recorded in `docs/target-facts.md`.

## Checks

- [ ] `mise run rust:ci`
- [ ] `mise run alz:check` (`-- --skip-identity` if you are not the owner)
- [ ] `mise run workflows:lint`, if a workflow changed
- [ ] Generated files changed only through `.layer/` and `mise run alz:build`

<!-- Paste the commands you ran and their results. Name anything you did not run, and why. -->

## Safety

- [ ] No client, employer or private project names, secrets or paths from your machine
- [ ] A change that reads layer content, writes or deletes files, runs a subprocess, or handles hooks, permissions or secrets says so, for a security review
