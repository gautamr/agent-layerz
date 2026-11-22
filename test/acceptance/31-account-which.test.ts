import { execFileSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  accountFolder,
  alz,
  built,
  configDir,
  hubEntry,
  PERSONAL,
  plain,
  REPO_LAYER,
  remoteStack,
  type Setup,
  sh,
  stateDir,
  useStack,
  writeAccounts,
  writeAllowed,
} from "../account-workspace";
import { gitProxy } from "../git-proxy";
import { type CliResult, readTree, snapshot } from "../helpers";
import { YAML_ALIAS_LIMIT } from "../spec/limits";

/**
 * Account resolution (spec 13.19), seen from outside: the binary is driven and every claim is
 * made through language-neutral proxies. What the TypeScript resolver returns, the calls it makes,
 * the files it opens and the modules it imports are checked in test/reference.
 *
 * Every `alz account which` case goes through `which`, which checks two proxies as well as the
 * case's own assertions on the reported output:
 *
 *  (b) no writes: HOME (the state folder included) and the workspace (cache, repository, hub) are
 *      equal before and after, in kind, mode, content and link target;
 *  (c) no git: a `git` stand-in first on PATH records its calls, and none is made.
 *
 * The usage cases and the control-character cases keep their assertions; R1.7 rewrites the usage
 * ones in the decision-26 form.
 */

/** Everything alz may have written to: the fake HOME and the workspace. */
const world = (s: Setup) => ({ home: snapshot(s.ws.home), root: snapshot(s.ws.root) });

/**
 * Runs `alz account which` the way a shell hook does, and checks that it wrote nothing and ran
 * no git.
 */
async function which(
  s: Setup,
  extra: Record<string, string> = {},
  args: string[] = [],
  dir = "repo",
): Promise<CliResult> {
  const proxy = gitProxy();
  const before = world(s);
  const result = await alz(s, ["account", "which", ...args], { ...proxy.env, ...extra }, dir);
  expect(world(s), "alz account which wrote").toEqual(before);
  expect(proxy.calls(), "alz account which ran git").toEqual([]);
  return result;
}

/** The lines of a stream that hold a warning. */
function warningLines(text: string): string[] {
  return text.split("\n").filter((line) => line.startsWith("warning ["));
}

const CLIENT_A = ".claude-client-a";
const CLIENT_A_API = ".codex-client-a-api";

describe("§13.19 sources: which account each target gets and why (AC 1)", () => {
  it("gives the default account with no files, never failing on a missing ~/.claude or ~/.codex", async () => {
    const s = plain();
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stderr).toBe("");
    expect(result.stdout).toContain(
      "accounts.yaml: ~/.config/agent-layerz/accounts.yaml (not found)",
    );
    expect(result.stdout).toContain(
      "allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (not found)",
    );
    expect(result.stdout).toContain("claude: default (source: default) CLAUDE_CONFIG_DIR unset");
    expect(result.stdout).toContain("codex: default (source: default) CODEX_HOME unset");
    expect(result.stdout).toContain("  use step skipped (no alz use choice)");
    expect(result.stdout).toContain("  user config step skipped (no accounts.yaml)");
    expect(result.stdout).toContain("  layer step skipped (no allowed.yaml)");
    expect(existsSync(`${s.ws.home}/.claude`)).toBe(false);
  });

  it("takes a folders entry from the user config, for every target or per target", async () => {
    const s = plain();
    accountFolder(s, CLIENT_A, ".claude-client-b");
    writeAccounts(
      s,
      `folders:\n  ${s.real}/repo: client-a\n  ${s.real}/other:\n    claude: client-b\n    codex: default\n`,
    );
    accountFolder(s, ".codex-client-a");
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      `claude: client-a (source: user config) CLAUDE_CONFIG_DIR=~/${CLIENT_A}`,
    );
    expect(result.stdout).toContain(
      "codex: client-a (source: user config) CODEX_HOME=~/.codex-client-a",
    );

    writeAccounts(s, `folders:\n  ${s.real}/repo:\n    claude: client-b\n    codex: default\n`);
    const per = await which(s);
    expect(per.stdout).toContain(
      "claude: client-b (source: user config) CLAUDE_CONFIG_DIR=~/.claude-client-b",
    );
    expect(per.stdout).toContain("codex: default (source: user config) CODEX_HOME unset");
  });

  it("takes the layer's account once allowed.yaml holds a matching entry", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      "allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (read; this repository is allowed)",
    );
    expect(result.stdout).toContain(
      `claude: client-a (source: layer) CLAUDE_CONFIG_DIR=~/${CLIENT_A}`,
    );
    expect(result.stdout).toContain(
      `codex: client-a-api (source: layer) CODEX_HOME=~/${CLIENT_A_API}`,
    );
    expect(result.stderr).toBe("");
  });

  it("takes the use choice over the user config and the layer, and `default` unsets the variable", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API, ".claude-use-a", ".claude-client-b");
    writeAllowed(s, [hubEntry(s)]);
    writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n`);
    const result = await which(s, {}, ["--use", "claude=use-a", "--use", "codex=default"]);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      "claude: use-a (source: use) CLAUDE_CONFIG_DIR=~/.claude-use-a",
    );
    expect(result.stdout).toContain("codex: default (source: use) CODEX_HOME unset");
  });

  it("keeps a value set outside alz with one warning, and leaves one that matches its marker alone", async () => {
    const s = plain();
    const foreign = await which(s, { CLAUDE_CONFIG_DIR: "/somewhere/else" });
    expect(foreign.code).toBe(0);
    expect(foreign.stdout).toContain(
      'claude: set outside alz (source: environment) CLAUDE_CONFIG_DIR="/somewhere/else" kept',
    );
    expect(foreign.stdout).toContain("codex: default (source: default)");
    expect(warningLines(foreign.stderr)).toHaveLength(1);
    expect(foreign.stderr).toContain("CLAUDE_CONFIG_DIR was set outside alz");
    expect(foreign.stderr).not.toContain("mise or direnv");

    // The hook's marker holds the value it set, so that value is not foreign.
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n`);
    const own = `${s.ws.home}/.claude-client-b`;
    const marked = await which(s, { CLAUDE_CONFIG_DIR: own, ALZ_SET_CLAUDE_CONFIG_DIR: own });
    expect(marked.stdout).toContain("claude: client-b (source: user config)");
    expect(marked.stderr).toBe("");

    // A stale marker, with the variable unset, is ignored.
    const stale = await which(s, { ALZ_SET_CLAUDE_CONFIG_DIR: "/old" });
    expect(stale.stdout).toContain("claude: client-b (source: user config)");

    // An explicit choice replaces a foreign value.
    accountFolder(s, ".claude-use-a");
    const replaced = await which(s, { CLAUDE_CONFIG_DIR: "/somewhere/else" }, [
      "--use",
      "claude=use-a",
    ]);
    expect(replaced.stdout).toContain("claude: use-a (source: use)");
    expect(replaced.stderr).toBe("");
  });

  it("treats a set but empty account variable as foreign and an unset one as not", async () => {
    const s = plain();
    const empty = await which(s, { CODEX_HOME: "" });
    expect(empty.stdout).toContain(
      'codex: set outside alz (source: environment) CODEX_HOME="" kept',
    );
    expect(warningLines(empty.stderr)).toHaveLength(1);
  });

  it("calls the default account by default_name and warns when that folder exists", async () => {
    const s = plain();
    writeAccounts(s, "default_name: personal\n");
    const result = await which(s);
    expect(result.stdout).toContain(
      "claude: default, called personal here (source: default) CLAUDE_CONFIG_DIR unset",
    );
    expect(result.stderr).toBe("");

    accountFolder(s, ".claude-personal");
    const warned = await which(s);
    expect(warningLines(warned.stderr)).toHaveLength(1);
    expect(warned.stderr).toContain("default_name 'personal'");
  });

  it("applies the default account for a layer or a use choice that names default_name", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A_API);
    writeAccounts(s, "default_name: client-a\n");
    writeAllowed(s, [hubEntry(s)]);
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      "claude: default, called client-a here (source: layer) CLAUDE_CONFIG_DIR unset",
    );
  });

  it("applies the default account for a use choice that names default_name", async () => {
    const s = plain();
    writeAccounts(s, "default_name: personal\n");
    const result = await which(s, {}, ["--use", "claude=personal"]);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      "claude: default, called personal here (source: use) CLAUDE_CONFIG_DIR unset",
    );
  });

  it("ignores a forged ALZ_USE_CLAUDE in the environment", async () => {
    const s = plain();
    accountFolder(s, ".claude-forged");
    const result = await which(s, { ALZ_USE_CLAUDE: "forged", ALZ_USE_CODEX: "forged" });
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).toContain("codex: default (source: default)");
  });

  it.each([
    [["--use", "claude"]],
    [["--use", "gemini=a"]],
    [["--use", "claude=Bad Name"]],
    [["--use", "claude=../x"]],
    [["--use", "claude=a", "--use", "claude=b"]],
    [["extra"]],
    [["--unknown"]],
  ])("exits 2 with the usage text for %j", async (args) => {
    const s = plain();
    const result = await which(s, {}, args);
    expect(result.code).toBe(2);
    expect(result.stdout).toBe("");
    expect(result.stderr).toContain("usage: alz <command>");
  });

  it("exits 2 for an unknown account subcommand", async () => {
    const result = await alz(plain(), ["account", "frobnicate"]);
    expect(result.code).toBe(2);
    expect(result.stderr).toContain("usage: alz <command>");
  });
});

