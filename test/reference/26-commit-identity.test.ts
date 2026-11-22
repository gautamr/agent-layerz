import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { git as runGit } from "../../src/resolve/git";
import { guardedClone, isInsideRepo } from "../../src/resolve/sources";
import { fixture, layers, readFile, readTree, type Workspace } from "../helpers";

// The cases of test/acceptance/26-commit-identity.test.ts that call the TypeScript implementation
// directly (`isInsideRepo`, `guardedClone`, `git`). Each keeps its name and body; the CLI side of
// each is in the acceptance file. Deleted with src/ at the cutover.

// Reserved addresses only: the wrong identity must never appear in any output.
const WRONG_NAME = "Shared CI";
const WRONG_EMAIL = "ci@example.invalid";
const ENV = { GIT_CONFIG_NOSYSTEM: "1" };
// .git, and the symlinks some tests add (readTree cannot read a link to a directory).
const TREE_IGNORE = [".git", "ext", "own", "link"];

function sh(ws: Workspace, dir: string, args: string[]): string {
  return execFileSync("git", args, {
    cwd: `${ws.root}/${dir}`,
    encoding: "utf8",
    env: { PATH: process.env.PATH ?? "", HOME: ws.home, GIT_CONFIG_NOSYSTEM: "1" },
  }).trim();
}

/** The fixture with `repo` as a git repository; the isolated global config holds the wrong identity. */
function gitFixture(): Workspace {
  const ws = fixture("26-commit-identity");
  writeFileSync(
    `${ws.home}/.gitconfig`,
    `[user]\n\tname = ${WRONG_NAME}\n\temail = ${WRONG_EMAIL}\n`,
  );
  sh(ws, "repo", ["init", "--quiet", "--initial-branch=main"]);
  return ws;
}

function gitConfig(ws: Workspace): string {
  return readFile(ws, "repo/.git/config");
}

function expectNoWrongValue(...results: { stdout: string; stderr: string }[]): void {
  for (const { stdout, stderr } of results) {
    for (const text of [stdout, stderr]) {
      expect(text).not.toContain(WRONG_EMAIL);
      expect(text).not.toContain(WRONG_NAME);
    }
  }
}

function setStack(ws: Workspace, sources: string[]): void {
  const entries = sources.map((source) => `  - source: ${source}\n`).join("");
  writeFileSync(`${ws.root}/repo/layer.yaml`, `stack:\n${entries}targets: [claude]\n`);
}

/** The build must fail with `message`, name nothing wrong, and write nothing. */
async function expectRejected(ws: Workspace, ...messages: string[]): Promise<void> {
  const config = gitConfig(ws);
  const tree = readTree(`${ws.root}/repo`, TREE_IGNORE);
  const result = await layers(ws, "repo", ["build"], ENV);
  expect(result.code).toBe(1);
  for (const message of messages) expect(result.stderr).toContain(message);
  expect(readTree(`${ws.root}/repo`, TREE_IGNORE)).toEqual(tree);
  expect(gitConfig(ws)).toBe(config);
  expectNoWrongValue(result);
}

