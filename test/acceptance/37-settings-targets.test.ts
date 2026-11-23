import { writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { fixture, layers, readTree, type Workspace } from "../helpers";

// §13.29: a settings or MCP file whose top-level key is not a known target (settings) or not a
// server id (mcp) is refused, whatever the key (§7.7, §7.9).

const SETTINGS_KEYS = [
  "gemini",
  "Claude",
  "CODEX",
  "claude ",
  "env",
  "permissions",
  "mcpServers",
  "hooks",
  "model",
  "__proto__",
  "1",
  "",
  "ünï",
];

const MCP_KEYS = [
  "has space",
  "-leading",
  "_leading",
  "dot.id",
  "a/b",
  "é",
  "a".repeat(65),
  "",
  "1 2",
];

const SERVER = "{ transport: stdio, command: server-bin }";

interface Kind {
  file: string;
  code: string;
  /** The text of the file whose only top-level key is `key`. */
  withKey: (key: string) => string;
  valid: string;
}

const SETTINGS: Kind = {
  file: "settings.yaml",
  code: "settings",
  withKey: (key) => `${JSON.stringify(key)}: {}\n`,
  valid: "claude: {}\ncodex: {}\n",
};

const MCP: Kind = {
  file: "mcp.yaml",
  code: "mcp",
  withKey: (key) => `${JSON.stringify(key)}: ${SERVER}\n`,
  valid: `playwright: ${SERVER}\n`,
};

function plant(ws: Workspace, kind: Kind, text: string): void {
  writeFileSync(`${ws.root}/personal/${kind.file}`, text);
}

const SUITES: [string, Kind, string[]][] = [
  ["settings.yaml", SETTINGS, SETTINGS_KEYS],
  ["mcp.yaml", MCP, MCP_KEYS],
];

describe.each(SUITES)("§13.29 %s top-level keys", (_name, kind, keys) => {
  it("a file with only valid keys lints clean", async () => {
    const ws = fixture("37-settings-targets");
    plant(ws, kind, kind.valid);
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  describe.each(keys)("key %j", (key) => {
    referenceDrift("TASK-55.5", "alz lint --layer refuses the file and names it", async () => {
      const ws = fixture("37-settings-targets");
      plant(ws, kind, kind.withKey(key));
      const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(`error [${kind.code}] personal:${kind.file}`);
    });

    // alz build exists in the Rust alz from R5 (TASK-55.6); until then this case fails against it.
    referenceDrift("TASK-55.5", "alz build refuses the file and writes nothing", async () => {
      const ws = fixture("37-settings-targets");
      plant(ws, kind, kind.withKey(key));
      const tree = readTree(`${ws.root}/repo`);
      const result = await layers(ws, "repo", ["build"]);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain(`error [${kind.code}] personal:${kind.file}`);
      expect(readTree(`${ws.root}/repo`)).toEqual(tree);
    });
  });
});

const CODEX_ROWS = [
  "codex: { features: { hooks: false } }\n",
  "codex: { features: { codex_hooks: false } }\n",
  "codex: { profile: work }\n",
  "codex: { profiles: { work: { approval_policy: never } } }\n",
  "codex: { profiles: { work: { sandbox_mode: danger-full-access } } }\n",
  "codex: { profiles: { work: { features: { hooks: false } } } }\n",
  "codex: { other: [{ features: { codex_hooks: false } }] }\n",
];

describe("§13.29 settings.yaml codex keys that duplicate a first-class item", () => {
  it("a file with only other nested keys lints clean", async () => {
    const ws = fixture("37-settings-targets");
    plant(
      ws,
      SETTINGS,
      "codex: { features: { other: true }, profiles: { work: { model_reasoning_effort: high } } }\n",
    );
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  describe.each(CODEX_ROWS)("file %j", (text) => {
    referenceDrift("TASK-55.5", "alz lint --layer refuses the file and names it", async () => {
      const ws = fixture("37-settings-targets");
      plant(ws, SETTINGS, text);
      const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("error [settings] personal:settings.yaml");
    });
  });
});