const BLOCKED = "~/.claude-alz-blocked";

describe("§13.19 paths: the repository root and the folders entries (AC 4)", () => {
  function workTree(s: Setup, ...dirs: string[]): void {
    for (const dir of dirs) mkdirSync(`${s.ws.root}/${dir}`, { recursive: true });
  }

  it("matches whole path segments and lets the longest folders entry win", async () => {
    const s = plain();
    workTree(s, "work/client-a/x", "work/client-ab/x");
    accountFolder(s, ".claude-a", ".claude-w");
    writeAccounts(s, `folders:\n  ${s.real}/work: w\n  ${s.real}/work/client-a: a\n`);
    const inA = await which(s, {}, [], "work/client-a/x");
    expect(inA.stdout).toContain("claude: a (source: user config)");
    const inAb = await which(s, {}, [], "work/client-ab/x");
    expect(inAb.stdout).toContain("claude: w (source: user config)");

    writeAccounts(s, `folders:\n  ${s.real}/work/client-a: a\n`);
    const apart = await which(s, {}, [], "work/client-ab/x");
    expect(apart.stdout).toContain("claude: default (source: default)");
    expect(apart.stdout).toContain(
      "user config step skipped (no folders entry for this directory)",
    );
  });

  it("says when the longest entry names no account for the target and goes no further up", async () => {
    const s = plain();
    workTree(s, "work/client-a/x");
    accountFolder(s, ".claude-w");
    writeAccounts(s, `folders:\n  ${s.real}/work: w\n  ${s.real}/work/client-a:\n    codex: x\n`);
    const result = await which(s, {}, [], "work/client-a/x");
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).toContain(
      "user config step skipped (the folders entry names no claude account)",
    );
  });

  it("resolves a symlinked working directory and a symlinked folders key", async () => {
    const s = plain();
    workTree(s, "work/client-a/x");
    symlinkSync(`${s.real}/work/client-a`, `${s.real}/link-a`);
    accountFolder(s, ".claude-a");
    writeAccounts(s, `folders:\n  ${s.real}/work/client-a: a\n`);
    const throughLink = await which(s, {}, [], "link-a/x");
    expect(throughLink.stdout).toContain("claude: a (source: user config)");

    writeAccounts(s, `folders:\n  ${s.real}/link-a: a\n`);
    const linkKey = await which(s, {}, [], "work/client-a/x");
    expect(linkKey.stdout).toContain("claude: a (source: user config)");
  });

  const folded = existsSync(process.cwd().toUpperCase());
  it.skipIf(!folded)("resolves a case variant on a case-insensitive file system", async () => {
    const s = plain();
    workTree(s, "work/client-a/x");
    accountFolder(s, ".claude-a");
    writeAccounts(s, `folders:\n  ${s.real}/WORK/Client-A: a\n`);
    const result = await which(s, {}, [], "work/client-a/x");
    expect(result.stdout).toContain("claude: a (source: user config)");
  });

  it("matches nothing for a folders key that does not resolve", async () => {
    const s = plain();
    workTree(s, "work/client-a/x");
    accountFolder(s, ".claude-a");
    writeAccounts(s, `folders:\n  ${s.real}/work/gone: a\n`);
    const result = await which(s, {}, [], "work/client-a/x");
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stderr).toBe("");
  });

  it("finds the nearest layer.yaml with a stack key and skips a layer's own manifest", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    const inLayer = await which(s, {}, [], "repo/.layer/instructions");
    expect(inLayer.stdout).toContain(`repository: ${s.real}/repo\n`);
    expect(inLayer.stdout).toContain("claude: client-a (source: layer)");
    const deep = await which(s, {}, [], "repo/sub/deeper");
    expect(deep.stdout).toContain(`repository: ${s.real}/repo\n`);
    expect(deep.stdout).toContain("claude: client-a (source: layer)");
  });

  it("treats a nested manifest as its own root, so the outer entry does not apply there", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    const nested = await which(s, {}, [], "repo/nested");
    expect(nested.code, nested.stderr).toBe(0);
    expect(nested.stdout).toContain(`repository: ${s.real}/repo/nested\n`);
    expect(nested.stdout).toContain("claude: default (source: default)");
    expect(nested.stdout).toContain("layer step skipped (this repository is not allowed)");
    expect(nested.stdout).not.toContain("alz account allow");
  });

  it("matches an allowed entry only by the exact real root, and ignores one recorded under another spelling", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    symlinkSync(`${s.real}/repo`, `${s.real}/repo-link`);
    writeAllowed(s, [hubEntry(s, { root: `${s.real}/repo-link` })]);
    const linked = await which(s);
    expect(linked.stdout).toContain("claude: default (source: default)");
    expect(warningLines(linked.stderr)).toHaveLength(1);
    expect(linked.stderr).toContain("entry 1 is ignored (its root is not a real path");

    writeAllowed(s, [hubEntry(s, { root: `${s.real}/repo/sub` })]);
    const parent = await which(s);
    expect(parent.stdout).toContain("claude: default (source: default)");

    writeAllowed(s, [hubEntry(s, { root: `${s.real}/gone` })]);
    const gone = await which(s);
    expect(gone.code).toBe(0);
    expect(gone.stdout).toContain("claude: default (source: default)");
    expect(gone.stderr).toBe("");
  });

  describe("a layer.yaml on the way that cannot be read stops the walk (decision-19)", () => {
    // The oversize payload is a valid stack with padding, so it is told apart from a schema
    // failure by its phrase. The alias payload is not a valid stack (it has an unknown `refs`
    // field), which is the point: it is "cannot be read" only because the alias limit is
    // enforced at parse time, before the schema is looked at. Without the limit it would say
    // "is not valid".
    const aliasList = Array.from({ length: 150 }, () => "*a").join(", ");
    type Problem = "cannot be read" | "is not valid";
    const broken: [string, (path: string, s: Setup) => void, Problem][] = [
      ["invalid YAML", (path) => writeFileSync(path, "stack: [\n"), "cannot be read"],
      [
        "a valid stack over 1 MiB",
        (path) => writeFileSync(path, `stack:\n  - source: path:x\n#${"x".repeat(1024 * 1024)}\n`),
        "cannot be read",
      ],
      [
        "a valid stack with more aliases than the limit",
        (path) =>
          writeFileSync(path, `a: &a [x]\nstack:\n  - source: path:x\nrefs: [${aliasList}]\n`),
        "cannot be read",
      ],
      [
        "a symbolic link",
        (path, s) => symlinkSync(`${s.real}/repo/layer.yaml`, path),
        "cannot be read",
      ],
      ["a directory", (path) => mkdirSync(path), "cannot be read"],
      [
        "a stack that is not valid",
        (path) => writeFileSync(path, "stack: [{source: 1}]\n"),
        "is not valid",
      ],
    ];
    if (process.getuid?.() !== 0) {
      broken.push([
        "a file nobody can read",
        (path) => {
          writeFileSync(path, "stack:\n  - source: path:x\n");
          chmodSync(path, 0o000);
        },
        "cannot be read",
      ]);
    }
    const warningPhrase = {
      "cannot be read": "it cannot be read, parsed or is too large",
      "is not valid": "its stack is not valid",
    };

    it.each(broken)(
      "fails closed for an allowed root with %s, applying the blocked folder",
      async (_label, make, problem) => {
        const s = await built();
        accountFolder(s, CLIENT_A, CLIENT_A_API);
        make(`${s.real}/repo/sub/layer.yaml`, s);
        writeAllowed(s, [
          hubEntry(s, { root: `${s.real}/repo/sub`, account: { claude: "client-a" } }),
        ]);
        const result = await which(s, {}, [], "repo/sub/deeper");
        expect(result.code).toBe(1);
        expect(result.stdout).toContain(`repository: ${s.real}/repo/sub (layer.yaml ${problem})\n`);
        expect(result.stdout).toContain("claude: fails closed (source: layer)");
        expect(result.stdout).toContain(`layer step skipped (layer.yaml ${problem}); fails closed`);
        expect(result.stdout).toContain(
          `the hook applies CLAUDE_CONFIG_DIR=${BLOCKED} (the blocked folder)`,
        );
        expect(result.stdout).toContain(`fix: repair ${s.real}/repo/sub/layer.yaml`);
        expect(result.stdout).toContain("codex: default (source: default)");
        expect(warningLines(result.stderr)).toHaveLength(1);
        expect(result.stderr).toContain(warningPhrase[problem]);
        expect(existsSync(`${s.ws.home}/.claude-alz-blocked`)).toBe(false);
      },
    );

    it.each(broken)(
      "gives the default account for a root that is not allowed, with %s, never the outer repository's",
      async (_label, make, problem) => {
        const s = await built();
        accountFolder(s, CLIENT_A, CLIENT_A_API);
        make(`${s.real}/repo/sub/layer.yaml`, s);
        writeAllowed(s, [hubEntry(s)]);
        const result = await which(s, {}, [], "repo/sub/deeper");
        expect(result.code).toBe(0);
        expect(result.stdout).toContain(`repository: ${s.real}/repo/sub (layer.yaml ${problem})\n`);
        expect(result.stdout).toContain("claude: default (source: default)");
        expect(result.stdout).toContain("layer step skipped (this repository is not allowed)");
        expect(warningLines(result.stderr)).toHaveLength(1);
        expect(result.stderr).toContain(warningPhrase[problem]);
      },
    );

    it("proves the alias payload parses within the limit and is refused only past it (through alz account which)", async () => {
      const within = Array.from({ length: 50 }, () => "*a").join(", ");
      expect(50).toBeLessThanOrEqual(YAML_ALIAS_LIMIT);
      expect(150).toBeGreaterThan(YAML_ALIAS_LIMIT);
      const s = await built();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      writeAllowed(s, [hubEntry(s)]);
      const file = `${s.real}/repo/sub/layer.yaml`;
      // Within the limit the text parses and fails the schema (it has an unknown `refs` field)...
      writeFileSync(file, `a: &a [x]\nstack:\n  - source: path:x\nrefs: [${within}]\n`);
      const parsed = await which(s, {}, [], "repo/sub/deeper");
      expect(parsed.stdout).toContain(`repository: ${s.real}/repo/sub (layer.yaml is not valid)\n`);
      // ...and past it the same shape cannot be read at all.
      writeFileSync(file, `a: &a [x]\nstack:\n  - source: path:x\nrefs: [${aliasList}]\n`);
      const refused = await which(s, {}, [], "repo/sub/deeper");
      expect(refused.stdout).toContain(
        `repository: ${s.real}/repo/sub (layer.yaml cannot be read)\n`,
      );
    });

    it("does not hang on a FIFO named layer.yaml on the walk, and treats it as unreadable", async () => {
      const s = await built();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      execFileSync("mkfifo", [`${s.real}/repo/sub/layer.yaml`]);
      writeAllowed(s, [
        hubEntry(s, { root: `${s.real}/repo/sub`, account: { claude: "client-a" } }),
      ]);
      const result = await which(s, {}, [], "repo/sub/deeper");
      expect(result.code).toBe(1);
      expect(result.stdout).toContain("(layer.yaml cannot be read)");
      expect(result.stdout).toContain(
        "layer step skipped (layer.yaml cannot be read); fails closed",
      );
    });
  });

  it("never opens a layer.yaml at $HOME or above it, or in an account folder", async () => {
    const s = plain();
    writeFileSync(`${s.ws.home}/layer.yaml`, "stack:\n  - source: path:x\n");
    mkdirSync(`${s.ws.home}/work`, { recursive: true });
    const atHome = await which(s, {}, [], "../home/work");
    expect(atHome.stdout).toContain("repository: none (no repository here)");
    const inFolder = join(s.ws.home, ".claude-client-a", "project");
    mkdirSync(inFolder, { recursive: true });
    writeFileSync(`${s.ws.home}/.claude-client-a/layer.yaml`, "stack:\n  - source: path:x\n");
    const result = await which(s, {}, [], `../home/.claude-client-a/project`);
    expect(result.stdout).toContain(
      "repository: none (the working directory is inside an account folder)",
    );
    expect(result.stdout).toContain(
      "layer step skipped (the working directory is inside an account folder)",
    );
  });
});

