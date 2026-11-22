import { execFileSync } from "node:child_process";
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { evaluateDeclarations } from "../../src/merge/declarers";
import { codexPatternMatches, isAccountName, parseAccount } from "../../src/model/account";
import type { Layer } from "../../src/model/layer";
import type { LayerMeta } from "../../src/model/layer-meta";
import { computeBuild } from "../../src/pipeline";
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

function config(ws: Workspace) {
  const home = ws.home;
  const result = computeBuild({
    repoRoot: `${ws.root}/repo`,
    home,
    env: { HOME: home, PATH: process.env.PATH ?? "" },
  });
  expect(result.diags.errors).toEqual([]);
  return result.config;
}

function git(ws: Workspace, dir: string, args: string[]): void {
  execFileSync("git", args, {
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
    expect(result.stderr).toContain("'account");
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

  it("parses a name to the same name for every target and a map as written", () => {
    expect(parseAccount("client-a")).toEqual({ claude: "client-a", codex: "client-a" });
    expect(parseAccount({ codex: "x1" })).toEqual({ codex: "x1" });
    expect(isAccountName("a".repeat(64))).toBe(true);
    expect(isAccountName("a".repeat(65))).toBe(false);
  });
});

describe("§13.24 account layer field: who wins (§7.13 'Who may declare it')", () => {
  it("takes the first layer's declaration when no client layer declares", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value("personal-a"));
    const declared = config(ws).account;
    expect(declared?.layer.meta.name).toBe("personal");
    expect(declared?.account).toEqual({ claude: "personal-a", codex: "personal-a" });
  });

  it("applies a name to every target whatever the manifest's targets are", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value("personal-a"));
    expect(readFile(ws, "repo/layer.yaml")).toContain("targets: [claude]");
    expect(config(ws).account?.account.codex).toBe("personal-a");
  });

  it("lets the client layer's whole value replace the first layer's, a map included", async () => {
    const ws = fixture("29-account-layer");
    declare(ws, "personal", value({ claude: "personal-a", codex: "personal-b" }));
    declare(ws, "client", value({ claude: "client-a" }));
    const declared = config(ws).account;
    expect(declared?.layer.meta.name).toBe("client");
    expect(declared?.account).toEqual({ claude: "client-a" });
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
    const declared = config(ws).account;
    expect(declared?.layer.meta.name).toBe("client");
    expect(declared?.account).toEqual({ claude: "client-a", codex: "client-a" });
  });

  it("declares nothing when no layer does", async () => {
    const ws = fixture("29-account-layer");
    expect(config(ws).account).toBeUndefined();
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

describe("codexPatternMatches mirrors how Codex matches policy patterns (target-facts §12 item 15)", () => {
  it.each([
    ["*", "CODEX_HOME", true],
    ["?ODEX_HOME", "CODEX_HOME", true],
    ["*home*", "CODEX_HOME", true],
    ["XDG_*", "XDG_STATE_HOME", true],
    ["*_HOME", "CODEX_HOME", true],
    ["codex_home", "CODEX_HOME", true],
    ["HOME", "HOME", true],
    ["H?ME", "HOME", true],
    ["*FOO*", "CODEX_HOME", false],
    ["?HOME", "HOME", false],
    ["HOM", "HOME", false],
    ["HOMES", "HOME", false],
    ["XDG_*", "HOME", false],
    ["", "HOME", false],
    ["*", "", true],
  ])("pattern %j against %s is %s", (pattern, name, expectedMatch) => {
    expect(codexPatternMatches(pattern, name)).toBe(expectedMatch);
  });
});

describe("evaluateDeclarations leaves no winner when the declarations conflict", () => {
  // `github:` sources are never inside the repository; the repo root need not exist.
  const REPO = "/nonexistent/alz-repo";

  function layer(name: string, index: number, meta: Partial<LayerMeta>): Layer {
    return {
      meta: {
        name,
        kind: index === 0 ? "personal" : "client",
        visibility: "private",
        providesSlots: [],
        requiresSlots: [],
        forbidPatterns: [],
        ...meta,
      },
      dir: `/nonexistent/${name}`,
      source: `github:example/${name}`,
      files: [],
      contentHash: "",
    };
  }
  const pick = (candidate: Layer) => candidate.meta.account;

  it("names the first layer's value when it alone declares", () => {
    const first = layer("first", 0, { account: { claude: "a" } });
    const result = evaluateDeclarations([first, layer("other", 1, {})], REPO, pick);
    expect(result.winner?.layer.meta.name).toBe("first");
    expect(result.conflict).toBe(false);
  });

  it("names the client layer's value over the first layer's", () => {
    const first = layer("first", 0, { account: { claude: "a" } });
    const client = layer("client", 1, { account: { codex: "b" } });
    const result = evaluateDeclarations([first, client], REPO, pick);
    expect(result.winner?.value).toEqual({ codex: "b" });
    expect(result.conflict).toBe(false);
  });

  it("names no winner when two client layers declare", () => {
    const first = layer("first", 0, { account: { claude: "a" } });
    const one = layer("one", 1, { account: { claude: "b" } });
    const two = layer("two", 2, { account: { claude: "c" } });
    const result = evaluateDeclarations([first, one, two], REPO, pick);
    expect(result.winner).toBeUndefined();
    expect(result.conflict).toBe(true);
    expect(result.clients.map((entry) => entry.meta.name)).toEqual(["one", "two"]);
  });

  it("names no winner when the first layer's declaration was refused", () => {
    // A first layer that sits inside the repository may not declare.
    const first = { ...layer("first", 0, { account: { claude: "a" } }), source: "path:./inside" };
    const client = layer("client", 1, { account: { claude: "b" } });
    const result = evaluateDeclarations([first, client], REPO, pick);
    expect(result.winner).toBeUndefined();
    expect(result.conflict).toBe(true);
    expect(result.refused.map((entry) => entry.layer.meta.name)).toEqual(["first"]);
  });

  it("still names the winner when a later layer that may not declare is refused", () => {
    const first = layer("first", 0, { account: { claude: "a" } });
    const group = layer("group", 1, { kind: "group", account: { claude: "g" } });
    const result = evaluateDeclarations([first, group], REPO, pick);
    expect(result.winner?.layer.meta.name).toBe("first");
    expect(result.refused.map((entry) => entry.layer.meta.name)).toEqual(["group"]);
  });
});
