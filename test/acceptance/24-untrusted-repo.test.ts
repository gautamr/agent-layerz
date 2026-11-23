import { appendFileSync, cpSync, existsSync, mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { describe, expect, it } from "vitest";
import {
  accountFolder,
  alz,
  built,
  hubEntry,
  plain,
  remoteStack,
  writeAllowed,
} from "../account-workspace";
import { referenceDrift } from "../drift";
import { fixture, layers, readFile, readTree, type Workspace, writeLock } from "../helpers";

// A cloned repo is untrusted input: its manifest and lock must not reach outside it.
describe("untrusted repo content", () => {
  it("refuses a lock that lists a path outside the repo, and deletes nothing", async () => {
    const ws = fixture("14-non-destructive");
    expect((await layers(ws, "repo", ["build", "--adopt"])).code).toBe(0);
    mkdirSync(`${ws.root}/victim`);
    writeFileSync(`${ws.root}/victim/important.txt`, "keep me");
    const lock = JSON.parse(readFile(ws, "repo/layer.lock"));
    lock.files["../victim/important.txt"] = "sha256:00";
    writeFileSync(`${ws.root}/repo/layer.lock`, JSON.stringify(lock));

    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("unsafe entry '../victim/important.txt'");
    expect(existsSync(`${ws.root}/victim/important.txt`)).toBe(true);
  });

  it("refuses a lock whose layer sha is not a commit id", async () => {
    const ws = fixture("14-non-destructive");
    expect((await layers(ws, "repo", ["build", "--adopt"])).code).toBe(0);
    const lock = JSON.parse(readFile(ws, "repo/layer.lock"));
    lock.layers[0].sha = "../../../etc";
    writeFileSync(`${ws.root}/repo/layer.lock`, JSON.stringify(lock));
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("unsafe entry '../../../etc'");
  });

  it.each([
    ["git:--upload-pack=touch PWNED@v1", "cannot start with '-'"],
    ["github:owner/repo@--upload-pack=touch", "is not a valid git ref"],
  ])("never passes %s to git as an option", async (source, message) => {
    const ws = fixture("14-non-destructive");
    writeFileSync(
      `${ws.root}/repo/layer.yaml`,
      `stack:\n  - source: ${source}\n  - source: path:./.layer\n`,
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(message);
    expect(existsSync(`${ws.root}/repo/PWNED`)).toBe(false);
  });

  it("rejects a manifest ref that git would read as an option", async () => {
    const ws = fixture("14-non-destructive");
    writeFileSync(
      `${ws.root}/repo/layer.yaml`,
      "stack:\n  - source: path:../personal\n    ref: --output=/tmp/x\n  - source: path:./.layer\n",
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("is not a valid git ref");
  });
});

// §13.28: the cases above that need only a verdict, through `alz lint`.
describe("untrusted repo content, through alz lint", () => {
  it("refuses a lock whose layer sha is not a commit id", async () => {
    const ws = fixture("14-non-destructive");
    writeLock(ws, [
      {
        name: "personal",
        source: "path:../personal",
        sha: "../../../etc",
        dir: `${ws.root}/personal`,
      },
    ]);
    const result = await layers(ws, "repo", ["lint"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/\[lock\]/);
    expect(result.stdout).not.toContain("lint ok");
  });

  it.each([
    ["git:--upload-pack=touch PWNED@v1", /cannot start with '-'|does not start with '-'/],
    ["github:owner/repo@--upload-pack=touch", /is not a valid git ref|must be a git ref/],
  ])("never passes %s to git as an option", async (source, message) => {
    const ws = fixture("14-non-destructive");
    writeFileSync(
      `${ws.root}/repo/layer.yaml`,
      `stack:\n  - source: ${source}\n  - source: path:./.layer\n`,
    );
    const result = await layers(ws, "repo", ["lint"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(message);
    expect(existsSync(`${ws.root}/repo/PWNED`)).toBe(false);
  });

  it("rejects a manifest ref that git would read as an option", async () => {
    const ws = fixture("14-non-destructive");
    writeFileSync(
      `${ws.root}/repo/layer.yaml`,
      "stack:\n  - source: path:../personal\n    ref: --output=/tmp/x\n  - source: path:./.layer\n",
    );
    const result = await layers(ws, "repo", ["lint"]);
    expect(result.code).toBe(1);
    // The spec fixes the code and the field, not the sentence (§10.1): both spellings are accepted
    // until cutover, and the Rust wording is pinned by its unit tests.
    expect(result.stderr).toMatch(/is not a valid git ref|stack\[\d+\]\.ref must be a git ref/);
    expect(result.stdout).not.toContain("lint ok");
  });
});

// §5.2 "Credentials in a source" (TASK-71): a git: URL with credentials is refused at the
// manifest, before any cache or git access, and the token is printed and written nowhere.
const TOKEN = "tok3n-s3cret";
const WITH_CREDENTIALS: [string, string][] = [
  ["a user and a token in an https URL", `git:https://user:${TOKEN}@host.invalid/x.git@v1`],
  ["a token as the user of an https URL", `git:https://${TOKEN}@host.invalid/x.git@v1`],
  ["a password in an ssh URL", `git:ssh://git:${TOKEN}@host.invalid/x.git@v1`],
  ["a token in an https URL without a ref", `git:https://${TOKEN}@host.invalid/x.git`],
  ["a password in a scp-like address without a ref", `git:me:${TOKEN}@host.invalid/o/r`],
  ["a password in a scp-like address before a ref", `git:user:${TOKEN}@host.invalid:repo@v1`],
];
const CREDENTIALS_MESSAGE =
  "a git: source in layer.yaml holds credentials in its URL; remove them and use a git credential helper";

function credentialStack(ws: Workspace, source: string): void {
  writeFileSync(
    `${ws.root}/repo/layer.yaml`,
    `stack:\n  - source: ${source}\n  - source: path:./.layer\n`,
  );
}

describe("a git: source with credentials (TASK-71)", () => {
  describe.each([
    ["lint", ["lint"]],
    // alz build exists in the Rust alz from R5 (TASK-55.6); until then this form fails against it.
    ["build", ["build"]],
  ])("through alz %s", (_command, args) => {
    describe.each(WITH_CREDENTIALS)("%s", (_label, source) => {
      referenceDrift("TASK-71", "is refused, and no token is printed or written", async () => {
        const ws = fixture("14-non-destructive");
        credentialStack(ws, source);
        const files = readTree(ws.root);
        const result = await layers(ws, "repo", args);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain(`error [source] ${CREDENTIALS_MESSAGE}`);
        expect(result.stdout + result.stderr).not.toContain(TOKEN);
        // Nothing is written, so no file but the manifest we wrote holds the token.
        expect(readTree(ws.root)).toEqual(files);
        expect(readTree(ws.home)).toEqual({});
      });
    });
  });

  it.each([
    ["a user name in an ssh URL", "git:ssh://git@host.invalid/x.git@v1"],
    ["a user name in a scp-like address", "git:git@host.invalid:o/x.git@v1"],
    ["a user name and a ref in a scp-like address", "git:git@host.invalid:repo.git@v1"],
    ["a colon and a ref without a user in a scp-like address", "git:host.invalid:client.git@v1"],
  ])("does not refuse %s for credentials", async (_label, source) => {
    const ws = fixture("14-non-destructive");
    credentialStack(ws, source);
    // The host does not exist: a fake ssh that ends at once keeps the case off the network.
    const result = await layers(ws, "repo", ["lint"], { GIT_SSH: "/usr/bin/false" });
    expect(result.stderr).not.toContain("holds credentials");
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(`cannot fetch '${source}'`);
  });
});

// §5.2 "Transports" (TASK-55.4): a git: URL that names a transport helper, `<helper>::<address>`,
// is refused at the manifest. git would run `git-remote-<helper>`, and `ext` runs a command.
describe("a git: source that names a transport helper", () => {
  describe.each([
    ["lint", ["lint"]],
    // alz build exists in the Rust alz from R5 (TASK-55.6); until then this form fails against it.
    ["build", ["build"]],
  ])("through alz %s", (_command, args) => {
    describe.each([
      ["ext", "git:ext::sh -c touch% PWNED@v1"],
      ["an empty helper", "git:::x@v1"],
      ["a helper in front of https", `git:https::https://${TOKEN}@host.invalid/x.git@v1`],
    ])("%s", (_label, source) => {
      referenceDrift("TASK-55.4", "is refused, and nothing is run or written", async () => {
        const ws = fixture("14-non-destructive");
        writeFileSync(
          `${ws.root}/repo/layer.yaml`,
          `stack:\n  - source: "${source}"\n  - source: path:./.layer\n`,
        );
        const files = readTree(ws.root);
        const result = await layers(ws, "repo", args);
        expect(result.code).toBe(1);
        expect(result.stderr).toMatch(/error \[manifest\].*transport helper/);
        expect(existsSync(`${ws.root}/repo/PWNED`)).toBe(false);
        expect(result.stdout + result.stderr).not.toContain(TOKEN);
        expect(readTree(ws.root)).toEqual(files);
      });
    });
  });
});

const PROTECTED_VARIABLES = [
  "CLAUDE_CONFIG_DIR",
  "CODEX_HOME",
  "HOME",
  "XDG_CONFIG_HOME",
  "XDG_STATE_HOME",
  "XDG_CACHE_HOME",
];

function put(ws: Workspace, path: string, text: string): void {
  mkdirSync(dirname(`${ws.root}/${path}`), { recursive: true });
  writeFileSync(`${ws.root}/${path}`, text);
}

function writeStack(ws: Workspace, sources: string[]): void {
  const entries = sources.map((source) => `  - source: ${source}\n`).join("");
  put(ws, "repo/layer.yaml", `stack:\n${entries}targets: [claude]\n`);
}

const ACCOUNT_VALUE = "secret-account-name";

/** A build that must fail with every message, name nothing it must not, and write nothing. */
async function expectRefused(ws: Workspace, ...messages: string[]): Promise<void> {
  const tree = readTree(`${ws.root}/repo`, ["ext"]);
  const result = await layers(ws, "repo", ["build"]);
  expect(result.code).toBe(1);
  for (const message of messages) expect(result.stderr).toContain(message);
  expect(result.stderr).not.toContain(ACCOUNT_VALUE);
  expect(readTree(`${ws.root}/repo`, ["ext"])).toEqual(tree);
}

// §7.13 'Who may declare it': the repository chooses the stack, so it cannot choose an account.
describe("an account declaration by a layer that may not make one", () => {
  const account = `account: ${ACCOUNT_VALUE}\n`;

  it.each(["repo", "client"])(
    "fails the build when the repo layer (kind: %s) declares it",
    async (kind) => {
      const ws = fixture("29-account-layer");
      put(
        ws,
        "repo/.layer/layer.yaml",
        `name: repo\nkind: ${kind}\nvisibility: private\n${account}`,
      );
      await expectRefused(
        ws,
        "error [account] repo:layer.yaml",
        "cannot declare account",
        "inside the repository",
      );
    },
  );

  it("fails the build when a group layer declares it", async () => {
    const ws = fixture("29-account-layer");
    appendFileSync(`${ws.root}/group/layer.yaml`, account);
    writeStack(ws, ["path:../personal", "path:../client", "path:../group", "path:./.layer"]);
    await expectRefused(ws, "group:layer.yaml", "its kind is 'group', not 'client'");
  });

  it("fails the build when a public layer that calls itself client declares it", async () => {
    const ws = fixture("29-account-layer");
    appendFileSync(`${ws.root}/public-client/layer.yaml`, account);
    writeStack(ws, ["path:../personal", "path:../public-client", "path:./.layer"]);
    await expectRefused(ws, "public-client:layer.yaml", "it is public");
  });

  it("fails the build when a second client layer declares it, and names both", async () => {
    const ws = fixture("29-account-layer");
    appendFileSync(`${ws.root}/client/layer.yaml`, account);
    appendFileSync(`${ws.root}/client-b/layer.yaml`, account);
    writeStack(ws, ["path:../personal", "path:../client", "path:../client-b", "path:./.layer"]);
    await expectRefused(ws, "layers 'client' and 'client-b' both declare account");
  });

  it("fails the build when a path: link inside the repository points at a client layer outside it", async () => {
    const ws = fixture("29-account-layer");
    appendFileSync(`${ws.root}/client/layer.yaml`, account);
    symlinkSync(`${ws.root}/client`, `${ws.root}/repo/ext`);
    writeStack(ws, ["path:../personal", "path:./ext", "path:./.layer"]);
    await expectRefused(ws, "client:layer.yaml", "inside the repository");
  });

  it("builds when the first layer and one client layer outside the repository declare it", async () => {
    const ws = fixture("29-account-layer");
    appendFileSync(`${ws.root}/personal/layer.yaml`, account);
    appendFileSync(`${ws.root}/client/layer.yaml`, account);
    expect((await layers(ws, "repo", ["build"])).code).toBe(0);
  });
});

type Form = { label: string; file: string; text: (name: string) => string };

const yamlIn = (file: string, body: (name: string) => string) => ({ file, text: body });

// §7.9: no layer may set a variable that chooses an account folder or locates alz's files.
const FORMS: Form[] = [
  { label: "claude env", ...yamlIn("settings.yaml", (v) => `claude:\n  env:\n    ${v}: x\n`) },
  {
    label: "codex set",
    ...yamlIn(
      "settings.yaml",
      (v) => `codex:\n  shell_environment_policy:\n    set:\n      ${v}: x\n`,
    ),
  },
  {
    label: "codex set, lower case",
    ...yamlIn(
      "settings.yaml",
      (v) => `codex:\n  shell_environment_policy:\n    set:\n      ${v.toLowerCase()}: x\n`,
    ),
  },
  {
    label: "codex exclude, exact",
    ...yamlIn("settings.yaml", (v) => `codex:\n  shell_environment_policy:\n    exclude: [${v}]\n`),
  },
  {
    label: "codex exclude, one-character wildcard",
    ...yamlIn(
      "settings.yaml",
      (v) => `codex:\n  shell_environment_policy:\n    exclude: ["?${v.slice(1)}"]\n`,
    ),
  },
  {
    label: "codex exclude, run wildcard and lower case",
    ...yamlIn(
      "settings.yaml",
      (v) => `codex:\n  shell_environment_policy:\n    exclude: ["*${v.slice(2).toLowerCase()}"]\n`,
    ),
  },
  {
    label: "codex exclude *home* style",
    ...yamlIn(
      "settings.yaml",
      (v) =>
        `codex:\n  shell_environment_policy:\n    exclude: ["*${v.slice(-3).toLowerCase()}*"]\n`,
    ),
  },
  {
    label: "codex filters entry set to exclude",
    ...yamlIn(
      "settings.yaml",
      (v) => `codex:\n  shell_environment_policy:\n    filters:\n      "${v}": exclude\n`,
    ),
  },
  {
    label: "codex policy under a profile",
    ...yamlIn(
      "settings.yaml",
      (v) =>
        `codex:\n  profiles:\n    work:\n      shell_environment_policy:\n        set:\n          ${v}: x\n`,
    ),
  },
  {
    label: "mcp env_from",
    ...yamlIn(
      "mcp.yaml",
      (v) => `srv:\n  transport: stdio\n  command: run-it\n  env_from: [${v}]\n`,
    ),
  },
  {
    label: "passthrough settings.json",
    file: "targets/claude/.claude/settings.json",
    text: (v) => `{"env": {"${v}": "x"}}\n`,
  },
  {
    label: "passthrough settings.local.json",
    file: "targets/claude/.claude/settings.local.json",
    text: (v) => `{"env": {"${v}": "x"}}\n`,
  },
  {
    label: "passthrough .mcp.json",
    file: "targets/claude/.mcp.json",
    text: (v) => `{"mcpServers": {"s": {"env": {"${v}": "x"}}}}\n`,
  },
  {
    label: "passthrough with a case-variant path",
    file: "targets/claude/.Claude/Settings.JSON",
    text: (v) => `{"env": {"${v}": "x"}}\n`,
  },
  {
    label: "passthrough config.toml, policy",
    file: "targets/codex/.codex/config.toml",
    text: (v) => `[shell_environment_policy.set]\n${v} = "x"\n`,
  },
  {
    label: "passthrough config.toml, MCP environment",
    file: "targets/codex/.codex/config.toml",
    text: (v) => `[mcp_servers.s.env]\n${v} = "x"\n`,
  },
  {
    label: "passthrough config.toml, case-variant path",
    file: "targets/codex/.Codex/Config.toml",
    text: (v) => `[mcp_servers.s.env]\n${v.toLowerCase()} = "x"\n`,
  },
  {
    label: "passthrough settings.json at any depth",
    file: "targets/claude/sub/.claude/settings.json",
    text: (v) => `{"env": {"${v}": "x"}}\n`,
  },
  {
    label: "passthrough .mcp.json at any depth",
    file: "targets/claude/sub/dir/.mcp.json",
    text: (v) => `{"mcpServers": {"s": {"env": {"${v}": "x"}}}}\n`,
  },
  {
    label: "passthrough plugin manifest",
    file: "targets/claude/plugin/.claude-plugin/plugin.json",
    text: (v) => `{"name": "p", "mcpServers": {"s": {"command": "x", "env": {"${v}": "x"}}}}\n`,
  },
  {
    label: "passthrough plugin file with a nested env",
    file: "targets/claude/plugin/.claude-plugin/marketplace.json",
    text: (v) => `{"plugins": [{"name": "p", "mcpServers": {"s": {"env": {"${v}": "x"}}}}]}\n`,
  },
  {
    label: "passthrough Codex role file",
    file: "targets/codex/.codex/agents/worker.toml",
    text: (v) => `[mcp_servers.s.env]\n${v} = "x"\n`,
  },
  {
    label: "passthrough .codex/config.toml under the claude target",
    file: "targets/claude/.codex/config.toml",
    text: (v) => `[shell_environment_policy.set]\n${v} = "x"\n`,
  },
  {
    label: "passthrough toml of any name under the claude target",
    file: "targets/claude/deep/worker.toml",
    text: (v) => `[mcp_servers.s.env]\n${v} = "x"\n`,
  },
  {
    label: "passthrough .claude/agents file under the codex target",
    file: "targets/codex/.claude/agents/helper.md",
    text: (v) => `---\nname: helper\nmcpServers:\n  s:\n    env:\n      ${v}: x\n---\nBody\n`,
  },
  {
    label: "passthrough .claude/agents file at any depth",
    file: "targets/claude/.claude/agents/team/helper.md",
    text: (v) => `---\nname: helper\nmcpServers:\n  s:\n    env:\n      ${v}: x\n---\nBody\n`,
  },
  {
    label: "passthrough Codex toml outside .codex",
    file: "targets/codex/worker.toml",
    text: (v) => `[mcp_servers.s.env]\n${v} = "x"\n`,
  },
  {
    label: "passthrough Claude agent, mcpServers map",
    file: "targets/claude/.claude/agents/helper.md",
    text: (v) => `---\nname: helper\nmcpServers:\n  s:\n    env:\n      ${v}: x\n---\nBody\n`,
  },
  {
    label: "agent frontmatter, claude.mcpServers map",
    file: "agents/helper.md",
    text: (v) =>
      `---\nname: helper\ndescription: A helper.\nclaude:\n  mcpServers:\n    s:\n      env:\n        ${v}: x\n---\nBody\n`,
  },
  {
    label: "agent frontmatter, claude.mcpServers list",
    file: "agents/helper.md",
    text: (v) =>
      `---\nname: helper\ndescription: A helper.\nclaude:\n  mcpServers:\n    - plain-name\n    - s:\n        env:\n          ${v}: x\n---\nBody\n`,
  },
  {
    label: "agent frontmatter, codex policy",
    file: "agents/helper.md",
    text: (v) =>
      `---\nname: helper\ndescription: A helper.\ncodex:\n  shell_environment_policy:\n    set:\n      ${v}: x\n---\nBody\n`,
  },
];

describe("a layer that sets an account or location variable", () => {
  const cases = FORMS.flatMap((form) =>
    PROTECTED_VARIABLES.map((name) => [form.label, form, name] as const),
  );

  it.each(cases)("fails the build: %s, %s", async (_label, form, name) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${form.file}`, form.text(name));
    const tree = readTree(`${ws.root}/repo`);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(`error [protected-variable] personal:${form.file}`);
    expect(result.stderr).toContain(`names ${name};`);
    expect(readTree(`${ws.root}/repo`)).toEqual(tree);
  });

  it("names the key path and the variable, never a lower-case spelling the layer wrote", async () => {
    const ws = fixture("29-account-layer");
    put(
      ws,
      "personal/settings.yaml",
      "codex:\n  shell_environment_policy:\n    set:\n      codex_home: x\n",
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.stderr).toContain("codex.shell_environment_policy.set names CODEX_HOME");
    expect(result.stderr).not.toContain("codex_home");
  });

  it("lint of a single layer reports it too", async () => {
    const ws = fixture("29-account-layer");
    put(ws, "personal/settings.yaml", "claude:\n  env:\n    HOME: x\n");
    const result = await layers(ws, "repo", ["lint", "--layer", "../personal"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [protected-variable] personal:settings.yaml");
  });

  it.each([
    [
      "a JSON passthrough settings file that does not parse",
      "targets/claude/.claude/settings.json",
      "not json",
    ],
    [
      "a TOML escape that could hide a name",
      "targets/codex/.codex/config.toml",
      '"\\u0048OME" = "x"\n',
    ],
    [
      "a policy pattern that is not a string",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    exclude: [5]\n",
    ],
    [
      "a policy exclude that is not a list",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    exclude: HOME\n",
    ],
    [
      "a filters value that is a list",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    filters: [HOME]\n",
    ],
    ["a claude env that is a list", "settings.yaml", "claude:\n  env: [HOME]\n"],
    ["a claude env that is a string", "settings.yaml", "claude:\n  env: HOME\n"],
    [
      "a policy set that is a list",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    set: [HOME]\n",
    ],
    [
      "a passthrough settings env that is a list",
      "targets/claude/.claude/settings.json",
      '{"env": ["HOME"]}\n',
    ],
    ["a TOML \\U escape", "targets/codex/.codex/config.toml", '"\\U00000048OME" = "x"\n'],
    ["a TOML \\x escape", "targets/codex/.codex/config.toml", '"\\x48OME" = "x"\n'],
    [
      "a TOML line-ending backslash that joins a name",
      "targets/codex/.codex/config.toml",
      'x = """CODEX_\\\nHOME"""\n',
    ],
    [
      "a Claude agent file whose frontmatter does not parse",
      "targets/claude/.claude/agents/helper.md",
      "---\nname: [\n---\nBody\n",
    ],
  ])("refuses %s, because it cannot be checked", async (_label, file, text) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${file}`, text);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [protected-variable]");
    expect(result.stderr).toContain("cannot be checked");
  });

  it.each([
    [
      "an exclude pattern that matches none of them",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    exclude: ['*FOO*']\n",
    ],
    [
      "include_only",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    include_only: [HOME, PATH]\n",
    ],
    ["inherit: core", "settings.yaml", "codex:\n  shell_environment_policy:\n    inherit: core\n"],
    [
      "a filters entry set to include",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    filters:\n      HOME: include\n",
    ],
    [
      "a set of another variable",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    set:\n      FOO: x\n",
    ],
    ["a claude env of another variable", "settings.yaml", "claude:\n  env:\n    FOO: x\n"],
    [
      "an mcp env_from of another variable",
      "mcp.yaml",
      "srv:\n  transport: stdio\n  command: run-it\n  env_from: [FOO]\n",
    ],
    [
      "a passthrough settings.json of another variable",
      "targets/claude/.claude/settings.json",
      '{"env": {"FOO": "x"}}\n',
    ],
    [
      "a passthrough config.toml that names none of them",
      "targets/codex/.codex/config.toml",
      'model = "x"\nurl = "https://example.invalid/homepage"\n',
    ],
  ])("still builds with %s", async (_label, file, text) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${file}`, text);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.stderr).not.toContain("protected-variable");
    expect(result.code).toBe(0);
  });
});

describe("a variable name that is not a plain name", () => {
  // `NAME=x` would reach a spawn as `CODEX_HOME=/x=<value>`, and NUL ends a name early.
  const BAD = "CODEX_HOME=/x";
  it.each([
    ["claude env", "settings.yaml", `claude:\n  env:\n    "${BAD}": x\n`],
    [
      "codex set",
      "settings.yaml",
      `codex:\n  shell_environment_policy:\n    set:\n      "${BAD}": x\n`,
    ],
    [
      "mcp env_from",
      "mcp.yaml",
      `srv:\n  transport: stdio\n  command: run-it\n  env_from: ["${BAD}"]\n`,
    ],
    [
      "passthrough settings.json",
      "targets/claude/.claude/settings.json",
      `{"env": {"${BAD}": "x"}}\n`,
    ],
    [
      "passthrough settings.json with NUL",
      "targets/claude/.claude/settings.json",
      '{"env": {"CODEX_HOME\\u0000X": "x"}}\n',
    ],
    [
      "passthrough .mcp.json",
      "targets/claude/.mcp.json",
      `{"mcpServers": {"s": {"env": {"${BAD}": "x"}}}}\n`,
    ],
  ])("is refused in %s, naming the key path and not the key", async (_label, file, text) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${file}`, text);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(`error [protected-variable] personal:${file}`);
    expect(result.stderr).toContain("not a plain variable name");
    expect(result.stderr).not.toContain("/x");
    expect(result.stderr).not.toContain("=");
  });
});

describe("passthrough paths that a file system could fold into a checked name", () => {
  it.each([
    ["a long s in settings.json", "targets/claude/.claude/\u017fettings.json"],
    ["a non-ASCII name under .claude", "targets/claude/.claude/caf\u00e9.md"],
    ["a non-ASCII name under .codex", "targets/codex/.codex/caf\u00e9.toml"],
    ["a non-ASCII .mcp.json path", "targets/claude/\u00e9/.mcp.json"],
    ["a long s in the .mcp.json file name", "targets/claude/.mcp.j\u017fon"],
    ["a non-ASCII name outside any config folder", "targets/claude/docs/caf\u00e9.md"],
    ["a non-ASCII target name", "targets/cl\u00e4ude/x.md"],
  ])("refuses %s", async (_label, file) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${file}`, "{}\n");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("has a non-ASCII character");
    expect(result.stderr).toContain("error [protected-variable]");
  });
});

describe("more refusals and controls for policy patterns and TOML text", () => {
  it.each([
    [
      "a filters action that is misspelt",
      "settings.yaml",
      "codex:\n  shell_environment_policy:\n    filters:\n      HOME: exlude\n",
    ],
    [
      "a config.toml whose only refusable content is the policy table",
      "targets/codex/.codex/config.toml",
      '[shell_environment_policy]\nexclude = ["*ome*"]\n',
    ],
  ])("refuses %s", async (_label, file, text) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${file}`, text);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [protected-variable]");
  });

  it("names shell_environment_policy for a config.toml that only has the policy table", async () => {
    const ws = fixture("29-account-layer");
    put(
      ws,
      "personal/targets/codex/.codex/config.toml",
      '[shell_environment_policy]\nexclude = ["*ome*"]\n',
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.stderr).toContain(".toml file names shell_environment_policy;");
  });
});