const MARKER = "SECRET-MARKER-5f2c1d";

/** A file in a folder of its own, written the way a careful user would: 0600 in a 0700 folder. */
function writeAt(folder: string, file: string, text: string): string {
  mkdirSync(folder, { recursive: true });
  chmodSync(folder, 0o700);
  writeFileSync(`${folder}/${file}`, text);
  chmodSync(`${folder}/${file}`, 0o600);
  return `${folder}/${file}`;
}

describe("§13.19 the two files: where alz reads them from (AC 2)", () => {
  const CONFIG = (s: Setup) => `folders:\n  ${s.real}/repo: client-b\n`;
  const applied = "claude: client-b (source: user config)";

  it("reads accounts.yaml in the safe case, so the cases below prove what ignores it", async () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, CONFIG(s));
    const result = await which(s);
    expect(result.stdout).toContain(applied);
    expect(result.stdout).toContain("accounts.yaml: ~/.config/agent-layerz/accounts.yaml (read)");
    expect(result.stderr).toBe("");
  });

  it("ignores a user config inside the current repository", async () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAt(`${s.real}/repo/cfg/agent-layerz`, "accounts.yaml", CONFIG(s));
    const result = await which(s, { XDG_CONFIG_HOME: `${s.real}/repo/cfg` });
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("(ignored: it lies inside the current repository)");
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).toContain("user config step skipped (accounts.yaml is ignored)");
    expect(warningLines(result.stderr)).toHaveLength(1);
  });

  it.each([
    [
      "a file that group can write",
      (s: Setup) => chmodSync(`${configDir(s)}/accounts.yaml`, 0o660),
    ],
    [
      "a file that others can write",
      (s: Setup) => chmodSync(`${configDir(s)}/accounts.yaml`, 0o602),
    ],
    ["a folder that group can write", (s: Setup) => chmodSync(configDir(s), 0o770)],
    [
      "a parent folder below $HOME that others can write",
      (s: Setup) => chmodSync(`${s.ws.home}/.config`, 0o757),
    ],
  ])("ignores a user config in %s", async (_label, weaken) => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, CONFIG(s));
    weaken(s);
    const result = await which(s);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("(ignored: ");
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(warningLines(result.stderr)).toHaveLength(1);
    expect(result.stderr).toContain(
      "accounts.yaml (~/.config/agent-layerz/accounts.yaml) is ignored",
    );
  });

  it("ignores allowed.yaml for the same reasons", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    chmodSync(`${stateDir(s)}/allowed.yaml`, 0o666);
    const result = await which(s);
    expect(result.stdout).toContain(
      "allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (ignored: group or others can write it)",
    );
    expect(result.stdout).toContain("layer step skipped (allowed.yaml is ignored)");
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).not.toContain("alz account allow");
  });

  it("ignores a file whose path names an account folder, before any file system call on it (through alz account which)", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    const result = await which(s, {
      XDG_CONFIG_HOME: `${s.ws.home}/.claude-x`,
      XDG_STATE_HOME: `${s.ws.home}/.claude-y`,
    });
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      "accounts.yaml: ~/.claude-x/agent-layerz/accounts.yaml (ignored: it lies inside an account folder)",
    );
    expect(result.stdout).toContain(
      "allowed.yaml: ~/.claude-y/agent-layerz/allowed.yaml (ignored: it lies inside an account folder)",
    );
    expect(result.stdout).toContain("claude: default (source: default)");
  });

  describe("a FIFO where a file should be: alz does not wait for a writer and reports the file as unreadable", () => {
    // `alz account which` runs in the shell hook, so a pipe swapped in for either file must not
    // hang every prompt. The check is that the command ends and says what it could not read; the
    // case runs under the suite's time limit, which R1.7 makes a real one by spawning the binary.
    const unreadable = "(ignored: it is not a regular file of at most 1 MiB that can be read)";

    it("does not hang on a FIFO at the accounts.yaml path, and ignores it", async () => {
      const s = plain();
      accountFolder(s, ".claude-client-b");
      const file = writeAccounts(s, CONFIG(s));
      rmSync(file);
      execFileSync("mkfifo", [file]);
      chmodSync(file, 0o600);
      const started = Date.now();
      const result = await which(s);
      expect(Date.now() - started).toBeLessThan(10_000);
      expect(result.code, result.stderr).toBe(0);
      expect(result.stdout).toContain(
        `accounts.yaml: ~/.config/agent-layerz/accounts.yaml ${unreadable}`,
      );
      expect(result.stdout).toContain("claude: default (source: default)");
      expect(result.stdout).toContain("user config step skipped (accounts.yaml is ignored)");
    });

    it("does not hang on a FIFO at the allowed.yaml path, and ignores it", async () => {
      const s = await built();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      const file = writeAllowed(s, [hubEntry(s)]);
      rmSync(file);
      execFileSync("mkfifo", [file]);
      chmodSync(file, 0o600);
      const started = Date.now();
      const result = await which(s);
      expect(Date.now() - started).toBeLessThan(10_000);
      expect(result.code, result.stderr).toBe(0);
      expect(result.stdout).toContain(
        `allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml ${unreadable}`,
      );
      expect(result.stdout).toContain("claude: default (source: default)");
      expect(result.stdout).toContain("layer step skipped (allowed.yaml is ignored)");
    });
  });

  it("treats a relative XDG_CONFIG_HOME as unset, with a warning, and an empty one as unset without", async () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, CONFIG(s));
    const relative = await which(s, { XDG_CONFIG_HOME: "relative/dir" });
    expect(relative.stdout).toContain(applied);
    expect(warningLines(relative.stderr)).toHaveLength(1);
    expect(relative.stderr).toContain("XDG_CONFIG_HOME is not an absolute path");
    const empty = await which(s, { XDG_CONFIG_HOME: "" });
    expect(empty.stdout).toContain(applied);
    expect(empty.stderr).toBe("");
  });

  describe.each(["__MISE_DIFF", "DIRENV_DIFF"])("with %s present", (manager) => {
    it("ignores a user config an XDG_CONFIG_HOME away from ~/.config locates, naming the variable", async () => {
      const s = plain();
      accountFolder(s, ".claude-client-b");
      writeAt(`${s.real}/alt/agent-layerz`, "accounts.yaml", CONFIG(s));
      const moved = { XDG_CONFIG_HOME: `${s.real}/alt` };
      const before = await which(s, moved);
      expect(before.stdout).toContain(applied);

      const result = await which(s, { ...moved, [manager]: MARKER });
      expect(result.code).toBe(0);
      expect(result.stdout).toContain("claude: default (source: default)");
      expect(warningLines(result.stderr)).toHaveLength(1);
      expect(result.stderr).toContain(
        "XDG_CONFIG_HOME points away from its default and mise or direnv is active here",
      );
    });

    it("ignores an allowed.yaml an XDG_STATE_HOME away from ~/.local/state locates", async () => {
      const s = await built();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      writeAt(
        `${s.real}/alt-state/agent-layerz`,
        "allowed.yaml",
        `- root: ${s.real}/repo\n  account:\n    claude: client-a\n    codex: client-a-api\n  source: ${s.hubLayer}\n`,
      );
      const moved = { XDG_STATE_HOME: `${s.real}/alt-state` };
      expect((await which(s, moved)).stdout).toContain("claude: client-a (source: layer)");
      const result = await which(s, { ...moved, [manager]: MARKER });
      expect(result.stdout).toContain("claude: default (source: default)");
      expect(result.stderr).toContain(
        "XDG_STATE_HOME points away from its default and mise or direnv is active here",
      );
    });

    it("still reads a user config at the default location", async () => {
      const s = plain();
      accountFolder(s, ".claude-client-b");
      writeAccounts(s, CONFIG(s));
      const result = await which(s, { [manager]: MARKER });
      expect(result.stdout).toContain(applied);
      expect(result.stderr).toBe("");
    });

    it("never puts the value of the variable in any output", async () => {
      const s = plain();
      const result = await which(s, {
        [manager]: MARKER,
        CLAUDE_CONFIG_DIR: "/somewhere/else",
        XDG_CONFIG_HOME: `${s.real}/alt`,
      });
      expect(result.stderr).toContain("mise or direnv is active here");
      expect(result.stdout + result.stderr).not.toContain(MARKER);
    });
  });

  it("ignores an allowed.yaml entry with a relative root, an invalid account name or a duplicate root", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    const good = hubEntry(s);
    const cases: [string, unknown[], RegExp][] = [
      [
        "a relative root",
        [{ ...good, root: "repo" }],
        /entry 1 is ignored \(its root is not an absolute path\)/,
      ],
      [
        "an invalid name",
        [{ ...good, account: { claude: "Bad_Name", codex: "x" } }],
        /entry 1 is ignored \(its account is not a mapping/,
      ],
      [
        "an unknown key",
        [{ ...good, extra: 1 }],
        /entry 1 is ignored \(it needs exactly root, account and source\)/,
      ],
      ["a source that is not text", [{ ...good, source: 5 }], /entry 1 is ignored \(its source/],
      [
        "a duplicate root",
        [good, { ...good, account: { claude: "other" } }],
        /entries 1, 2 are ignored \(they have the same root\)/,
      ],
    ];
    for (const [label, entries, warning] of cases) {
      writeAllowed(s, entries as never);
      const result = await which(s);
      expect(result.code, label).toBe(0);
      expect(result.stdout, label).toContain("claude: default (source: default)");
      expect(result.stdout, label).toContain("layer step skipped (this repository is not allowed)");
      expect(result.stderr, label).toMatch(warning);
      expect(result.stderr, label).not.toContain("Bad_Name");
    }
    writeAllowed(s, [], "root: not-a-list\n");
    const shape = await which(s);
    expect(shape.stderr).toContain("allowed.yaml is ignored: its top level must be a list");
  });

  it("warns about a file inside a git work tree and still reads it", async () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(s, CONFIG(s));
    mkdirSync(`${s.ws.home}/.config/.git`);
    const result = await which(s);
    expect(result.stdout).toContain(applied);
    expect(warningLines(result.stderr)).toHaveLength(1);
    expect(result.stderr).toContain("lies inside a git work tree");
  });

  it("ignores allowed.yaml inside a git work tree, which a checkout could have planted", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    mkdirSync(`${s.ws.home}/.local/state/.git`);
    const result = await which(s);
    expect(result.stdout).toContain("allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml");
    expect(result.stdout).toContain(
      "(ignored: it lies inside a git work tree (~/.local/state/.git))",
    );
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(warningLines(result.stderr)).toHaveLength(1);
  });

  it("lists an approval for a root that does not exist here, quoted and with ~ for $HOME", async () => {
    const s = plain();
    writeAllowed(s, [
      hubEntry(s, { root: `${s.ws.home}/gone` }),
      hubEntry(s, { root: `${s.ws.home}/we\u001b[31mird` }),
    ]);
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain('  stale approval: "~/gone" does not exist here');
    expect(result.stdout).toContain('  stale approval: "~/we[31mird" does not exist here');
    expect(result.stdout).not.toContain("\u001b");
  });

  it("ignores accounts.yaml entries that are not valid, one warning each, without echoing account names", async () => {
    const s = plain();
    accountFolder(s, ".claude-client-b");
    writeAccounts(
      s,
      `unknown: 1\nfolders:\n  relative/path: client-b\n  ${s.real}/repo: Bad_Name\n  ~/.claude-x: client-b\n  ${s.real}/ok: client-b\n`,
    );
    const result = await which(s);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(warningLines(result.stderr)).toHaveLength(4);
    expect(result.stderr).not.toContain("Bad_Name");
  });

  it("builds byte-identical output and lock with and without accounts.yaml, allowed.yaml and account variables", async () => {
    const s = await built();
    const read = () => ({
      repo: readTree(`${s.ws.root}/repo`),
      lock: readFileSync(`${s.ws.root}/repo/layer.lock`, "utf8"),
    });
    const before = read();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAccounts(s, CONFIG(s));
    writeAllowed(s, [hubEntry(s)]);
    const result = await alz(s, ["build"], {
      CLAUDE_CONFIG_DIR: `${s.ws.home}/${CLIENT_A}`,
      CODEX_HOME: `${s.ws.home}/${CLIENT_A_API}`,
      __MISE_DIFF: MARKER,
      DIRENV_DIFF: MARKER,
    });
    expect(result.code, result.stderr).toBe(0);
    expect(read()).toEqual(before);
    expect((await alz(s, ["check"])).code).toBe(0);
  });
});

