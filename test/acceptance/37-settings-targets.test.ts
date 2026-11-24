import { mkdirSync, writeFileSync } from "node:fs";
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
      "codex: { features: { other: true }, other: { work: { model_reasoning_effort: high } } }\n",
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

// §7.9 the keys with a security meaning: reserved keys are refused wherever the table of native
// keys says, floor keys are read, and an agent's native fields bring no servers or hooks (§7.2).

const RESERVED_ROWS = [
  "claude: { apiKeyHelper: ./login.sh }\n",
  "claude: { statusLine: { type: command, command: ./status.sh } }\n",
  "claude: { fileSuggestion: { type: command, command: ./suggest.sh } }\n",
  "claude: { sandbox: { credentials: { helper: ./cred.sh } } }\n",
  "claude: { sandbox: { network: { httpProxyPort: 8080 } } }\n",
  "claude: { sandbox: { network: { socksProxyPort: 1080 } } }\n",
  "claude: { sandbox: { filesystem: { disabled: true } } }\n",
  "claude: { sandbox: { allowAppleEvents: true } }\n",
  "claude: { autoMode: { allow: [x] } }\n",
  "claude: { skipAutoPermissionPrompt: true }\n",
  "claude: { skipDangerousModePermissionPrompt: true }\n",
  "claude: { allowManagedHooksOnly: true }\n",
  "claude: { allowManagedMcpServersOnly: true }\n",
  "claude: { managedMcpServers: [x] }\n",
  "claude: { allowAllClaudeAiMcps: true }\n",
  "claude: { enabledPlugins: { 'a@b': true } }\n",
  "claude: { extraKnownMarketplaces: { b: { source: x } } }\n",
  "claude: { prependPlugins: [x] }\n",
  "claude: { pluginConfigs: {} }\n",
  "claude: { strictKnownMarketplaces: [] }\n",
  "claude: { syncClaudeAiPlugins: true }\n",
  "codex: { auto_review: { policy: x } }\n",
  "codex: { default_permissions: workspace }\n",
  "codex: { permissions: { workspace: { filesystem: {} } } }\n",
  "codex: { projects: { '/work/shop': { trust_level: trusted } } }\n",
  "codex: { plugins: { a: { enabled: true } } }\n",
  "codex: { marketplaces: { a: { source: x } } }\n",
  "codex: { apps: { a: { default_tools_approval_mode: auto } } }\n",
  "codex: { windows: { sandbox: unelevated } }\n",
  "codex: { profiles: { work: { model_provider: other } } }\n",
  "claude: { awsAuthRefresh: ./refresh.sh }\n",
  "claude: { awsCredentialExport: ./export.sh }\n",
  "claude: { gcpAuthRefresh: ./refresh.sh }\n",
  "claude: { otelHeadersHelper: ./headers.sh }\n",
  "claude: { sandbox: { filesystem: { allowManagedReadPathsOnly: true } } }\n",
  "claude: { sandbox: { ripgrep: { command: rg } } }\n",
  "claude: { sandbox: { bwrapPath: /usr/bin/bwrap } }\n",
  "claude: { sandbox: { socatPath: /usr/bin/socat } }\n",
  "claude: { sandbox: { network: { tlsTerminate: true } } }\n",
  "claude: { processWrapper: ./wrap.sh }\n",
  "claude: { policyHelper: ./policy.sh }\n",
  "claude: { sshHostCommands: [x] }\n",
  "claude: { forceLoginGatewayUrl: 'https://example.invalid' }\n",
  "claude: { allowManagedPermissionRulesOnly: true }\n",
  "claude: { disableSideloadFlags: true }\n",
  "claude: { skipWebFetchPreflight: true }\n",
  "claude: { forceLoginMethod: console }\n",
  "claude: { forceLoginOrgUUID: 00000000-0000-0000-0000-000000000000 }\n",
  "codex: { notify: [./notify.sh] }\n",
  "codex: { model_provider: other }\n",
  "codex: { model_providers: { other: { base_url: 'https://example.invalid' } } }\n",
  "codex: { openai_base_url: 'https://example.invalid' }\n",
  "codex: { chatgpt_base_url: 'https://example.invalid' }\n",
  "codex: { otel: { exporter: none } }\n",
];

