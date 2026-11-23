import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { tool } from "../exec";
import { fixture, layers, readTree, snapshot, type Workspace } from "../helpers";
import { alzBin, tempDir } from "../run-root";

// §13.28: what a command that resolves the stack refuses or leaves alone, seen through `alz lint`
// and, where the criterion names it, `alz build` and `alz check` (TASK-52, TASK-53, TASK-63,
// TASK-69). The build and check forms are checked against the Rust alz again in R5 (TASK-55.6):
// until then they fail against it, and against the reference they are the expected failures.

const FIXTURE = "28-cache-integrity";
const GIT_ENV = {
  PATH: process.env.PATH ?? "",
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_AUTHOR_NAME: "Test",
  GIT_AUTHOR_EMAIL: "test@example.invalid",
  GIT_COMMITTER_NAME: "Test",
  GIT_COMMITTER_EMAIL: "test@example.invalid",
};
const FORMS: [string, string[]][] = [
  ["lint", ["lint"]],
  ["build", ["build"]],
];

function sh(ws: Workspace, cwd: string, args: string[]): string {
  return tool("git", ["-c", "commit.gpgsign=false", ...args], {
    cwd,
    env: { ...GIT_ENV, HOME: ws.home },
  }).trim();
}

/** A git repository holding a copy of the fixture layer `from`, committed on `main`. */
function remote(ws: Workspace, from: string, dest: string): string {
  const path = `${ws.root}/${dest}`;
  cpSync(`${ws.root}/${from}`, path, { recursive: true });
  sh(ws, path, ["init", "--quiet", "--initial-branch=main"]);
  sh(ws, path, ["add", "."]);
  sh(ws, path, ["commit", "--quiet", "-m", "layer"]);
  return path;
}

function stack(ws: Workspace, ...sources: string[]): void {
  const entries = [...sources, "path:./.layer"].map((source) => `  - source: ${source}\n`);
  writeFileSync(`${ws.root}/repo/layer.yaml`, `targets: [claude]\nstack:\n${entries.join("")}`);
}

const LAYER_YAML = "name: inner\nkind: group\nvisibility: private\nversion: 1.0.0\n";
const REFUSED = (source: string): string =>
  `error [source] layer '${source}' touches an account folder or the home folder; alz never reads there`;

describe.each(FORMS)(
  "a layer in or around an account folder, through alz %s (TASK-52)",
  (_name, args) => {
    /** The folder is mode 000 while `run` runs, so a read of anything in it would fail loudly. */
    async function sealed<T>(folder: string, run: () => Promise<T>): Promise<T> {
      chmodSync(folder, 0o000);
      try {
        return await run();
      } finally {
        chmodSync(folder, 0o700);
      }
    }

    referenceDrift(
      "TASK-52",
      "refuses a layer that is the home folder, and leaves it unchanged",
      async () => {
        const ws = fixture(FIXTURE);
        writeFileSync(`${ws.home}/layer.yaml`, LAYER_YAML);
        stack(ws, "path:../../home");
        const home = snapshot(ws.home);
        const repo = readTree(`${ws.root}/repo`);
        const result = await layers(ws, "repo", args);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain(REFUSED("path:../../home"));
        expect(snapshot(ws.home)).toEqual(home);
        expect(readTree(`${ws.root}/repo`)).toEqual(repo);
      },
    );

    referenceDrift("TASK-52", "refuses a layer above the home folder", async () => {
      const ws = fixture(FIXTURE);
      stack(ws, "path:../..");
      const result = await layers(ws, "repo", args);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(REFUSED("path:../.."));
    });

    referenceDrift(
      "TASK-52",
      "refuses a layer reached through a link into an account folder, before looking inside it",
      async () => {
        const ws = fixture(FIXTURE);
        const account = `${ws.home}/.claude-x`;
        mkdirSync(account);
        writeFileSync(`${account}/layer.yaml`, LAYER_YAML);
        symlinkSync(account, `${ws.root}/repo/linked`);
        stack(ws, "path:./linked");
        // A name in the folder cannot be looked at (mode 000): the refusal comes before any lookup.
        const result = await sealed(account, () => layers(ws, "repo", args));
        expect(result.code).toBe(1);
        expect(result.stderr).toContain(REFUSED("path:./linked"));
        expect(readdirSync(account)).toEqual(["layer.yaml"]);
      },
    );

    referenceDrift("TASK-52", "refuses a layer cache inside an account folder", async () => {
      const ws = fixture(FIXTURE);
      const account = `${ws.home}/.claude-x`;
      mkdirSync(account);
      const a = remote(ws, "layer-a", "remotes/team-a");
      stack(ws, `git:${a}@main`);
      const result = await layers(ws, "repo", args, { XDG_CACHE_HOME: `${account}/cache` });
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("error [cache] the layer cache at ");
      expect(result.stderr).toContain(
        "touches an account folder or the home folder; set XDG_CACHE_HOME to another folder",
      );
      expect(readdirSync(account)).toEqual([]);
    });
  },
);