/** The first of the hub's commits that declares what the test needs, tagged `v3`. */
function hubV3(s: Setup, layerYaml: string): void {
  writeFileSync(`${s.hub}/layer/layer.yaml`, layerYaml);
  sh(s.ws, s.hub, ["add", "."]);
  sh(s.ws, s.hub, ["commit", "--quiet", "-m", "hub v3"]);
  sh(s.ws, s.hub, ["tag", "v3"]);
}

const HUB_HEAD = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\n";

function removeTreeEntries(s: Setup): void {
  for (const name of readdirSync(s.cache)) {
    if (name.startsWith("tree2@")) rmSync(`${s.cache}/${name}`, { recursive: true, force: true });
  }
}

describe("§13.19 allow: a layer's account applies only while allowed.yaml holds the same entry (AC 5)", () => {
  it("gives the default account and opens no lock or cache file for a root that has no entry (through alz account which)", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s, { root: `${s.real}/some-other-repository` })]);
    // A cache that is gone would be a miss if the stack were read; here it must make no difference.
    rmSync(s.cache, { recursive: true, force: true });
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).toContain("layer step skipped (this repository is not allowed)");
    expect(result.stdout).not.toContain("alz account allow");
  });

  it.each([
    ["a changed name", { account: { claude: "other-name", codex: "client-a-api" } }],
    ["a target added by the layer", { account: { claude: "client-a" } }],
    ["a changed source", { source: "/somewhere/else/layer" }],
  ])(
    "fails closed on the blocked folder for %s, with the allow command and one warning",
    async (_label, patch) => {
      const s = await built();
      accountFolder(s, CLIENT_A, CLIENT_A_API, ".codex-other");
      writeAllowed(s, [hubEntry(s, patch)]);
      const result = await which(s);
      expect(result.code).toBe(1);
      for (const [name, variable] of [
        ["claude", "CLAUDE_CONFIG_DIR"],
        ["codex", "CODEX_HOME"],
      ]) {
        expect(result.stdout).toContain(`${name}: fails closed (source: layer)`);
        expect(result.stdout).toContain(
          `the hook applies ${variable}=~/.${name}-alz-blocked (the blocked folder)`,
        );
      }
      expect(result.stdout).toContain(
        "layer step skipped (the layers' account declaration changed since it was allowed); fails closed",
      );
      expect(result.stdout).toContain("fix: alz account allow\n");
      expect(result.stdout).not.toContain("--remove");
      expect(warningLines(result.stderr)).toHaveLength(1);
      expect(result.stderr).toContain("was ");
      expect(result.stderr).toContain(", now claude=client-a, codex=client-a-api from ");
      expect(existsSync(`${s.ws.home}/.claude-alz-blocked`)).toBe(false);
      expect(existsSync(`${s.ws.home}/.codex-alz-blocked`)).toBe(false);
    },
  );

  it("fails closed for a target the layer removed from its map", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    hubV3(s, `${HUB_HEAD}account:\n  claude: client-a\n`);
    useStack(s, PERSONAL, ["path:../hub/layer", "v3"], REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    writeAllowed(s, [hubEntry(s)]);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain("claude: fails closed (source: layer)");
    expect(result.stdout).toContain("codex: fails closed (source: layer)");
    expect(result.stdout).toContain("fix: alz account allow\n");
  });

  it("keeps the account across a ref bump of the declaring layer", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    useStack(s, PERSONAL, ["path:../hub/layer", "v2"], REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");
    expect(result.stderr).toBe("");
  });

  it("fails closed on the blocked folder when the allowed source is gone from the stack and the cache misses", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s, { source: "/nowhere/layer" })]);
    removeTreeEntries(s);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      "layer step skipped (the allowed source is no longer in the stack); fails closed",
    );
    expect(result.stdout).toContain(
      "the hook applies CLAUDE_CONFIG_DIR=~/.claude-alz-blocked (the blocked folder)",
    );
    expect(result.stdout).toContain("fix: alz account allow\n");
  });

  it("names `alz account allow --remove` when the layers no longer declare an account", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    hubV3(s, HUB_HEAD);
    writeFileSync(
      `${s.ws.root}/personal/layer.yaml`,
      "name: personal\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
    );
    useStack(s, PERSONAL, ["path:../hub/layer", "v3"], REPO_LAYER);
    expect((await alz(s, ["build"])).code).toBe(0);
    writeAllowed(s, [hubEntry(s)]);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      "layer step skipped (the layers no longer declare an account); fails closed",
    );
    expect(result.stdout).toContain("fix: alz account allow --remove\n");
    expect(result.stdout).toContain(
      "the hook applies CLAUDE_CONFIG_DIR=~/.claude-alz-blocked (the blocked folder)",
    );
  });

  it("applies nothing when two client layers declare", async () => {
    const s = plain();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    const files = (name: string) => ({
      "layer.yaml": `name: ${name}\nkind: client\nvisibility: private\nversion: 0.1.0\naccount: client-a\n`,
      "instructions/10-x.md": `- ${name}.\n`,
    });
    remoteStack(s, [
      {
        url: "https://example.invalid/first.git",
        ref: "v1",
        sha: "3".repeat(40),
        files: {
          "layer.yaml": "name: first\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
          "instructions/10-x.md": "- first.\n",
        },
      },
      {
        url: "https://example.invalid/one.git",
        ref: "v1",
        sha: "1".repeat(40),
        files: files("client-one"),
      },
      {
        url: "https://example.invalid/two.git",
        ref: "v1",
        sha: "2".repeat(40),
        files: files("client-two"),
      },
    ]);
    writeAllowed(s, [hubEntry(s, { source: "https://example.invalid/one.git" })]);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      "layer step skipped (several client layers declare an account); fails closed",
    );
    expect(result.stdout).toContain(
      "the hook applies CLAUDE_CONFIG_DIR=~/.claude-alz-blocked (the blocked folder)",
    );
    expect(result.stdout).toContain("fix: remove one of the declarations, then run alz build");
  });
});

