import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { evaluateDeclarations } from "../../src/merge/declarers";
import { codexPatternMatches, isAccountName, parseAccount } from "../../src/model/account";
import type { Layer } from "../../src/model/layer";
import type { LayerMeta } from "../../src/model/layer-meta";
import { computeBuild } from "../../src/pipeline";
import { fixture, readFile, type Workspace } from "../helpers";

// The cases of test/acceptance/29-account-layer.test.ts that call the TypeScript implementation
// directly: unit tables of the account model and the declaration rules, and the cases that read
// the resolved config in process. Each keeps its name and body; the CLI side of each is in the
// acceptance file. Deleted with src/ at the cutover.

function declare(ws: Workspace, layer: string, yaml: string): void {
  appendFileSync(`${ws.root}/${layer}/layer.yaml`, `account:${yaml}\n`);
}

/** The value as YAML: JSON is YAML, so a string or a map is written exactly as given. */
function value(raw: unknown): string {
  return ` ${JSON.stringify(raw)}`;
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

describe("§13.24 account layer field: schema (§7.13 'Account names')", () => {
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