const FLOOR_ROWS = [
  "claude: { sandbox: { enabled: true, failIfUnavailable: true, autoAllowBashIfSandboxed: false, allowUnsandboxedCommands: false, excludedCommands: [git], filesystem: { allowWrite: [.], allowRead: [.], denyWrite: [.env], denyRead: [.env] }, ignoreViolations: { '*': [/tmp] }, enableWeakerNestedSandbox: false, enableWeakerNetworkIsolation: false, network: { allowedDomains: [example.invalid], allowUnixSockets: [], allowMachLookup: [], allowAllUnixSockets: false, allowLocalBinding: false, deniedDomains: [bad.invalid] } } }\n",
  "claude: { disableAutoMode: disable, allowedHttpHookUrls: [], httpHookAllowedEnvVars: [], disableSkillShellExecution: true, allowedMcpServers: [], deniedMcpServers: [], disableClaudeAiConnectors: true }\n",
  "codex: { sandbox_workspace_write: { network_access: false, writable_roots: [], exclude_tmpdir_env_var: true, exclude_slash_tmp: true }, approvals_reviewer: user, allow_login_shell: false, web_search: disabled, tools: { web_search: { context_size: low, allowed_domains: [] } } }\n",
];

describe("§13.29 settings.yaml keys with a security meaning", () => {
  it.each(FLOOR_ROWS)("floor keys are read and lint clean: %j", async (text) => {
    const ws = fixture("37-settings-targets");
    plant(ws, SETTINGS, text);
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  it("alz lint --layer refuses claude mcpServers in settings.yaml and names the file", async () => {
    const ws = fixture("37-settings-targets");
    plant(ws, SETTINGS, "claude: { mcpServers: { s: { command: server-bin } } }\n");
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [settings] personal:settings.yaml");
  });

  describe.each(RESERVED_ROWS)("file %j", (text) => {
    referenceDrift(
      "TASK-55.5.16",
      "alz lint --layer refuses the reserved key and names the file",
      async () => {
        const ws = fixture("37-settings-targets");
        plant(ws, SETTINGS, text);
        const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [settings] personal:settings.yaml");
      },
    );
  });
});

const HELPER_HEAD = "---\nname: helper\ndescription: A helper.\n";

const AGENT_ROWS = [
  "claude:\n  mcpServers:\n    s: { command: server-bin }\n",
  "claude:\n  hooks:\n    Stop: []\n",
  "codex:\n  mcp_servers:\n    s: { command: server-bin }\n",
  "codex:\n  hooks: []\n",
];

function plantAgent(ws: Workspace, native: string): void {
  mkdirSync(`${ws.root}/personal/agents`, { recursive: true });
  writeFileSync(`${ws.root}/personal/agents/helper.md`, `${HELPER_HEAD}${native}---\nBody\n`);
}

describe("§13.29 an agent's native fields bring no servers or hooks (§7.2)", () => {
  it("an agent with other native fields lints clean", async () => {
    const ws = fixture("37-settings-targets");
    plantAgent(ws, "claude:\n  color: blue\n");
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  describe.each(AGENT_ROWS)("native fields %j", (native) => {
    referenceDrift(
      "TASK-55.5.16",
      "alz lint --layer refuses the key and names the agent file",
      async () => {
        const ws = fixture("37-settings-targets");
        plantAgent(ws, native);
        const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [agent] personal:agents/helper.md");
      },
    );
  });
});

describe("§13.29 permissions.yaml floor fields (§7.6)", () => {
  referenceDrift("TASK-55.5.16", "the three flags are read and lint clean", async () => {
    const ws = fixture("37-settings-targets");
    writeFileSync(
      `${ws.root}/personal/permissions.yaml`,
      "disable_bypass_mode: true\nblock_reads_outside: true\nblock_writes_outside: false\n",
    );
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  it("a flag that is not a boolean is refused naming the file", async () => {
    const ws = fixture("37-settings-targets");
    writeFileSync(`${ws.root}/personal/permissions.yaml`, "disable_bypass_mode: sometimes\n");
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [permissions] personal:permissions.yaml");
  });
});

// §7.9 and §7.2: under `claude:`, settings.yaml and the agent files take only the keys that
// docs/target-facts.md records with a type, and check the type of each value (TASK-79).

const UNLISTED_SETTINGS_ROWS = [
  "claude: { notAKeyOfClaudeCode: true }\n",
  "claude: { sandbox: { notAKeyOfClaudeCode: true } }\n",
  "claude: { sandbox: { network: { notAKeyOfClaudeCode: [] } } }\n",
  "claude: { spinnerVerbs: { verbs: [a], notAKeyOfClaudeCode: 1 } }\n",
];

const WRONG_TYPE_SETTINGS_ROWS = [
  "claude: { verbose: secret-value }\n",
  "claude: { language: [secret-value] }\n",
  "claude: { cleanupPeriodDays: secret-value }\n",
  "claude: { editorMode: secret-value }\n",
  "claude: { availableModels: secret-value }\n",
  "claude: { env: { FOO: 1 } }\n",
  "claude: { sandbox: { enabled: secret-value } }\n",
  "claude: { sandbox: { network: { allowedDomains: secret-value } } }\n",
  "claude: { allowedMcpServers: [secret-value] }\n",
];

describe("§7.9 the keys and value types of Claude settings", () => {
  it("settings of the documented types lint clean", async () => {
    const ws = fixture("37-settings-targets");
    plant(
      ws,
      SETTINGS,
      "claude: { verbose: true, language: german, cleanupPeriodDays: 30, editorMode: vim, availableModels: [a], env: { FOO: x }, sandbox: { enabled: true, network: { allowedDomains: [example.invalid] } }, allowedMcpServers: [{ serverName: a }] }\n",
    );
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  describe.each(UNLISTED_SETTINGS_ROWS)("file %j", (text) => {
    referenceDrift(
      "TASK-79",
      "alz lint --layer refuses a key that Claude Code does not document and names the file",
      async () => {
        const ws = fixture("37-settings-targets");
        plant(ws, SETTINGS, text);
        const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [settings] personal:settings.yaml");
        expect(result.stderr).toContain("notAKeyOfClaudeCode");
      },
    );
  });

  describe.each(WRONG_TYPE_SETTINGS_ROWS)("file %j", (text) => {
    referenceDrift(
      "TASK-79",
      "alz lint --layer refuses a value of the wrong type, names the file and prints no value",
      async () => {
        const ws = fixture("37-settings-targets");
        plant(ws, SETTINGS, text);
        const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [settings] personal:settings.yaml");
        expect(result.stderr).not.toContain("secret-value");
      },
    );
  });
});

const UNLISTED_AGENT_ROWS = ["claude:\n  notAFieldOfClaudeCode: true\n"];

const WRONG_TYPE_AGENT_ROWS = [
  "claude:\n  maxTurns: secret-value\n",
  "claude:\n  background: secret-value\n",
  "claude:\n  color: secret-value\n",
  "claude:\n  initialPrompt: [secret-value]\n",
];

describe("§7.2 the fields and value types of an agent's Claude fields", () => {
  it("an agent with fields of the documented types lints clean", async () => {
    const ws = fixture("37-settings-targets");
    plantAgent(
      ws,
      "claude:\n  maxTurns: 5\n  background: true\n  color: blue\n  effort: high\n  initialPrompt: Start\n  disallowedTools: [Bash]\n",
    );
    const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  describe.each(UNLISTED_AGENT_ROWS)("native fields %j", (native) => {
    referenceDrift(
      "TASK-79",
      "alz lint --layer refuses a field that Claude Code does not document and names the agent file",
      async () => {
        const ws = fixture("37-settings-targets");
        plantAgent(ws, native);
        const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [agent] personal:agents/helper.md");
      },
    );
  });

  describe.each(WRONG_TYPE_AGENT_ROWS)("native fields %j", (native) => {
    referenceDrift(
      "TASK-79",
      "alz lint --layer refuses a value of the wrong type, names the agent file and prints no value",
      async () => {
        const ws = fixture("37-settings-targets");
        plantAgent(ws, native);
        const result = await layers(ws, ".", ["lint", "--layer", "personal"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [agent] personal:agents/helper.md");
        expect(result.stderr).not.toContain("secret-value");
      },
    );
  });
});
