import { accessSync, constants, readFileSync, statSync, writeFileSync } from "node:fs";
import { delimiter, join } from "node:path";
import { tempDir } from "./run-root";

/**
 * A `git` stand-in for tests that prove a command never runs git (or that another one does).
 *
 * The stand-in is a small shell script in a fresh temporary directory. It appends the working
 * directory and every argument of each call to a log, then replaces itself with the real git,
 * so what a command does with git is unchanged. The real git is looked up on the caller's PATH
 * before the stand-in's directory is put in front of it, and its absolute path is written into
 * the script. A binary under test is made to find the stand-in with `proxy.env`.
 *
 * Language-neutral: it needs only `/bin/sh` and the environment, so it drives any alz binary.
 *
 * It sees only a PATH lookup of `git`: a binary that runs git by an absolute path, or links a git
 * library (libgit2, gitoxide), bypasses it and "no git was run" proves nothing. The Rust alz must
 * keep running git as a PATH-resolved subprocess (decision-24), or this proxy is replaced.
 */

export interface GitCall {
  cwd: string;
  args: string[];
}

export interface GitProxy {
  /** `PATH` with the stand-in's directory first: pass it to the command under test. */
  env: { PATH: string };
  /** Every call recorded so far, oldest first. */
  calls: () => GitCall[];
}

/** The first executable regular file called `name` on `path`, as an absolute path. */
export function resolveOnPath(name: string, path: string): string | undefined {
  return path
    .split(delimiter)
    .filter((dir) => dir.startsWith("/"))
    .map((dir) => join(dir, name))
    .find((candidate) => {
      try {
        accessSync(candidate, constants.X_OK);
        return statSync(candidate).isFile();
      } catch {
        return false;
      }
    });
}

/** A single-quoted shell word: safe for any path, including one with a quote in it. */
const shellWord = (text: string): string => `'${text.replaceAll("'", `'\\''`)}'`;

const script = (log: string, realGit: string): string =>
  [
    "#!/bin/sh",
    "{",
    `  printf 'c:%s\\0' "$PWD"`,
    `  for a in "$@"; do printf 'a:%s\\0' "$a"; done`,
    `  printf 'e:\\0'`,
    `} >> ${shellWord(log)}`,
    `exec ${shellWord(realGit)} "$@"`,
    "",
  ].join("\n");

/** The calls in a log: one `c:` record, any number of `a:` records, then `e:`. Pure. */
export function parseGitCalls(log: string): GitCall[] {
  const records = log.split("\0").filter((record) => record !== "");
  return records.reduce<{ done: GitCall[]; open: GitCall | undefined }>(
    ({ done, open }, record) => {
      const tag = record.slice(0, 2);
      const text = record.slice(2);
      if (tag === "c:") return { done, open: { cwd: text, args: [] } };
      if (tag === "a:" && open) return { done, open: { ...open, args: [...open.args, text] } };
      if (tag === "e:" && open) return { done: [...done, open], open: undefined };
      return { done, open };
    },
    { done: [], open: undefined },
  ).done;
}

/** Installs the stand-in. `realPath` is where the real git is found (default: this process's PATH). */
export function gitProxy(realPath: string = process.env.PATH ?? ""): GitProxy {
  const realGit = resolveOnPath("git", realPath);
  if (realGit === undefined) throw new Error("no git on PATH: the stand-in has nothing to run");
  const dir = tempDir("alz-git-proxy-");
  const log = join(dir, "calls.log");
  writeFileSync(log, "");
  writeFileSync(join(dir, "git"), script(log, realGit), { mode: 0o755 });
  return {
    env: { PATH: `${dir}${delimiter}${realPath}` },
    calls: () => parseGitCalls(readFileSync(log, "utf8")),
  };
}
