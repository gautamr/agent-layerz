import {
  appendFileSync,
  chmodSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { describe, expect, it } from "vitest";
import YAML from "yaml";
import { tool } from "../exec";
import { fixture, layers, readFile, readTree, type Workspace } from "../helpers";

// Reserved names only. A refused value must never appear in any output.
const TOKEN_LIKE = "AKIAIOSFODNN7EXAMPLE";

function declare(ws: Workspace, layer: string, yaml: string): void {
  appendFileSync(`${ws.root}/${layer}/layer.yaml`, `account:${yaml}\n`);
}

/** The value as YAML: JSON is YAML, so a string or a map is written exactly as given. */
function value(raw: unknown): string {
  return ` ${JSON.stringify(raw)}`;
}

function lintLayer(ws: Workspace, layer: string) {
  return layers(ws, "repo", ["lint", "--layer", `../${layer}`]);
}

function git(ws: Workspace, dir: string, args: string[]): void {
  tool("git", args, {
    cwd: `${ws.root}/${dir}`,
    env: { PATH: process.env.PATH ?? "", HOME: ws.home, GIT_CONFIG_NOSYSTEM: "1" },
  });
}

describe("§13.24 account layer field: schema (§7.13 'Account names')", () => {
  it.each([
    ["a name", value("client-a")],
    ["the name default", value("default")],
    ["a name of 64 characters", value("a".repeat(64))],
    ["a name that starts with a digit", value("2client")],
    ["a name for every target", value({ claude: "client-a", codex: "client-a-api" })],
    ["a name for one target", value({ codex: "client-a" })],
  ])("accepts %s", async (_label, yaml) => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", yaml);
    const result = await lintLayer(ws, "personal");
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  it.each([
    ["65 characters", "a".repeat(65)],
    ["upper case", "Client-A"],
    ["a leading '-'", "-leading"],
    ["a home path", "~/secret-dir"],
    ["an absolute path", "/etc/secret-dir"],
    ["a parent path", "../secret-dir"],
    ["a slash", "a/secret-dir"],
    ["a token-like value", TOKEN_LIKE],
    ["a space", "client a"],
    ["an empty string", ""],
    ["a map with an unknown target", { cursor: "secret-dir" }],
    ["a map with a path", { claude: "~/secret-dir" }],
    ["a map with a name too long", { codex: "a".repeat(65) }],
    ["an empty map", {}],
    ["null", null],
    ["a number", 5],
    ["a boolean", true],
    ["a list", ["secret-dir"]],
    ["a nested map", { claude: { name: "secret-dir" } }],
    ["a map with a null name", { claude: null }],
  ])("lint refuses %s and prints no value", async (_label, raw) => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value(raw));
    const result = await lintLayer(ws, "personal");
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [layer]");
    // §10.1 prints key paths unquoted: `account`, `account.claude`.
    expect(result.stderr).toContain("account");
    for (const text of [result.stdout, result.stderr]) {
      expect(text).not.toContain("secret-dir");
      expect(text).not.toContain(TOKEN_LIKE);
      expect(text).not.toContain("Client-A");
      expect(text).not.toContain("client a");
    }
  });

  it("is a schema error for the build too, and writes nothing", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value("~/secret-dir"));
    const tree = readTree(`${ws.root}/repo`);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("'account'");
    expect(result.stderr).not.toContain("secret-dir");
    expect(readTree(`${ws.root}/repo`)).toEqual(tree);
  });
});

/** The allowed.yaml entry `alz account allow` records: this repository, the declaring layer's real path. */
function allow(ws: Workspace, layer: string, account: Record<string, string>): void {
  const real = realpathSync.native(ws.root);
  const state = `${ws.home}/.local/state/agent-layerz`;
  mkdirSync(state, { recursive: true });
  for (const folder of [`${ws.home}/.local`, `${ws.home}/.local/state`]) chmodSync(folder, 0o755);
  chmodSync(state, 0o700);
  const entry = { root: `${real}/repo`, account, source: `${real}/${layer}` };
  writeFileSync(`${state}/allowed.yaml`, YAML.stringify([entry], { lineWidth: 0 }), {
    mode: 0o600,
  });
  chmodSync(`${state}/allowed.yaml`, 0o600);
  for (const [target, name] of Object.entries(account)) {
    mkdirSync(`${ws.home}/.${target}-${name}`, { recursive: true });
  }
}

/** Builds, as resolution reads the lock, then runs `alz account which` with every XDG location inside the isolated HOME. */
async function which(ws: Workspace) {
  const env = {
    XDG_STATE_HOME: `${ws.home}/.local/state`,
    XDG_CONFIG_HOME: `${ws.home}/.config`,
    XDG_CACHE_HOME: `${realpathSync.native(ws.root)}/cache`,
  };
  const build = await layers(ws, "repo", ["build"], env);
  expect(build.code, build.stderr).toBe(0);
  return layers(ws, "repo", ["account", "which"], env);
}