/**
 * A crash is not a diagnostic: an internal failure (a TypeError in the TypeScript reference, a
 * panic in Rust) exits 70, or prints text that is no diagnostic line. Every stderr line must be
 * `<level> [<code>] <where>: <message>` (decision-26), and the exit code a handled one.
 */
const DIAGNOSTIC_LINE = /^(error|warning|info) \[[a-z0-9-]+\] .+: .+$/;

function expectHandled(result: { code: number; stderr: string }): void {
  expect([0, 1], `exit code ${result.code}, stderr: ${result.stderr}`).toContain(result.code);
  const lines = result.stderr.split("\n").filter((line) => line !== "");
  expect(lines.filter((line) => !DIAGNOSTIC_LINE.test(line))).toEqual([]);
}

describe("names that are properties of every object", () => {
  it.each([
    ["a passthrough target", "targets/__proto__/constructor"],
    ["a passthrough path", "targets/claude/__proto__"],
    ["a passthrough path named constructor", "targets/claude/constructor"],
    ["a passthrough JSON path", "targets/claude/.claude/__proto__.json"],
  ])("gives no crash for %s", async (_label, file) => {
    const ws = fixture("29-account-layer");
    put(ws, `personal/${file}`, "x\n");
    const result = await layers(ws, "repo", ["build"]);
    expectHandled(result);
  });

  it.each(["__proto__", "constructor", "toString"])(
    "settings.yaml keys named %s are treated as plain keys",
    async (name) => {
      const ws = fixture("29-account-layer");
      put(ws, "personal/settings.yaml", `claude:\n  ${name}: x\n`);
      const result = await layers(ws, "repo", ["build"]);
      expectHandled(result);
      expect(result.stderr).not.toContain("is set through");
    },
  );
});

