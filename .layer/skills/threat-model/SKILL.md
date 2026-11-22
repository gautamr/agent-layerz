---
name: threat-model
description: The agent-layerz threat model and review checklist. Use when reviewing or designing anything that reads repository or layer content, writes or deletes files, runs git or another subprocess, emits hooks or permissions, handles secrets, or changes the leak guard.
---
# agent-layerz threat model

## Trust boundaries
| Input | Trust | Why |
| --- | --- | --- |
| The repository being built: `layer.yaml`, `layer.lock`, `.layer/`, existing files | Untrusted | People run `alz build` in repos they clone |
| Remote layers (`github:`, `git:`) | Trusted code, untrusted paths | A layer can ship hooks that agents run, so users choose layers deliberately; its file names and refs still need validation |
| The user's environment variables and flags | Trusted, with limits | Set by the person running `alz`. The agent-session markers (`CLAUDECODE` and five more) prove a session only by being set; their absence proves nothing. Only Claude Code and Codex sessions are detected, and the Codex markers come from its source only (owner check O5, a release gate). `XDG_*` variables that point away from their default while `__MISE_DIFF` or `DIRENV_DIFF` exists may come from repository content (mise, direnv), so they are not trusted; those two variables are checked for existence only, never read |
| `accounts.yaml` (`${XDG_CONFIG_HOME:-~/.config}/agent-layerz/`) | Trusted only when the §7.13 'Both files' checks pass | alz never writes it. The owner writes it by hand, but any process running as the user (an agent) can edit it, so it is ignored inside the repo, in an account folder, when not owned by the user, writable by group or others, or located by an `XDG_CONFIG_HOME` moved while mise or direnv is active; names and paths in it are validated |
| `allowed.yaml` (`${XDG_STATE_HOME:-~/.local/state}/agent-layerz/`) | Trusted only when the same checks pass, and an entry counts only while it equals the current declaration | Only `alz account allow` writes it, after a person types the account name on a terminal outside an agent session; the writer refuses every state the reader ignores, a file inside a git work tree and a link to a file that is not empty, `[]` or alz's own. An agent that can write outside the repo can still edit it: the sandbox is the guard (§7.13 'Limit', target-facts §12 item 21). The terminal and marker gate is a speed bump: an agent may be able to open a real terminal through `tmux`, `open` or `osascript` (owner check O8) |
| Account folders `~/.claude*`, `~/.codex*` | Off limits | Read directory names only, never file contents; never written |

## Checklist
1. **Paths.** Every write, move and delete goes through `insideRoot`, except the one file
   outside the repo: `allowed.yaml`, its lock and its temporary file, written only by
   `alz account allow` through `src/resolve/account-allow-writer.ts` at a path built from
   `$HOME`, `XDG_STATE_HOME` and fixed names. That file never lies inside the repo, an account
   folder or a git work tree or, for a linked `allowed.yaml`, the directory of its real path,
   which passes the same checks; missing parent folders are created with mode 0700. Paths from manifests, locks or layers are
   relative, have no `..`, no absolute or drive prefix, no backslashes.
2. **Subprocesses.** Arguments come as an array, never a shell string. A value from untrusted
   input that git or another tool could read as an option is either validated (refs, SHAs)
   or placed after `--` (URLs).
3. **Locks.** Lock entries can steer deletes and cache reads; they are validated when read.
4. **Secrets.** No secret value is stored in a layer or generated file. MCP servers reference
   environment variables by name. Public layers pass the secret scanner and their
   `forbid_patterns`.
5. **Security floor.** A more specific layer can never remove a deny rule or a locked hook,
   and never widen an agent's tools without `allow_widen` in a non-public layer.
6. **Adoption.** It plans everything before writing; any problem means nothing changes. It
   never copies a literal environment value.
7. **Symlinks.** Layer files that are symlinks are skipped, so a layer cannot point outside itself.
8. **Person-only commands.** `alz account allow` and `--remove` refuse without a TTY on
   standard input or with any agent-session marker set, before any file system call. They
   have no `--yes`, read the answer only through `Io.terminal` (`src/cli/terminal.ts` is the
   only file that reads standard input), never print an answer, and print repository text
   only through `quote` or `stripControl`; text a person decides on (the source in the
   prompt) is escaped instead (`quoteEscaped`), so an invisible character or a look-alike
   letter shows.

## Known findings (fixed in 0.1.0, keep their regression tests)
- A crafted `layer.lock` path made `alz build` delete files outside the repo.
- `git:--upload-pack=…` reached `git clone` as an option.
- Manifest refs starting with `-` reached `git rev-parse` as options.
- A crafted lock SHA pointed the cache read at any directory.
Tests: `test/acceptance/24-untrusted-repo.test.ts`.
