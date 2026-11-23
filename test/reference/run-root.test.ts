import {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { capture } from "../exec";
import { tempDir } from "../run-root";
import { buildReference, checkedAlzBin } from "../setup/alz-bin";
import { removeTree } from "../setup/remove-tree";
import { ROOT_PREFIX, removeStaleRoots } from "../setup/stale-roots";

// The run root and the binary the acceptance suite spawns (TASK-55.2.7).

const DEAD = 999_999_999;
const uid = process.getuid?.() ?? -1;

/** A run root in `temp`, as the global setup leaves it, with `pid` in its pid file. */
function root(temp: string, name: string, pid: number | string | undefined): string {
  const path = join(temp, `${ROOT_PREFIX}${name}`);
  mkdirSync(join(path, "tmp"), { recursive: true });
  if (pid !== undefined) writeFileSync(join(path, "pid"), `${pid}\n`);
  return path;
}

describe("stale run roots are removed, and only those", () => {
  it("removes a root whose pid names a process that no longer exists", () => {
    const temp = tempDir("alz-stale-");
    const stale = root(temp, "dead", DEAD);
    expect(removeStaleRoots(temp, uid)).toEqual([`${ROOT_PREFIX}dead`]);
    expect(existsSync(stale)).toBe(false);
  });

  it("keeps a root of a running process", () => {
    const temp = tempDir("alz-stale-");
    const live = root(temp, "live", process.pid);
    expect(removeStaleRoots(temp, uid)).toEqual([]);
    expect(existsSync(live)).toBe(true);
  });

  it.each([
    ["no pid file", undefined],
    ["a pid file that holds no pid", "not a pid"],
    ["a pid file that holds zero", "0"],
  ])("keeps a root with %s", (_label, pid) => {
    const temp = tempDir("alz-stale-");
    const kept = root(temp, "odd", pid);
    expect(removeStaleRoots(temp, uid)).toEqual([]);
    expect(existsSync(kept)).toBe(true);
  });

  it("keeps a root another user owns", () => {
    const temp = tempDir("alz-stale-");
    const kept = root(temp, "theirs", DEAD);
    expect(removeStaleRoots(temp, uid + 1)).toEqual([]);
    expect(existsSync(kept)).toBe(true);
  });

  it("keeps a link, and does not follow it to a root elsewhere", () => {
    const temp = tempDir("alz-stale-");
    const elsewhere = root(tempDir("alz-elsewhere-"), "dead", DEAD);
    symlinkSync(elsewhere, join(temp, `${ROOT_PREFIX}link`));
    expect(removeStaleRoots(temp, uid)).toEqual([]);
    expect(existsSync(elsewhere)).toBe(true);
  });

  it("keeps every entry without the prefix, folders with a dead pid included", () => {
    const temp = tempDir("alz-stale-");
    const other = join(temp, "alz-other");
    mkdirSync(other);
    writeFileSync(join(other, "pid"), `${DEAD}\n`);
    writeFileSync(join(temp, `${ROOT_PREFIX}file`), `${DEAD}\n`);
    expect(removeStaleRoots(temp, uid)).toEqual([]);
    expect(existsSync(other)).toBe(true);
  });

  it("removes a stale root that holds a folder nobody can read", () => {
    const temp = tempDir("alz-stale-");
    const stale = root(temp, "locked", DEAD);
    const locked = join(stale, "tmp", "locked");
    mkdirSync(locked);
    writeFileSync(join(locked, "file"), "x");
    chmodSync(locked, 0o000);
    removeStaleRoots(temp, uid);
    expect(existsSync(stale)).toBe(false);
  });
});

describe("removing a tree", () => {
  it("removes a folder that has a mode 000 folder in it", () => {
    const dir = tempDir("alz-remove-");
    const locked = join(dir, "a", "locked");
    mkdirSync(locked, { recursive: true });
    writeFileSync(join(locked, "file"), "x");
    chmodSync(locked, 0o000);
    removeTree(dir);
    expect(existsSync(dir)).toBe(false);
  });
});

describe("ALZ_BIN", () => {
  const executable = (dir: string, mode = 0o755): string => {
    const path = join(dir, "alz");
    writeFileSync(path, "#!/bin/sh\nexit 0\n", { mode });
    chmodSync(path, mode);
    return path;
  };

  it("accepts an absolute path to an executable regular file", () => {
    const path = executable(tempDir("alz-bin-"));
    expect(checkedAlzBin(path)).toBe(path);
  });

  it("accepts a link whose target is such a file (checked after realpath)", () => {
    const dir = tempDir("alz-bin-");
    const link = join(dir, "link");
    symlinkSync(executable(dir), link);
    expect(checkedAlzBin(link)).toBe(link);
  });

  it.each([
    ["empty", () => "", "is empty"],
    ["relative", () => "target/release/alz", "is not an absolute path"],
    ["missing", () => join(tempDir("alz-bin-"), "none"), "does not exist"],
    ["a directory", () => tempDir("alz-bin-"), "is not a regular file"],
    ["not executable", () => executable(tempDir("alz-bin-"), 0o644), "is not executable"],
    [
      "a link to a file that is not executable",
      () => {
        const dir = tempDir("alz-bin-");
        const target = executable(dir, 0o644);
        const link = join(dir, "link");
        symlinkSync(target, link);
        return link;
      },
      "is not executable",
    ],
  ])("refuses a value that is %s, naming ALZ_BIN", (_label, value, why) => {
    expect(() => checkedAlzBin(value())).toThrow(new RegExp(`ALZ_BIN.*${why}`));
  });
});

describe("the TypeScript reference built for the run", () => {
  it("starts with an absolute node path and runs with PATH=/nonexistent", () => {
    const dir = tempDir("alz-bin-");
    const bin = buildReference(dir);
    expect(readFileSync(bin, "utf8").split("\n")[0]).toBe(`#!${process.execPath}`);
    expect(statSync(bin).mode & 0o777).toBe(0o755);
    const result = capture(bin, ["--version"], { cwd: dir, env: { PATH: "/nonexistent" } });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain("0.1.0");
  });
});