describe("§13.19 fail closed: a named account that cannot apply (AC 1)", () => {
  function expectNoBlockedFolders(s: Setup): void {
    expect(existsSync(`${s.ws.home}/.claude-alz-blocked`)).toBe(false);
    expect(existsSync(`${s.ws.home}/.codex-alz-blocked`)).toBe(false);
  }

  const existing = (s: Setup) =>
    accountFolder(s, ".claude-aaa", ".claude-bbb", ".claude-alz-blocked", ".codex-ccc");

  it("names a missing folder from a use choice and lists the folders that exist", async () => {
    const s = plain();
    existing(s);
    rmSync(`${s.ws.home}/.claude-alz-blocked`, { recursive: true });
    const result = await which(s, {}, ["--use", "claude=nope"]);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain("claude: fails closed (source: use)");
    expect(result.stdout).toContain(
      "use step names nope, but ~/.claude-nope does not exist; fails closed",
    );
    expect(result.stdout).toContain(
      "the hook applies CLAUDE_CONFIG_DIR=~/.claude-nope (the missing folder)",
    );
    expect(result.stdout).toContain("fix: alz account init nope (account folders here: aaa, bbb)");
    expect(result.stdout).toContain("codex: default (source: default)");
    expectNoBlockedFolders(s);
  });

  it("names a missing folder from the user config, never counting the blocked folder as existing", async () => {
    const s = plain();
    existing(s);
    writeAccounts(s, `folders:\n  ${s.real}/repo:\n    claude: alz-blocked\n    codex: ddd\n`);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      "user config step names alz-blocked, but ~/.claude-alz-blocked does not exist; fails closed",
    );
    expect(result.stdout).toContain(
      "fix: the name alz-blocked is reserved for the blocked folders; choose another account name",
    );
    expect(result.stdout).not.toContain("alz account init alz-blocked");
    expect(result.stdout).toContain(
      "user config step names ddd, but ~/.codex-ddd does not exist; fails closed",
    );
    expect(result.stdout).toContain("fix: alz account init ddd (account folders here: ccc)");
  });

  it("names a missing folder from the layer", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A);
    writeAllowed(s, [hubEntry(s)]);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      `claude: client-a (source: layer) CLAUDE_CONFIG_DIR=~/${CLIENT_A}`,
    );
    expect(result.stdout).toContain("codex: fails closed (source: layer)");
    expect(result.stdout).toContain(
      "layer step names client-a-api, but ~/.codex-client-a-api does not exist; fails closed",
    );
    expect(result.stdout).toContain(
      "the hook applies CODEX_HOME=~/.codex-client-a-api (the missing folder)",
    );
    expect(result.stdout).toContain("fix: alz account init client-a-api (no account folders here)");
  });

  it("applies the recorded account on a cache miss while the recorded source is still in the stack", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A);
    writeAllowed(s, [hubEntry(s)]);
    removeTreeEntries(s);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain("claude: fails closed (source: layer)");
    expect(result.stdout).toContain(
      "layer step skipped (cache miss: a cached layer is missing); fails closed",
    );
    expect(result.stdout).toContain(
      `the hook applies CLAUDE_CONFIG_DIR=~/${CLIENT_A} (the recorded account client-a)`,
    );
    expect(result.stdout).toContain(
      `the hook applies CODEX_HOME=~/${CLIENT_A_API} (the recorded account client-a-api; that folder does not exist)`,
    );
    expect(result.stdout).toContain("fix: alz build\n");
    expectNoBlockedFolders(s);
  });

  it("unsets the variable on a cache miss when the recorded account is the default", async () => {
    const s = await built();
    writeAllowed(s, [hubEntry(s, { account: { claude: "default", codex: "default" } })]);
    removeTreeEntries(s);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      "the hook applies CLAUDE_CONFIG_DIR unset (the recorded account default)",
    );
    expect(result.stdout).toContain(
      "the hook applies CODEX_HOME unset (the recorded account default)",
    );
  });

  it("treats a cache that XDG_CACHE_HOME moved as a miss while mise or direnv is active here", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    writeAllowed(s, [hubEntry(s)]);
    expect((await which(s)).code).toBe(0);
    for (const manager of ["__MISE_DIFF", "DIRENV_DIFF"]) {
      const result = await which(s, { [manager]: MARKER });
      expect(result.code).toBe(1);
      expect(result.stdout).toContain(
        "layer step skipped (cache miss: XDG_CACHE_HOME points away from ~/.cache and mise or direnv is active here); fails closed",
      );
      expect(result.stdout).toContain(
        `the hook applies CLAUDE_CONFIG_DIR=~/${CLIENT_A} (the recorded account client-a)`,
      );
      expect(result.stdout + result.stderr).not.toContain(MARKER);
    }
  });

  it("skips the layer step without failing for a root that is not allowed, even on a cache miss", async () => {
    const s = await built();
    removeTreeEntries(s);
    const result = await which(s);
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("claude: default (source: default)");
  });
});