// The winning declaration is seen through `alz account which`, once the person allowed it.
describe("§13.24 account layer field: who wins (§7.13 'Who may declare it')", () => {
  it("takes the first layer's declaration when no client layer declares", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value("personal-a"));
    allow(ws, "personal", { claude: "personal-a", codex: "personal-a" });
    const result = await which(ws);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: personal-a (source: layer)");
    expect(result.stdout).toContain("codex: personal-a (source: layer)");
  });

  it("applies a name to every target whatever the manifest's targets are", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value("personal-a"));
    allow(ws, "personal", { claude: "personal-a", codex: "personal-a" });
    expect(readFile(ws, "repo/layer.yaml")).toContain("targets: [claude]");
    expect((await which(ws)).stdout).toContain("codex: personal-a (source: layer)");
  });

  it("lets the client layer's whole value replace the first layer's, a map included", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value({ claude: "personal-a", codex: "personal-b" }));
    declare(ws, "client", value({ claude: "client-a" }));
    allow(ws, "client", { claude: "client-a" });
    const result = await which(ws);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");
    // The maps are never mixed: the first layer's codex name does not survive.
    expect(result.stdout).toContain("codex: default (source: default)");
    expect(result.stdout).toContain("layer step skipped (the layers name no codex account)");
    expect(result.stdout).not.toContain("personal-b");
  });

  it("lets a confidential client layer declare", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value("personal-a"));
    const path = `${ws.root}/client/layer.yaml`;
    writeFileSync(path, readFileSync(path, "utf8").replace("private", "confidential"));
    // The output visibility follows the most private layer, so the repo layer must not be less private.
    const repoMeta = `${ws.root}/repo/.layer/layer.yaml`;
    writeFileSync(repoMeta, readFileSync(repoMeta, "utf8").replace("private", "confidential"));
    declare(ws, "client", value("client-a"));
    allow(ws, "client", { claude: "client-a", codex: "client-a" });
    const result = await which(ws);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");
    expect(result.stdout).toContain("codex: client-a (source: layer)");
  });

  it("declares nothing when no layer does", async () => {
    const ws = fixture("29-account-layer");
    const result = await which(ws);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: default (source: default)");
    expect(result.stdout).toContain("codex: default (source: default)");
  });
});

describe("§13.24 account layer field: no effect on output yet", () => {
  it("builds byte-identical files with and without a declaration, and names no account", async () => {
    const plain = fixture("29-account-layer");
    const declared = fixture("29-account-layer");
    declare(declared, "personal", value("personal-secret-name"));
    declare(declared, "client", value({ claude: "client-secret-name", codex: "client-codex" }));
    expect((await layers(plain, "repo", ["build"])).code).toBe(0);
    expect((await layers(declared, "repo", ["build"])).code).toBe(0);

    const without = readTree(`${plain.root}/repo`);
    const withDecl = readTree(`${declared.root}/repo`);
    // The lock records each layer's content hash, which covers layer.yaml; its file list must not change.
    const { "layer.lock": lockWithout = "", ...restWithout } = without;
    const { "layer.lock": lockWith = "", ...restWith } = withDecl;
    expect(restWith).toEqual(restWithout);
    expect(JSON.parse(lockWith).files).toEqual(JSON.parse(lockWithout).files);
    for (const text of [...Object.values(withDecl)]) {
      expect(text).not.toContain("secret-name");
      expect(text).not.toContain("client-codex");
    }
  });
});

describe("§13.24 account layer field: init", () => {
  it("prints a refused declaration as a warning, exits 0 and writes no identity", async () => {
    const ws = fixture("29-account-layer");
    appendFileSync(
      `${ws.root}/personal/layer.yaml`,
      "commit_identity:\n  name: Owner Example\n  email: owner@example.com\n",
    );
    declare(ws, "group", value("group-a"));
    mkdirSync(`${ws.root}/fresh`);
    git(ws, "fresh", ["init", "--quiet", "--initial-branch=main"]);
    const result = await layers(ws, "fresh", [
      "init",
      "--stack",
      "../personal",
      "--stack",
      "../group",
    ]);
    expect(result.code).toBe(0);
    expect(result.stderr).toContain("warning [account] group:layer.yaml");
    expect(result.stderr).not.toContain("group-a");
    expect(result.stdout).not.toContain("set user.");
    expect(readFileSync(`${ws.root}/fresh/.git/config`, "utf8")).not.toContain("user");
    // build fails on the same problem.
    expect((await layers(ws, "fresh", ["build"])).code).toBe(1);
  });
});