describe("an MCP server refused for a protected variable", () => {
  it("does not also report that mcp.enable names a server no layer defines", async () => {
    const ws = fixture("29-account-layer");
    put(
      ws,
      "personal/mcp.yaml",
      "srv:\n  transport: stdio\n  command: run-it\n  env_from: [HOME]\n",
    );
    put(
      ws,
      "repo/layer.yaml",
      "stack:\n  - source: path:../personal\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\nmcp:\n  enable: [srv]\n",
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [protected-variable] personal:mcp.yaml");
    expect(result.stderr).not.toContain("mcp.enable lists");
  });
});

describe("one report per problem", () => {
  it("reports a protected variable in .mcp.json mcpServers once", async () => {
    const ws = fixture("29-account-layer");
    put(
      ws,
      "personal/targets/claude/.mcp.json",
      '{"mcpServers": {"s": {"env": {"CODEX_HOME": "x"}}}}\n',
    );
    const result = await layers(ws, "repo", ["build"]);
    const reports = result.stderr
      .split("\n")
      .filter((line) =>
        line.includes("error [protected-variable] personal:targets/claude/.mcp.json"),
      );
    expect(reports).toHaveLength(1);
  });

  it("refuses a JSON file nested deeper than the checker reads, instead of stopping silently", async () => {
    const ws = fixture("29-account-layer");
    const deep = `${'{"a":'.repeat(40)}{"env":{"CODEX_HOME":"x"}}${"}".repeat(40)}`;
    put(ws, "personal/targets/claude/.claude/settings.json", `{"x": ${deep}}\n`);
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("is nested deeper than 32 levels, so it cannot be checked");
    const reports = result.stderr
      .split("\n")
      .filter((line) => line.includes("nested deeper than 32 levels"));
    expect(reports).toHaveLength(1);
  });
});

// §7.13 'Who may declare it': once built, resolution ignores a declaration by a layer that may
// not make one and prints one warning. The client's allowed account still applies.
describe("a refused account declaration, at resolution", () => {
  const ignoredLines = (text: string) =>
    text.split("\n").filter((line) => line.startsWith("warning [account-declaration-ignored]"));

  it("ignores a repo-layer declaration added after the build, with one warning for both targets", async () => {
    const s = await built();
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    writeAllowed(s, [hubEntry(s)]);
    appendFileSync(`${s.real}/repo/.layer/layer.yaml`, "account: repo-account\n");

    const result = await alz(s, ["account", "which"]);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");
    expect(result.stdout).toContain("codex: client-a-api (source: layer)");
    expect(ignoredLines(result.stderr)).toHaveLength(1);
    expect(result.stderr).toContain("it is inside the repository");
  });

  it("ignores a group layer's declaration read from a cache entry and a lock, with one warning", async () => {
    const s = plain();
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    const layer = (name: string, kind: string, visibility: string, account: string) => ({
      "layer.yaml": `name: ${name}\nkind: ${kind}\nvisibility: ${visibility}\nversion: 0.1.0\n${account}`,
      "instructions/10-x.md": `- ${name}.\n`,
    });
    remoteStack(s, [
      {
        url: "https://example.invalid/first.git",
        ref: "v1",
        sha: "3".repeat(40),
        files: layer("first", "personal", "public", ""),
      },
      {
        url: "https://example.invalid/client.git",
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
        url: "https://example.invalid/group.git",
        ref: "v1",
        sha: "2".repeat(40),
        files: layer("group", "group", "private", "account: group-a\n"),
      },
    ]);
    writeAllowed(s, [hubEntry(s, { source: "https://example.invalid/client.git" })]);

    const result = await alz(s, ["account", "which"]);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("claude: client-a (source: layer)");
    expect(result.stdout).toContain("codex: client-a-api (source: layer)");
    expect(ignoredLines(result.stderr)).toHaveLength(1);
    expect(result.stderr).toContain("its kind is 'group', not 'client'");
    expect(result.stderr).toContain('"git:https://example.invalid/group.git@v1"');
  });
});

// `alz account allow` itself needs a person at a terminal, so what it prints before it asks is a
// reference case until R1.8; what the binary shows without one is the account that applies.
describe("an account declaration by a repository that is not to be trusted, without a terminal", () => {
  const hasControl = (text: string): boolean =>
    [...text].some((char) => {
      const code = char.codePointAt(0) ?? 0;
      return code !== 0x0a && (code <= 0x1f || (code >= 0x7f && code <= 0x9f));
    });

  it("prints no control character for a hub directory whose name holds ESC, from build and account which", async () => {
    const s = await built("v1", "32-account-allow");
    const folder = "we\u001b[31mird";
    cpSync(`${s.ws.root}/hub/layer`, `${s.ws.root}/${folder}/layer`, { recursive: true });
    writeFileSync(
      `${s.ws.root}/repo/layer.yaml`,
      `stack:\n  - source: path:../personal\n  - source: ${JSON.stringify(`path:../${folder}/layer`)}\n  - source: path:./.layer\ntargets: [claude]\n`,
    );
    const build = await alz(s, ["build"]);
    expect(build.code, build.stderr).toBe(0);
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    writeAllowed(s, [hubEntry(s, { source: `${s.real}/${folder}/layer` })]);
    const which = await alz(s, ["account", "which"]);
    expect(which.code, which.stderr).toBe(0);
    expect(which.stdout).toContain("claude: client-a (source: layer)");
    expect(hasControl(build.stdout + build.stderr + which.stdout + which.stderr)).toBe(false);
  });
});