describe("§13.19 reads only: no git, no write, no fetch (AC 3)", () => {
  async function prepared() {
    const s = await built();
    accountFolder(s, CLIENT_A_API);
    // A planted account folder with a file in it: alz may only list its name.
    mkdirSync(`${s.ws.home}/${CLIENT_A}`);
    writeFileSync(`${s.ws.home}/${CLIENT_A}/.credentials.json`, "secret");
    writeAccounts(s, `default_name: personal\nfolders:\n  ${s.real}/elsewhere: client-a\n`);
    writeAllowed(s, [hubEntry(s)]);
    return s;
  }

  it("proves the proxies work: a build is seen running git and writing (through the git stand-in)", async () => {
    const s = await built("v1");
    useStack(s, PERSONAL, ["path:../hub/layer", "v2"], REPO_LAYER);
    const proxy = gitProxy();
    const before = world(s);
    const result = await alz(s, ["build"], proxy.env);
    expect(result.code, result.stderr).toBe(0);
    expect(proxy.calls().length).toBeGreaterThan(0);
    expect(world(s)).not.toEqual(before);
  });

  it("never runs git, writes or opens anything it need not, in an allowed repository (through alz account which)", async () => {
    const s = await prepared();
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");
    expect(result.stdout).toContain("codex: client-a-api (source: layer)");
    expect(result.stderr).toBe("");
    // The planted folder's name is listed (it is how the account is found); its content is not.
    expect(result.stdout + result.stderr).not.toContain("secret");
  });

  it("opens no stack file for a repository that is not allowed, and runs no git (through alz account which)", async () => {
    const s = await prepared();
    writeAllowed(s, [hubEntry(s, { root: `${s.real}/other` })]);
    // With the cache gone, reading the stack would be a cache miss and fail closed.
    rmSync(s.cache, { recursive: true, force: true });
    const result = await which(s);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: default, called personal here (source: default)");
    expect(result.stdout).toContain("layer step skipped (this repository is not allowed)");
  });
});

describe("§13.19 quoting: repository text never reaches the terminal raw (AC 1)", () => {
  /** Any C0, DEL or C1 character other than a newline: what could move a terminal's cursor or title. */
  const hasControl = (text: string): boolean =>
    [...text].some((char) => {
      const code = char.codePointAt(0) ?? 0;
      return code !== 0x0a && (code <= 0x1f || (code >= 0x7f && code <= 0x9f));
    });

  it("proves the control-character check can fail", () => {
    expect(hasControl("a\u001b[31mb")).toBe(true);
    expect(hasControl("a\nb")).toBe(false);
  });

  it("prints a directory named $(touch pwned) with an ESC above it as text, creates nothing and evaluates nothing", async () => {
    const s = plain();
    const folder = "we\u001b[31mird/$(touch pwned)";
    mkdirSync(`${s.ws.root}/${folder}`, { recursive: true });
    writeFileSync(`${s.ws.root}/${folder}/layer.yaml`, "stack:\n  - source: path:x\n");
    writeAccounts(s, "default_name: personal\n");
    const result = await which(
      s,
      { CLAUDE_CONFIG_DIR: "/odd\u001b]0;title\u0007/path" },
      [],
      folder,
    );
    expect(result.code).toBe(0);
    expect(result.stdout).toContain("$(touch pwned)");
    expect(hasControl(result.stdout + result.stderr)).toBe(false);
    expect(existsSync(`${s.ws.root}/${folder}/pwned`)).toBe(false);
    expect(existsSync(`${process.cwd()}/pwned`)).toBe(false);
  });

  it("prints the source of a changed entry with its control characters removed", async () => {
    const s = await built();
    accountFolder(s, CLIENT_A, CLIENT_A_API);
    const entry = {
      root: `${s.real}/repo`,
      account: { claude: "client-a" },
      source: "path:/x\u001b[31m/y\u0007",
    };
    writeAllowed(s, [], JSON.stringify([entry]));
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain('from "path:/x[31m/y"');
    expect(hasControl(result.stdout + result.stderr)).toBe(false);
  });
});

