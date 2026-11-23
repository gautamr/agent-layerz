import { createHash } from "node:crypto";
import { lstatSync, mkdirSync, readFileSync, symlinkSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { capture } from "../exec";
import { holdsVersion, packageVersion } from "../helpers";
import { tempDir } from "../run-root";

// TypeScript reference only: scripts/install-local.sh runs npm, bundles src/ with esbuild and
// writes dist/. It goes with src/ at the cutover; the Rust binary has its own install task.

const repoRoot = join(import.meta.dirname, "..", "..");
const script = join(repoRoot, "scripts", "install-local.sh");

/**
 * The only variables the script gets from this process: PATH, an isolated HOME and npm cache, no
 * user npm config, and the folders it installs into. Nothing else of the parent (tokens, the real
 * ~/.npmrc, proxies) reaches npm.
 */
const installEnv = (home: string, binDir: string): Record<string, string> => ({
  PATH: process.env.PATH ?? "",
  HOME: home,
  BIN_DIR: binDir,
  PREFIX: "",
  npm_config_cache: tempDir("alz-install-npm-cache-"),
  npm_config_userconfig: "/dev/null",
});

/** Runs the script with an isolated HOME, so npm never touches the real home folder. */
function install(binDir: string) {
  const home = tempDir("alz-install-home-");
  return capture("sh", [script], {
    cwd: repoRoot,
    env: installEnv(home, binDir),
    timeoutMs: 100_000,
  });
}

function sha256(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

describe("the environment of the install script", () => {
  it("holds exactly PATH, HOME, BIN_DIR, PREFIX and the two npm settings", () => {
    const env = installEnv("/h", "/b");
    expect(Object.keys(env).sort()).toEqual([
      "BIN_DIR",
      "HOME",
      "PATH",
      "PREFIX",
      "npm_config_cache",
      "npm_config_userconfig",
    ]);
    expect(env.HOME).toBe("/h");
    expect(env.npm_config_userconfig).toBe("/dev/null");
    expect(env.npm_config_cache).toMatch(/alz-suite-[^/]+\/tmp\//);
  });
});

describe("installing from source (scripts/install-local.sh)", () => {
  it("installs one regular executable that runs without node_modules", () => {
    const base = tempDir("alz-install-");
    const binDir = join(base, "bin");

    const result = install(binDir);

    expect(result.status, result.stderr).toBe(0);
    const target = join(binDir, "alz");
    const stat = lstatSync(target);
    expect(stat.isFile()).toBe(true);
    expect(stat.isSymbolicLink()).toBe(false);
    expect(stat.mode & 0o777).toBe(0o755);
    expect(result.stdout).toContain(`sha256   ${sha256(target)}`);
    expect(sha256(target)).toBe(sha256(join(repoRoot, "dist", "alz-standalone.mjs")));

    // An empty directory far from the checkout: nothing to resolve yaml or diff from. The binary
    // gets a HOME of its own.
    const empty = join(base, "empty");
    mkdirSync(empty);
    const env = { HOME: tempDir("alz-install-run-home-") };
    const version = capture(target, ["--version"], { cwd: empty, env });
    expect(version.status).toBe(0);
    expect(version.stderr).toBe("");
    expect(holdsVersion(version.stdout, packageVersion())).toBe(true);
    const help = capture(target, ["--help"], { cwd: empty, env });
    expect(help.status).toBe(0);
    expect(help.stdout).not.toBe("");
  }, 120_000);

  it("refuses to replace a symlink at the target", () => {
    const base = tempDir("alz-install-");
    const binDir = join(base, "bin");
    mkdirSync(binDir);
    symlinkSync(join(base, "elsewhere"), join(binDir, "alz"));

    const result = install(binDir);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain("is a symlink");
    expect(lstatSync(join(binDir, "alz")).isSymbolicLink()).toBe(true);
  }, 120_000);
});