describe("§13.18 commit identity", () => {
  describe("hardening", () => {
    /** A bare repo of the client layer, committed at `dir` inside `repo`, tagged v1. */
    function bareClientIn(ws: Workspace, dir: string): void {
      if (!existsSync(`${ws.root}/client-src`)) {
        cpSync(`${ws.root}/client`, `${ws.root}/client-src`, { recursive: true });
        sh(ws, "client-src", ["init", "--quiet", "--initial-branch=main"]);
        sh(ws, "client-src", ["add", "."]);
        sh(ws, "client-src", [
          "-c",
          "user.name=Test",
          "-c",
          "user.email=test@example.com",
          "commit",
          "--quiet",
          "-m",
          "client",
        ]);
        sh(ws, "client-src", ["tag", "v1"]);
      }
      mkdirSync(`${ws.root}/repo/${dir}/..`, { recursive: true });
      sh(ws, "repo", ["clone", "--bare", "--quiet", "../client-src", dir]);
    }

    it("rejects a file:/// URL whose relative spelling is a folder in the repo", async () => {
      const ws = gitFixture();
      bareClientIn(ws, "file:/idrepo");
      expect(isInsideRepo(`${ws.root}/repo`, "git:file:///idrepo@v1")).toBe(true);
      setStack(ws, ["path:../personal", "git:file:///idrepo@v1", "path:./.layer"]);
      // Whether git then clones the in-repo folder or refuses it, nothing is built or written.
      await expectRejected(ws);
    });

    it("rejects a local git: source whose text and disk disagree about '..'", async () => {
      const ws = gitFixture();
      mkdirSync(`${ws.root}/repo/d1/d2/d3`, { recursive: true });
      symlinkSync("d1/d2/d3", `${ws.root}/repo/link`);
      // As text, link/../../x.git is outside the repo and exists; on disk it is the repo's own d1/x.git.
      mkdirSync(`${ws.root}/x.git`);
      bareClientIn(ws, "d1/x.git");
      expect(isInsideRepo(`${ws.root}/repo`, "git:link/../../x.git@v1")).toBe(true);
      setStack(ws, ["path:../personal", "git:link/../../x.git@v1", "path:./.layer"]);
      await expectRejected(ws, "must not contain '..' segments");
    });

    it("keeps a guarded clone from falling back to a local path, even with GIT_ALLOW_PROTOCOL=file", () => {
      const ws = gitFixture();
      bareClientIn(ws, "x:y");
      const env = {
        PATH: process.env.PATH ?? "",
        HOME: ws.home,
        GIT_CONFIG_NOSYSTEM: "1",
        GIT_ALLOW_PROTOCOL: "file",
      };
      const cwd = `${ws.root}/repo`;
      // Control: with that variable set, the setting alone does not stop the local clone.
      runGit(
        ["-c", "protocol.file.allow=never", "clone", "--quiet", "--", "x:y", `${ws.root}/o1`],
        env,
        cwd,
      );
      expect(existsSync(`${ws.root}/o1/layer.yaml`)).toBe(true);
      // alz's guard drops the variable first.
      const guard = guardedClone(env);
      expect(guard.env.GIT_ALLOW_PROTOCOL).toBeUndefined();
      expect(() =>
        runGit([...guard.args, "clone", "--quiet", "--", "x:y", `${ws.root}/o2`], guard.env, cwd),
      ).toThrow();
      expect(existsSync(`${ws.root}/o2`)).toBe(false);
    });

    it("treats a git: source as remote only when no such local path exists", () => {
      const ws = fixture("26-commit-identity");
      const root = `${ws.root}/repo`;
      for (const source of [
        "git:https://example.invalid/client.git@v1",
        "git:ssh://example.invalid/client.git@v1",
        "git:host.invalid:client.git@v1",
        "github:owner/client@v1",
      ]) {
        expect(isInsideRepo(root, source)).toBe(false);
      }
      expect(isInsideRepo(root, "git:./client.git@v1")).toBe(true);
      expect(isInsideRepo(root, `git:file://${root}/client.git@v1`)).toBe(true);

      // The same names are inside as soon as the folder git would try first exists.
      mkdirSync(`${root}/host.invalid:client.git`);
      expect(isInsideRepo(root, "git:host.invalid:client.git@v1")).toBe(true);
      mkdirSync(`${root}/host.invalid:other.git`);
      expect(isInsideRepo(root, "git:host.invalid:other@v1")).toBe(true);
      writeFileSync(`${root}/host.invalid:bundled.bundle`, "");
      expect(isInsideRepo(root, "git:host.invalid:bundled@v1")).toBe(true);
      // ... and a remote that no folder shadows stays outside.
      expect(isInsideRepo(root, "git:ssh://example.invalid/client.git@v1")).toBe(false);
    });
  });
});
