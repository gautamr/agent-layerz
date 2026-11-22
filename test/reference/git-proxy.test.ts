import { execFileSync } from "node:child_process";
import { mkdtempSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { gitProxy, parseGitCalls, resolveOnPath } from "../git-proxy";

describe("the git stand-in of test/git-proxy.ts", () => {
  const run = (path: string, args: string[], cwd: string): string =>
    execFileSync("git", args, { cwd, env: { PATH: path }, encoding: "utf8" });

  it("records the working directory and every argument, then runs the real git", () => {
    const cwd = realpathSync(mkdtempSync(join(tmpdir(), "alz-proxy-cwd-")));
    const proxy = gitProxy();
    expect(proxy.calls()).toEqual([]);
    expect(run(proxy.env.PATH, ["--version"], cwd)).toMatch(/^git version /);
    run(proxy.env.PATH, ["-c", "a b=c", "--version", "", "x\ny"], cwd);
    expect(proxy.calls()).toEqual([
      { cwd, args: ["--version"] },
      { cwd, args: ["-c", "a b=c", "--version", "", "x\ny"] },
    ]);
  });

  it("finds the real git before the stand-in's directory is put first on PATH", () => {
    const proxy = gitProxy();
    const [dir] = proxy.env.PATH.split(":");
    expect(resolveOnPath("git", proxy.env.PATH)).toBe(`${dir}/git`);
    expect(resolveOnPath("git", process.env.PATH ?? "")).not.toBe(`${dir}/git`);
  });

  it("parses a log with an unfinished call and stray records without inventing calls", () => {
    expect(parseGitCalls("c:/x\0a:one\0e:\0c:/y\0a:two\0")).toEqual([{ cwd: "/x", args: ["one"] }]);
    expect(parseGitCalls("a:orphan\0e:\0")).toEqual([]);
  });
});