describe.each(FORMS)(
  "an empty or relative XDG_CACHE_HOME, through alz %s (TASK-53)",
  (name, args) => {
    for (const [label, value] of [
      ["empty", ""],
      ["relative", "rel-cache"],
    ] as const) {
      referenceDrift(
        "TASK-53",
        `a ${label} value counts as unset: the cache is ~/.cache/agent-layerz`,
        async () => {
          const ws = fixture(FIXTURE);
          const a = remote(ws, "layer-a", "remotes/team-a");
          stack(ws, `git:${a}@main`);
          const repo = snapshot(`${ws.root}/repo`);
          const result = await layers(ws, "repo", args, { XDG_CACHE_HOME: value });
          expect(result.code, result.stderr).toBe(0);
          const entries = readdirSync(`${ws.home}/.cache/agent-layerz`);
          expect(entries).toHaveLength(1);
          expect(entries[0]).toMatch(/^git-[0-9a-f]{64}@[0-9a-f]{40}$/);
          // Nothing is made in the working directory, which is the repository.
          expect(existsSync(`${ws.root}/repo/agent-layerz`)).toBe(false);
          expect(existsSync(`${ws.root}/repo/rel-cache`)).toBe(false);
          if (name === "lint") expect(snapshot(`${ws.root}/repo`)).toEqual(repo);
        },
      );
    }
  },
);

describe.each([
  ["lint", "lint"],
  ["check", "check"],
])("alz %s as a pre-commit hook (TASK-63)", (_name, command) => {
  /**
   * A pre-commit hook that snapshots the whole repository (every file under the main `.git`,
   * the index included) before and after it runs alz, in its own files, then lets the commit go.
   * Git gives a hook GIT_INDEX_FILE and, in a linked worktree, GIT_DIR.
   */
  function hook(ws: Workspace, main: string, out: string): void {
    const script = [
      "#!/bin/sh",
      `snap() { (cd '${main}/.git' && find . -type f ! -name '*.lock' | LC_ALL=C sort | while IFS= read -r f; do printf '%s %s\\n' "$f" "$(cksum < "$f")"; done); }`,
      `snap > '${out}/before'`,
      `'${alzBin()}' ${command} > '${out}/stdout' 2> '${out}/stderr'`,
      `echo $? > '${out}/code'`,
      `snap > '${out}/after'`,
      "exit 0",
      "",
    ].join("\n");
    writeFileSync(`${main}/.git/hooks/pre-commit`, script, { mode: 0o755 });
    void ws;
  }

  /** A repository whose stack has a git layer, with the hook installed. */
  function repository(): { ws: Workspace; main: string; out: string; env: Record<string, string> } {
    const ws = fixture(FIXTURE);
    const a = remote(ws, "layer-a", "remotes/team-a");
    stack(ws, `git:${a}@main`);
    const main = `${ws.root}/repo`;
    sh(ws, main, ["init", "--quiet", "--initial-branch=main"]);
    sh(ws, main, ["add", "."]);
    sh(ws, main, ["commit", "--quiet", "-m", "initial"]);
    const out = tempDir("alz-hook-");
    hook(ws, main, out);
    return {
      ws,
      main,
      out,
      env: { ...GIT_ENV, HOME: ws.home, XDG_CACHE_HOME: `${ws.root}/cache` },
    };
  }

  function expectUntouched(out: string): void {
    const read = (name: string): string => tool("cat", [`${out}/${name}`]);
    expect(read("after")).toBe(read("before"));
    expect(read("before")).toContain("./index ");
    // A usage error (exit 2) means alz never ran the command, which proves nothing.
    expect(read("code").trim(), read("stderr")).not.toBe("2");
    if (command === "lint") expect(read("code").trim(), read("stderr")).toBe("0");
  }

  // The reference leaves the index alone in a main checkout (the case passes there); it fails in
  // a linked worktree, where the hook's GIT_DIR reaches its cache calls.
  it("leaves the index and the repository byte-identical in a main checkout", async () => {
    const { ws, main, out, env } = repository();
    writeFileSync(`${main}/notes.md`, "notes\n");
    sh(ws, main, ["add", "notes.md"]);
    tool("git", ["-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "notes"], {
      cwd: main,
      env,
    });
    expectUntouched(out);
  });

  referenceDrift(
    "TASK-63",
    "leaves the index and the repository byte-identical in a linked worktree",
    async () => {
      const { ws, main, out, env } = repository();
      const worktree = `${ws.root}/wt`;
      sh(ws, main, ["worktree", "add", "--quiet", worktree, "-b", "wt"]);
      writeFileSync(`${worktree}/notes.md`, "notes\n");
      sh(ws, worktree, ["add", "notes.md"]);
      tool("git", ["-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "notes"], {
        cwd: worktree,
        env,
      });
      expectUntouched(out);
    },
  );
});

// A name that is not valid UTF-8 can be made only where the file system takes any bytes: Linux.
const onlyOnLinux =
  process.platform === "linux"
    ? referenceDrift
    : (_task: string, name: string) => it.skip(name, () => {});

describe.each(FORMS)(
  "a layer file name that is not valid UTF-8, through alz %s (TASK-69)",
  (_name, args) => {
    const bad = (dir: string, ...rest: string[]): Buffer =>
      Buffer.concat([
        Buffer.from(`${dir}/`),
        ...rest.map((part) => Buffer.from(part)),
        Buffer.from([0xff]),
      ]);

    for (const [label, make] of [
      ["a file", (dir: string) => writeFileSync(bad(dir, "instructions/x"), "x\n")],
      ["a folder", (dir: string) => mkdirSync(bad(dir, "instructions/d"))],
    ] as const) {
      onlyOnLinux(
        "TASK-69",
        `is the error layer when ${label} is named with the byte 0xFF`,
        async () => {
          const ws = fixture(FIXTURE);
          const dir = `${ws.root}/layer-a`;
          make(dir);
          stack(ws, "path:../layer-a");
          const result = await layers(ws, "repo", args);
          expect(result.code).toBe(1);
          expect(result.stderr).toContain(
            "error [layer] a file or folder name in layer 'path:../layer-a' is not valid UTF-8; rename it",
          );
          expect(result.stdout + result.stderr).not.toContain("�");
        },
      );
    }
  },
);