describe("§13.19 review follow-ups: ordering of the checks, folders that are only contained, bad choices", () => {
  describe.each(["__MISE_DIFF", "DIRENV_DIFF"])(
    "a lookup %s moved to a folder that does not exist",
    (manager) => {
      it("is ignored with the XDG warning for accounts.yaml, not reported as not found", async () => {
        const s = plain();
        const moved = { XDG_CONFIG_HOME: `${s.real}/nowhere`, [manager]: MARKER };
        const result = await which(s, moved);
        expect(result.code).toBe(0);
        expect(result.stdout).toContain(
          `accounts.yaml: ${s.real}/nowhere/agent-layerz/accounts.yaml (ignored: XDG_CONFIG_HOME points away from its default and mise or direnv is active here)`,
        );
        expect(result.stdout).not.toContain("(not found)\nallowed");
        expect(result.stderr).toContain(
          "XDG_CONFIG_HOME points away from its default and mise or direnv is active here",
        );
        const without = await which(s, { XDG_CONFIG_HOME: `${s.real}/nowhere` });
        expect(without.stdout).toContain("nowhere/agent-layerz/accounts.yaml (not found)");
      });

      it("is ignored with the XDG warning for allowed.yaml, so the layer step is skipped", async () => {
        const s = await built();
        accountFolder(s, CLIENT_A, CLIENT_A_API);
        const result = await which(s, { XDG_STATE_HOME: `${s.real}/nowhere`, [manager]: MARKER });
        expect(result.stdout).toContain(
          `allowed.yaml: ${s.real}/nowhere/agent-layerz/allowed.yaml (ignored: XDG_STATE_HOME points away from its default and mise or direnv is active here)`,
        );
        expect(result.stdout).toContain("layer step skipped (allowed.yaml is ignored)");
        expect(result.stderr).toContain("XDG_STATE_HOME points away");
        expect(result.stdout + result.stderr).not.toContain(MARKER);
      });
    },
  );

  describe("a repository root that merely contains a configured account folder", () => {
    it("stays a root and fails closed on the recorded account, never the default account", async () => {
      const s = await built();
      mkdirSync(`${s.real}/repo/.claude-local`);
      writeAllowed(s, [hubEntry(s, { account: { codex: "client-c" } })]);
      const result = await which(s, { CLAUDE_CONFIG_DIR: `${s.real}/repo/.claude-local` });
      expect(result.code).toBe(1);
      expect(result.stdout).toContain(`repository: ${s.real}/repo\n`);
      expect(result.stdout).toContain("claude: set outside alz (source: environment)");
      expect(result.stdout).toContain("codex: fails closed (source: layer)");
      expect(result.stdout).toContain(
        "layer step skipped (cache miss: a layer or the repository lies inside an account folder); fails closed",
      );
      expect(result.stdout).toContain(
        "the hook applies CODEX_HOME=~/.codex-client-c (the recorded account client-c; that folder does not exist)",
      );
      expect(result.stdout).toContain("fix: alz build");
    });

    it("gives the default account when that root is not allowed", async () => {
      const s = await built();
      mkdirSync(`${s.real}/repo/.codex-local`);
      writeAllowed(s, [hubEntry(s, { root: `${s.real}/other` })]);
      const result = await which(s, { CODEX_HOME: `${s.real}/repo/.codex-local` });
      expect(result.code).toBe(0);
      expect(result.stdout).toContain(`repository: ${s.real}/repo\n`);
      expect(result.stdout).toContain("layer step skipped (this repository is not allowed)");
    });

    it("still stops at the configured folder itself", async () => {
      const s = await built();
      mkdirSync(`${s.real}/repo/.claude-local`);
      const result = await which(
        s,
        { CLAUDE_CONFIG_DIR: `${s.real}/repo/.claude-local` },
        [],
        "repo/.claude-local",
      );
      expect(result.stdout).toContain(
        "repository: none (the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names)",
      );
    });
  });

  describe("a working directory inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names (mise or direnv can set it)", () => {
    it.each([
      ["the repository root", (s: Setup) => `${s.real}/repo`],
      ["a parent of the repository", (s: Setup) => s.real],
    ])(
      "fails closed on the blocked folder for an allowed repository when the variable names %s",
      async (_label, configured) => {
        const s = await built();
        writeAllowed(s, [hubEntry(s, { account: { codex: "client-c" } })]);
        for (const dir of ["repo", "repo/sub/deeper"]) {
          const result = await which(s, { CLAUDE_CONFIG_DIR: configured(s) }, [], dir);
          expect(result.code, dir).toBe(1);
          expect(result.stdout, dir).toContain(
            "repository: none (the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names)",
          );
          expect(result.stdout, dir).toContain("claude: set outside alz (source: environment)");
          expect(result.stdout, dir).toContain("codex: fails closed (source: layer)");
          expect(result.stdout, dir).toContain(
            "layer step skipped (an allowed repository lies in a folder CLAUDE_CONFIG_DIR or CODEX_HOME names, so its layers are not read); fails closed",
          );
          expect(result.stdout, dir).toContain(
            "the hook applies CODEX_HOME=~/.codex-alz-blocked (the blocked folder)",
          );
          expect(existsSync(`${s.ws.home}/.codex-alz-blocked`)).toBe(false);
        }
      },
    );

    it("applies the default account when no allowed entry covers the working directory", async () => {
      const s = await built();
      writeAllowed(s, [hubEntry(s, { root: `${s.real}/other`, account: { codex: "client-c" } })]);
      const result = await which(s, { CLAUDE_CONFIG_DIR: `${s.real}/repo` });
      expect(result.code).toBe(0);
      expect(result.stdout).toContain("codex: default (source: default)");
      expect(result.stdout).toContain(
        "layer step skipped (the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names)",
      );
    });
  });

  it("stops `alz account which --use` with a bad name before any lower step, via the usage check", async () => {
    const s = plain();
    const result = await which(s, {}, ["--use", "claude=../x"]);
    expect(result.code).toBe(2);
  });

  describe("a HOME that is not an absolute path", () => {
    it("ignores both files with one warning, sets no folder and fails closed for a named account", async () => {
      const s = plain();
      const plainRun = await which(s, { HOME: "relative/home" });
      expect(plainRun.code).toBe(0);
      expect(plainRun.stdout).toContain("(ignored: HOME is not an absolute path)");
      expect(plainRun.stdout).toContain(
        "claude: default (source: default) CLAUDE_CONFIG_DIR unset",
      );
      expect(warningLines(plainRun.stderr)).toHaveLength(1);
      expect(plainRun.stderr).toContain("HOME is not an absolute path");

      const named = await which(s, { HOME: "relative/home" }, ["--use", "claude=x"]);
      expect(named.code).toBe(1);
      expect(named.stdout).toContain("claude: fails closed (source: use)");
      expect(named.stdout).toContain("the hook sets no folder");
      expect(named.stdout).not.toContain("CLAUDE_CONFIG_DIR=");
      expect(existsSync(`${process.cwd()}/relative`)).toBe(false);
    });
  });

  it("removes control characters from an unknown command, subcommand or option in the usage error", async () => {
    const s = plain();
    for (const args of [
      ["bad\u001b[31mcommand"],
      ["account", "bad\u001b[31msub"],
      ["--bad\u001b[31m"],
    ]) {
      const result = await alz(s, args);
      expect(result.code).toBe(2);
      expect([...result.stderr].some((char) => char === "\u001b")).toBe(false);
    }
  });

  it("prints a cwd at or above $HOME as no repository, not as an account folder", async () => {
    const s = plain();
    writeFileSync(`${s.ws.home}/layer.yaml`, "stack:\n  - source: path:x\n");
    for (const dir of ["../home", "."]) {
      const result = await which(s, {}, [], dir);
      expect(result.stdout, dir).toContain("repository: none (no repository here)");
      expect(result.stdout, dir).toContain("layer step skipped (no repository here)");
    }
  });

  describe("the working directory is resolved once, and never inside an account folder", () => {
    it("makes no file system call under an account folder, and skips the user config step (through alz account which)", async () => {
      const s = plain();
      const folder = `${s.ws.home}/${CLIENT_A}`;
      mkdirSync(`${folder}/project`, { recursive: true });
      writeFileSync(`${folder}/.credentials.json`, "secret");
      writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n`);
      const result = await which(s, {}, [], `../home/${CLIENT_A}/project`);
      expect(result.stdout).toContain(
        "repository: none (the working directory is inside an account folder)",
      );
      expect(result.stdout).toContain("claude: default (source: default)");
      expect(result.stdout).toContain(
        "user config step skipped (the working directory is inside an account folder)",
      );
      expect(result.stdout + result.stderr).not.toContain("secret");
    });

    it("treats a link into an account folder as an account folder, and hands the file system only the link's own path (through alz account which)", async () => {
      const s = plain();
      const folder = `${s.ws.home}/${CLIENT_A}`;
      mkdirSync(`${folder}/project`, { recursive: true });
      symlinkSync(`${folder}/project`, `${s.real}/link-project`);
      const result = await which(s, {}, [], "link-project");
      expect(result.stdout).toContain(
        "repository: none (the working directory is inside an account folder)",
      );
      expect(result.stdout).toContain(
        "user config step skipped (the working directory is inside an account folder)",
      );
    });
  });

  describe("text that names an account folder is ignored before any file system call on it", () => {
    it("covers a folders key, an allowed root and a stack source (through alz account which)", async () => {
      const s = await built();
      for (const name of [".claude-x", ".claude-y"]) {
        mkdirSync(`${s.ws.home}/${name}/agent-layerz/project`, { recursive: true });
      }
      writeAccounts(s, "folders:\n  ~/.claude-x/agent-layerz/project: x\n");
      writeAllowed(s, [
        hubEntry(s, { root: `${s.ws.home}/.claude-y/agent-layerz/project` }),
        hubEntry(s),
      ]);
      useStack(
        s,
        PERSONAL,
        ["path:../hub/layer", "v1"],
        [`path:${s.ws.home}/.claude-x/agent-layerz/project`],
      );
      for (const name of readdirSync(s.cache)) {
        if (name.startsWith("tree2@"))
          rmSync(`${s.cache}/${name}`, { recursive: true, force: true });
      }
      const result = await which(s);
      expect(warningLines(result.stderr).length).toBeGreaterThanOrEqual(2);
      expect(result.stderr).toContain("lies inside an account folder, or is $HOME or above it");
      expect(result.stderr).toContain(
        "allowed.yaml: entry 1 is ignored (its root lies inside an account folder",
      );
      // The one entry whose root is a real repository still applies: its cache is gone, so it fails closed.
      expect(result.code).toBe(1);
      expect(result.stdout).toContain(
        "layer step skipped (cache miss: a cached layer is missing); fails closed",
      );
    });
  });

  describe("a folders key or an allowed root is checked again once it is real", () => {
    it("ignores a link into an account folder, and $HOME itself, as a folders key", async () => {
      const s = plain();
      accountFolder(s, CLIENT_A);
      symlinkSync(`${s.ws.home}/${CLIENT_A}`, `${s.real}/link-account`);
      writeAccounts(
        s,
        `folders:\n  ${s.real}/link-account: x\n  ~: y\n  ${s.real}/repo: client-b\n`,
      );
      accountFolder(s, ".claude-client-b");
      const result = await which(s);
      expect(result.stdout).toContain("claude: client-b (source: user config)");
      expect(warningLines(result.stderr)).toHaveLength(2);
      expect(result.stderr).toContain("lies inside an account folder, or is $HOME or above it");
      expect(result.stderr).toContain("link-account");
    });
  });

  describe("a symbolic link as accounts.yaml: both folders are checked", () => {
    function linked(s: Setup): { linkDir: string; targetDir: string } {
      const targetDir = `${s.real}/elsewhere/agent-layerz`;
      writeAt(targetDir, "accounts.yaml", `folders:\n  ${s.real}/repo: client-b\n`);
      const linkDir = configDir(s);
      mkdirSync(linkDir, { recursive: true });
      chmodSync(`${s.ws.home}/.config`, 0o755);
      chmodSync(linkDir, 0o700);
      symlinkSync(`${targetDir}/accounts.yaml`, `${linkDir}/accounts.yaml`);
      accountFolder(s, ".claude-client-b");
      return { linkDir, targetDir };
    }

    it("reads it when both folders are safe", async () => {
      const s = plain();
      linked(s);
      const result = await which(s);
      expect(result.stdout).toContain("claude: client-b (source: user config)");
    });

    it.each([
      ["the folder of the link", (d: { linkDir: string }) => chmodSync(d.linkDir, 0o770)],
      ["the folder of the target", (d: { targetDir: string }) => chmodSync(d.targetDir, 0o770)],
    ])("ignores it when %s can be written by group", async (_label, weaken) => {
      const s = plain();
      weaken(linked(s) as never);
      const result = await which(s);
      expect(result.stdout).toContain("claude: default (source: default)");
      expect(result.stderr).toContain("can be written by group or others");
    });
  });

  describe("the 1 MiB and alias limits apply to both files", () => {
    const padding = `#${"x".repeat(1024 * 1024)}\n`;
    const aliases = `a: &a [x]\nb: [${Array.from({ length: 150 }, () => "*a").join(", ")}]\n`;

    it("ignores an accounts.yaml over 1 MiB or past the alias limit", async () => {
      const s = plain();
      accountFolder(s, ".claude-client-b");
      writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n${padding}`);
      const big = await which(s);
      expect(big.stdout).toContain(
        "(ignored: it is not a regular file of at most 1 MiB that can be read)",
      );
      expect(big.stdout).toContain("claude: default (source: default)");
      writeAccounts(s, `folders:\n  ${s.real}/repo: client-b\n${aliases}`);
      const bomb = await which(s);
      expect(bomb.stdout).toContain("(ignored: it is not valid YAML)");
      expect(bomb.stdout).toContain("claude: default (source: default)");
    });

    it("ignores an allowed.yaml over 1 MiB or past the alias limit", async () => {
      const s = await built();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      const entry = `- root: ${s.real}/repo\n  account:\n    claude: client-a\n    codex: client-a-api\n  source: ${s.hubLayer}\n`;
      writeAllowed(s, [], `${entry}${padding}`);
      const big = await which(s);
      expect(big.stdout).toContain(
        "allowed.yaml: ~/.local/state/agent-layerz/allowed.yaml (ignored: it is not a regular file",
      );
      expect(big.stdout).toContain("claude: default (source: default)");
      writeAllowed(s, [], `${entry}${aliases}`);
      const bomb = await which(s);
      expect(bomb.stdout).toContain("(ignored: it is not valid YAML)");
    });
  });

  describe("repository text with control characters in the changed and ignored warnings", () => {
    const ESC_URL = (name: string) => `https://example.invalid/${name}\u001b[31m.git`;
    const layer = (name: string, kind: string, visibility: string, account: string) => ({
      "layer.yaml": `name: ${name}\nkind: ${kind}\nvisibility: ${visibility}\nversion: 0.1.0\n${account}`,
      "instructions/10-x.md": `- ${name}.\n`,
    });
    const hasEsc = (text: string) => [...text].some((char) => char === "\u001b");

    it("quotes the source of an ignored declaration and the sources of a changed entry", async () => {
      const s = plain();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      remoteStack(s, [
        {
          url: "https://example.invalid/first.git",
          ref: "v1",
          sha: "3".repeat(40),
          files: layer("first", "personal", "public", ""),
        },
        {
          url: ESC_URL("client"),
          ref: "v1",
          sha: "1".repeat(40),
          files: layer(
            "client",
            "client",
            "private",
            "account:\n  claude: client-a\n  codex: client-a-api\n",
          ),
        },
        {
          url: ESC_URL("group"),
          ref: "v1",
          sha: "2".repeat(40),
          files: layer("group", "group", "private", "account: group-a\n"),
        },
      ]);
      writeAllowed(s, [hubEntry(s, { source: `https://old.invalid/c\u001b[32m.git` })]);
      const result = await which(s);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain('"git:https://example.invalid/group[31m.git@v1"');
      expect(result.stderr).toContain('from "https://old.invalid/c[32m.git"');
      expect(result.stderr).toContain('from "https://example.invalid/client[31m.git"');
      expect(hasEsc(result.stdout + result.stderr)).toBe(false);
    });

    it("refuses a layer whose name holds a control character as a cache miss and prints none", async () => {
      const s = plain();
      accountFolder(s, CLIENT_A, CLIENT_A_API);
      remoteStack(s, [
        {
          url: "https://example.invalid/client.git",
          ref: "v1",
          sha: "1".repeat(40),
          files: {
            "layer.yaml": `name: "cli\\e[31mnt"\nkind: client\nvisibility: private\nversion: 0.1.0\naccount: client-a\n`,
            "instructions/10-x.md": "- x.\n",
          },
        },
      ]);
      writeAllowed(s, [hubEntry(s, { source: "https://example.invalid/client.git" })]);
      const result = await which(s);
      expect(result.code).toBe(1);
      expect(result.stdout).toContain("cache miss: a layer is not valid");
      expect(hasEsc(result.stdout + result.stderr)).toBe(false);
    });
  });

  it("unsets the variable on a cache miss when the recorded account is the configured default_name", async () => {
    const s = await built();
    writeAccounts(s, "default_name: personal\n");
    writeAllowed(s, [hubEntry(s, { account: { claude: "personal", codex: "personal" } })]);
    removeTreeEntries(s);
    const result = await which(s);
    expect(result.code).toBe(1);
    expect(result.stdout).toContain(
      "the hook applies CLAUDE_CONFIG_DIR unset (the recorded account default)",
    );
    expect(result.stdout).toContain(
      "the hook applies CODEX_HOME unset (the recorded account default)",
    );
  });
});
