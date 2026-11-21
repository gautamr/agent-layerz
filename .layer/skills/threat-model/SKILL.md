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
| The user's environment variables and flags | Trusted | Set by the person running `alz` |
| Account folders `~/.claude*`, `~/.codex*` | Off limits | Read directory names only, never file contents; never written |

## Checklist
1. **Paths.** Every write, move and delete goes through `insideRoot`. Paths from manifests,
   locks or layers are relative, have no `..`, no absolute or drive prefix, no backslashes.
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

## Known findings (fixed in 0.1.0, keep their regression tests)
- A crafted `layer.lock` path made `alz build` delete files outside the repo.
- `git:--upload-pack=…` reached `git clone` as an option.
- Manifest refs starting with `-` reached `git rev-parse` as options.
- A crafted lock SHA pointed the cache read at any directory.
Tests: `test/acceptance/24-untrusted-repo.test.ts`.
