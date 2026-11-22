import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { fixture, layers, readFile, readTree, type Workspace } from "../helpers";

// Reserved addresses only: the wrong identity must never appear in any output.
const WRONG_NAME = "Shared CI";
const WRONG_EMAIL = "ci@example.invalid";
const ENV = { GIT_CONFIG_NOSYSTEM: "1" };
// .git, and the symlinks some tests add (readTree cannot read a link to a directory).
const TREE_IGNORE = [".git", "ext", "own", "link"];

function sh(ws: Workspace, dir: string, args: string[]): string {
  return execFileSync("git", args, {
    cwd: `${ws.root}/${dir}`,
    encoding: "utf8",
    env: { PATH: process.env.PATH ?? "", HOME: ws.home, GIT_CONFIG_NOSYSTEM: "1" },
  }).trim();
}

const RIGHT_GLOBAL = "[user]\n\tname = Owner Example\n\temail = owner@client.example\n";

/** The fixture with `repo` as a git repository; the isolated global config holds the wrong identity unless told otherwise. */
function gitFixture(global: "wrong" | "right" = "wrong"): Workspace {
  const ws = fixture("26-commit-identity");
  writeFileSync(
    `${ws.home}/.gitconfig`,
    global === "wrong"
      ? `[user]\n\tname = ${WRONG_NAME}\n\temail = ${WRONG_EMAIL}\n`
      : RIGHT_GLOBAL,
  );
  sh(ws, "repo", ["init", "--quiet", "--initial-branch=main"]);
  return ws;
}

function gitConfig(ws: Workspace): string {
  return readFile(ws, "repo/.git/config");
}

function expectNoWrongValue(...results: { stdout: string; stderr: string }[]): void {
  for (const { stdout, stderr } of results) {
    for (const text of [stdout, stderr]) {
      expect(text).not.toContain(WRONG_EMAIL);
      expect(text).not.toContain(WRONG_NAME);
    }
  }
}

function setIdentity(ws: Workspace, layer: string, identity: string): void {
  const path = `${ws.root}/${layer}/layer.yaml`;
  const text = readFileSync(path, "utf8").replace(/commit_identity:[\s\S]*$/, "");
  writeFileSync(path, `${text}commit_identity:\n${identity}`);
}

/** A `commit_identity` body. JSON strings are YAML strings; non-ASCII is escaped so no scalar breaks a line. */
function identityYaml(name: unknown, email: unknown): string {
  const scalar = (value: unknown) =>
    JSON.stringify(value).replace(
      /[\u0080-\uffff]/g,
      (char) => `\\u${char.charCodeAt(0).toString(16).padStart(4, "0")}`,
    );
  return `  name: ${scalar(name)}\n  email: ${scalar(email)}\n`;
}

/** Does this machine's temp directory ignore case in file names? */
function caseInsensitiveTmp(): boolean {
  const dir = mkdtempSync(join(tmpdir(), "alz-case-"));
  try {
    writeFileSync(join(dir, "a"), "");
    return existsSync(join(dir, "A"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const CLIENT_IDENTITY = "  name: Owner Example\n  email: owner@client.example\n";

function writeLayer(ws: Workspace, dir: string, yaml: string): void {
  mkdirSync(`${ws.root}/${dir}`, { recursive: true });
  writeFileSync(`${ws.root}/${dir}/layer.yaml`, yaml);
}

function setStack(ws: Workspace, sources: string[]): void {
  const entries = sources.map((source) => `  - source: ${source}\n`).join("");
  writeFileSync(`${ws.root}/repo/layer.yaml`, `stack:\n${entries}targets: [claude]\n`);
}

/** The build must fail with `message`, name nothing wrong, and write nothing. */
async function expectRejected(ws: Workspace, ...messages: string[]): Promise<void> {
  const config = gitConfig(ws);
  const tree = readTree(`${ws.root}/repo`, TREE_IGNORE);
  const result = await layers(ws, "repo", ["build"], ENV);
  expect(result.code).toBe(1);
  for (const message of messages) expect(result.stderr).toContain(message);
  expect(readTree(`${ws.root}/repo`, TREE_IGNORE)).toEqual(tree);
  expect(gitConfig(ws)).toBe(config);
  expectNoWrongValue(result);
}

function removeIdentity(ws: Workspace, layer: string): void {
  const path = `${ws.root}/${layer}/layer.yaml`;
  writeFileSync(path, readFileSync(path, "utf8").replace(/commit_identity:[\s\S]*$/, ""));
}

/** The exact §7.12 section, as the spec shows it. */
function commitsSection(who: string, layer: string): string {
  return `## Commits

- Author and committer of every commit made in a local clone: \`${who}\` (declared by layer \`${layer}\`). This repository's local git config sets it; never change \`user.name\` or \`user.email\`, and never commit locally under a CI, bot or shared identity.
- A commit made by an agent ends with a \`Co-Authored-By:\` trailer that names the agent. Use the trailer your tool adds; if it adds none, write one.
- If \`git var GIT_AUTHOR_IDENT\` shows another identity, do not commit; report it.
- Bots that commit on the hosting service, such as release or CI bots, are exempt from this rule.
`;
}

describe("§13.18 commit identity", () => {
  describe("schema (§7.12 validation)", () => {
    it.each([
      ["a newline in the name", identityYaml("Owner\nExample", "owner@example.com")],
      ["a control character in the name", identityYaml("Owner\u0007", "owner@example.com")],
      ["a C1 control character in the name", identityYaml("Own\u0085er", "owner@example.com")],
      ["a line separator in the name", identityYaml("Own\u2028er", "owner@example.com")],
      ["a zero-width space in the name", identityYaml("Own\u200ber", "owner@example.com")],
      ["a bidi control in the name", identityYaml("Own\u202eer", "owner@example.com")],
      ["a soft hyphen in the name", identityYaml("Own\u00ader", "owner@example.com")],
      ["a name over 128 characters", identityYaml("a".repeat(129), "owner@example.com")],
      ["an option-like name", identityYaml("--upload-pack=x", "owner@example.com")],
      ["a name with a leading space", identityYaml(" Owner", "owner@example.com")],
      ["a name with a trailing space", identityYaml("Owner ", "owner@example.com")],
      ["a name ending in a period", identityYaml("Owner.", "owner@example.com")],
      ["a name starting with a period", identityYaml(".Owner", "owner@example.com")],
      ["a name ending in a comma", identityYaml("Owner,", "owner@example.com")],
      ["a name ending in a semicolon", identityYaml("Owner;", "owner@example.com")],
      ["a name ending in a colon", identityYaml("Owner:", "owner@example.com")],
      ["a name starting with a double quote", identityYaml('"Owner', "owner@example.com")],
      ["a name ending in an apostrophe", identityYaml("Owner'", "owner@example.com")],
      ["a name with a backslash", identityYaml("Own\\er", "owner@example.com")],
      ["a name with a backslash and an apostrophe", identityYaml("Owner\\'x", "owner@example.com")],
      ["a name with <", identityYaml("Own<er", "owner@example.com")],
      ["a name with >", identityYaml("Own>er", "owner@example.com")],
      ["a name with a backtick", identityYaml("Own`er", "owner@example.com")],
      ["a name that is not a string", identityYaml(12, "owner@example.com")],
      ["an email that is not a string", identityYaml("Owner", true)],
      ["an option-like email", identityYaml("Owner", "-x@example.com")],
      ["an email starting with a period", identityYaml("Owner", ".owner@example.com")],
      ["an email with a zero-width space", identityYaml("Owner", "owner@exam\u200bple.com")],
      ["an email without @", identityYaml("Owner", "owner.example.com")],
      ["an email over 254 characters", identityYaml("Owner", `${"a".repeat(250)}@example.com`)],
      ["an extra key", `${identityYaml("Owner", "owner@example.com")}  signing_key: abc\n`],
      ["a missing email", "  name: Owner\n"],
    ])(
      "rejects %s as a schema error, before build --adopt writes anything",
      async (_label, identity) => {
        const ws = gitFixture();
        setIdentity(ws, "personal", identity);
        const before = gitConfig(ws);

        const result = await layers(ws, "repo", ["build", "--adopt"], ENV);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [layer]");
        expect(result.stderr).toContain("commit_identity");
        expect(result.stdout).not.toContain("set user.");
        expect(gitConfig(ws)).toBe(before);
        expectNoWrongValue(result);
      },
    );

    it("accepts an apostrophe inside a name", async () => {
      const ws = gitFixture();
      setIdentity(ws, "client", identityYaml("O'Example", "owner@client.example"));
      const result = await layers(ws, "repo", ["lint", "--layer", "../client"], ENV);
      expect(result.code).toBe(0);
    });

    it("lints a valid declaration in a single layer", async () => {
      const ws = gitFixture();
      for (const layer of ["personal", "client"]) {
        const result = await layers(ws, "repo", ["lint", "--layer", `../${layer}`], ENV);
        expect(result.code).toBe(0);
      }
    });
  });

  describe("who may declare it (§7.12)", () => {
    it("rejects a declaration in the repo layer, whatever its kind says", async () => {
      for (const kind of ["repo", "client"]) {
        const ws = gitFixture();
        writeLayer(
          ws,
          "repo/.layer",
          `name: repo\nkind: ${kind}\nvisibility: private\ncommit_identity:\n${CLIENT_IDENTITY}`,
        );
        await expectRejected(ws, "repo:layer.yaml", "inside the repository");
      }
    });

    it("rejects a declaration in a group layer", async () => {
      const ws = gitFixture();
      writeLayer(
        ws,
        "group",
        `name: group\nkind: group\nvisibility: private\ncommit_identity:\n${CLIENT_IDENTITY}`,
      );
      setStack(ws, ["path:../personal", "path:../client", "path:../group", "path:./.layer"]);
      await expectRejected(ws, "group:layer.yaml", "its kind is 'group', not 'client'");
    });

    it("rejects a public layer that calls itself client", async () => {
      const ws = gitFixture();
      writeLayer(
        ws,
        "client",
        `name: client\nkind: client\nvisibility: public\ncommit_identity:\n${CLIENT_IDENTITY}`,
      );
      await expectRejected(ws, "client:layer.yaml", "it is public");
    });

    it("rejects a second client layer and names both", async () => {
      const ws = gitFixture();
      writeLayer(
        ws,
        "client2",
        `name: client2\nkind: client\nvisibility: private\ncommit_identity:\n${CLIENT_IDENTITY}`,
      );
      setStack(ws, ["path:../personal", "path:../client", "path:../client2", "path:./.layer"]);
      await expectRejected(ws, "'client' and 'client2'");
    });

    it("rejects a symlink inside the repo that points at a client layer outside it", async () => {
      const ws = gitFixture();
      symlinkSync(`${ws.root}/client`, `${ws.root}/repo/ext`);
      setStack(ws, ["path:../personal", "path:./ext", "path:./.layer"]);
      await expectRejected(ws, "client:layer.yaml", "inside the repository");
    });

    it("rejects a first layer that sits inside the repository", async () => {
      const ws = gitFixture();
      symlinkSync(`${ws.root}/personal`, `${ws.root}/repo/own`);
      setStack(ws, ["path:./own", "path:./.layer"]);
      await expectRejected(ws, "personal:layer.yaml", "inside the repository");
    });
  });

  describe("generated Commits rule (§7.12)", () => {
    it("lets the client layer override the first layer and names it", async () => {
      const ws = gitFixture();
      const result = await layers(ws, "repo", ["build"], ENV);
      expect(result.code).toBe(0);
      const agents = readFile(ws, "repo/AGENTS.md");
      expect(
        agents.endsWith(`\n${commitsSection("Owner Example <owner@client.example>", "client")}`),
      ).toBe(true);
      expect(agents).not.toContain("owner@example.com");
      expect(agents.split("\n")[0]).toContain("+ client@0.1.0:layer.yaml. Edit");
      expectNoWrongValue(result);
    });

    it("uses the first layer's identity when no client layer declares one", async () => {
      const ws = gitFixture();
      removeIdentity(ws, "client");
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const agents = readFile(ws, "repo/AGENTS.md");
      expect(
        agents.endsWith(`\n${commitsSection("Owner Example <owner@example.com>", "personal")}`),
      ).toBe(true);
      expect(agents.split("\n")[0]).toContain("+ personal@1.0.0:layer.yaml. Edit");
    });

    it("adds nothing when no layer declares an identity", async () => {
      const ws = gitFixture();
      removeIdentity(ws, "client");
      removeIdentity(ws, "personal");
      const result = await layers(ws, "repo", ["build"], ENV);
      expect(result.code).toBe(0);
      const agents = readFile(ws, "repo/AGENTS.md");
      expect(agents).not.toContain("Commits");
      expect(agents.split("\n")[0]).not.toContain("layer.yaml");
      expect(result.stderr).toBe("");
    });

    it("builds byte-identical output twice and in two workspaces", async () => {
      const first = gitFixture();
      const second = gitFixture();
      expect((await layers(first, "repo", ["build"], ENV)).code).toBe(0);
      const once = readTree(`${first.root}/repo`, TREE_IGNORE);
      expect((await layers(first, "repo", ["build"], ENV)).code).toBe(0);
      expect(readTree(`${first.root}/repo`, TREE_IGNORE)).toEqual(once);
      expect((await layers(second, "repo", ["build"], ENV)).code).toBe(0);
      expect(readTree(`${second.root}/repo`, TREE_IGNORE)).toEqual(once);
    });

    it("counts the section towards instructions_max_lines", async () => {
      const ws = gitFixture();
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const lines = readFile(ws, "repo/AGENTS.md").split("\n").length - 1;
      const manifest = readFile(ws, "repo/layer.yaml");
      writeFileSync(
        `${ws.root}/repo/layer.yaml`,
        `${manifest}budgets:\n  instructions_max_lines: ${lines - 1}\n`,
      );
      const over = await layers(ws, "repo", ["build"], ENV);
      expect(over.code).toBe(1);
      expect(over.stderr).toContain("error [budget]");
      writeFileSync(
        `${ws.root}/repo/layer.yaml`,
        `${manifest}budgets:\n  instructions_max_lines: ${lines}\n`,
      );
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
    });
  });

  describe("alz check (§9.4)", () => {
    it("fails on a wrong global identity and names the layer, scope and file but not the value", async () => {
      const ws = gitFixture();
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check"], ENV);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("error [identity]");
      expect(result.stderr).toContain("layer 'client' (path:../client)");
      expect(result.stderr).toContain("global config (~/.gitconfig)");
      expect(result.stderr).toContain("git config --local user.email 'owner@client.example'");
      expect(result.stderr).toContain("git config --local user.name 'Owner Example'");
      expect(result.stdout).not.toContain("check passed");
      expectNoWrongValue(result);
    });

    it("fails on a wrong environment identity and names the variable only", async () => {
      const ws = gitFixture("right");
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check"], {
        ...ENV,
        GIT_AUTHOR_EMAIL: "env@example.invalid",
      });
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("the environment (GIT_AUTHOR_EMAIL)");
      expect(result.stderr).toContain("unset GIT_AUTHOR_EMAIL");
      expect(result.stderr).not.toContain("GIT_COMMITTER_EMAIL");
      expect(result.stderr).not.toContain("env@example.invalid");
    });

    it("fails on a wrong local identity and names .git/config", async () => {
      const ws = gitFixture("right");
      sh(ws, "repo", ["config", "--local", "user.email", "other@example.invalid"]);
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check"], ENV);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("local config (.git/config)");
      expect(result.stderr).not.toContain("other@example.invalid");
    });

    it("passes with --skip-identity", async () => {
      const ws = gitFixture();
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check", "--skip-identity"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).not.toContain("[identity]");
      expectNoWrongValue(result);
    });

    it("passes when the effective identity matches", async () => {
      const ws = gitFixture("right");
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toBe("");
    });

    it("notes that a directory outside any git repository was not checked", async () => {
      const ws = fixture("26-commit-identity");
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("note [identity]");
    });
  });

  describe("setting it up (§9.4)", () => {
    it("build --adopt adds the client identity to .git/config and leaves the global file alone", async () => {
      const ws = gitFixture();
      const global = readFileSync(`${ws.home}/.gitconfig`, "utf8");
      const result = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(result.stdout).toContain(
        "set user.email in .git/config to 'owner@client.example' (declared by layer 'client' (path:../client))",
      );
      expect(result.stdout).toContain("set user.name in .git/config to 'Owner Example'");
      expect(result.stderr).not.toContain("[identity]");
      expect(sh(ws, "repo", ["config", "--local", "user.email"])).toBe("owner@client.example");
      expect(sh(ws, "repo", ["config", "--local", "user.name"])).toBe("Owner Example");
      expect(readFileSync(`${ws.home}/.gitconfig`, "utf8")).toBe(global);
      expectNoWrongValue(result);
      const check = await layers(ws, "repo", ["check"], ENV);
      expect(check.code).toBe(0);
      expectNoWrongValue(check);
    });

    it("a second build --adopt changes nothing", async () => {
      const ws = gitFixture();
      expect((await layers(ws, "repo", ["build", "--adopt"], ENV)).code).toBe(0);
      const config = gitConfig(ws);
      const tree = readTree(`${ws.root}/repo`, TREE_IGNORE);
      const again = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(again.code).toBe(0);
      expect(again.stdout).not.toContain("set user.");
      expect(gitConfig(ws)).toBe(config);
      expect(readTree(`${ws.root}/repo`, TREE_IGNORE)).toEqual(tree);
    });

    it("never replaces a different local value: it warns, prints the fix, and check fails", async () => {
      const ws = gitFixture("right");
      sh(ws, "repo", ["config", "--local", "user.email", "other@example.invalid"]);
      const result = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(sh(ws, "repo", ["config", "--local", "user.email"])).toBe("other@example.invalid");
      expect(result.stdout).toContain("set user.name in .git/config");
      expect(result.stderr).toContain("warning [identity] user.email in .git/config differs");
      expect(result.stderr).toContain("git config --local user.email 'owner@client.example'");
      expect(result.stderr).not.toContain("other@example.invalid");
      const check = await layers(ws, "repo", ["check"], ENV);
      expect(check.code).toBe(1);
      expect(check.stderr).not.toContain("other@example.invalid");
    });

    it("plain build warns about a wrong identity and writes nothing", async () => {
      const ws = gitFixture();
      const config = gitConfig(ws);
      const result = await layers(ws, "repo", ["build"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [identity]");
      expect(result.stderr).toContain("global config (~/.gitconfig)");
      expect(result.stdout).not.toContain("set user.");
      expect(gitConfig(ws)).toBe(config);
      expectNoWrongValue(result);
    });

    it("reports an environment variable that still overrides the written identity", async () => {
      const ws = gitFixture();
      const result = await layers(ws, "repo", ["build", "--adopt"], {
        ...ENV,
        GIT_COMMITTER_NAME: "Env Person",
      });
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("the environment (GIT_COMMITTER_NAME)");
      expect(result.stderr).not.toContain("Env Person");
      expectNoWrongValue(result);
    });

    it("init --stack sets the first layer's identity in a fresh repository", async () => {
      const ws = gitFixture();
      mkdirSync(`${ws.root}/fresh`);
      sh(ws, "fresh", ["init", "--quiet", "--initial-branch=main"]);
      const result = await layers(ws, "fresh", ["init", "--stack", "../personal"], ENV);
      expect(result.code).toBe(0);
      expect(result.stdout).toContain(
        "set user.email in .git/config to 'owner@example.com' (declared by layer 'personal' (path:../personal))",
      );
      expect(sh(ws, "fresh", ["config", "--local", "user.email"])).toBe("owner@example.com");
      expect(sh(ws, "fresh", ["config", "--local", "user.name"])).toBe("Owner Example");
      expectNoWrongValue(result);
    });

    it("init only notes that build --adopt will set it when the stack cannot be resolved yet", async () => {
      const ws = gitFixture();
      mkdirSync(`${ws.root}/fresh`);
      sh(ws, "fresh", ["init", "--quiet", "--initial-branch=main"]);
      const result = await layers(ws, "fresh", ["init", "--stack", "../missing"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("note [identity]");
      expect(result.stderr).toContain("alz build --adopt");
      expect(existsSync(`${ws.root}/fresh/.git/config`)).toBe(true);
      expect(readFileSync(`${ws.root}/fresh/.git/config`, "utf8")).not.toContain("user");
    });

    it("writes nothing when .git is a gitfile and prints the commands to run by hand", async () => {
      const ws = gitFixture("right");
      sh(ws, "repo", ["add", "layer.yaml", ".layer"]);
      sh(ws, "repo", [
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "commit",
        "--quiet",
        "-m",
        "scaffold",
      ]);
      sh(ws, "repo", ["worktree", "add", "--quiet", "../wt", "-b", "wt"]);
      // The linked worktree sees a wrong global identity.
      writeFileSync(
        `${ws.home}/.gitconfig`,
        `[user]\n\tname = ${WRONG_NAME}\n\temail = ${WRONG_EMAIL}\n`,
      );
      const config = gitConfig(ws);

      const result = await layers(ws, "wt", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [identity] .git is not a plain directory");
      expect(result.stderr).toContain("git config --local user.email 'owner@client.example'");
      expect(result.stdout).not.toContain("set user.");
      expect(gitConfig(ws)).toBe(config);
      expectNoWrongValue(result);
    });

    it("writes nothing when the repo root is inside another work tree", async () => {
      const ws = fixture("26-commit-identity");
      writeFileSync(
        `${ws.home}/.gitconfig`,
        `[user]\n\tname = ${WRONG_NAME}\n\temail = ${WRONG_EMAIL}\n`,
      );
      sh(ws, ".", ["init", "--quiet", "--initial-branch=main"]);
      const outer = readFile(ws, ".git/config");
      const result = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("not the top level of its work tree");
      expect(readFile(ws, ".git/config")).toBe(outer);
      expect(existsSync(`${ws.root}/repo/.git`)).toBe(false);
      expectNoWrongValue(result);
    });
  });

  describe("hardening", () => {
    const CLIENT_DECLARES = `name: client\nkind: client\nvisibility: private\ncommit_identity:\n${CLIENT_IDENTITY}`;

    it("does not write through a .git/config that is a symlink", async () => {
      const ws = gitFixture();
      const target = `${ws.home}/elsewhere-config`;
      writeFileSync(target, readFileSync(`${ws.root}/repo/.git/config`, "utf8"));
      rmSync(`${ws.root}/repo/.git/config`);
      symlinkSync(target, `${ws.root}/repo/.git/config`);
      const before = readFileSync(target, "utf8");

      const result = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [identity] .git/config is not a regular file");
      expect(result.stdout).not.toContain("set user.");
      expect(readFileSync(target, "utf8")).toBe(before);
      expect(lstatSync(`${ws.root}/repo/.git/config`).isSymbolicLink()).toBe(true);
      expectNoWrongValue(result);
    });

    it("gives no shell command for a name with an apostrophe, and still writes it", async () => {
      const ws = gitFixture();
      setIdentity(ws, "client", identityYaml("O'Example", "owner@client.example"));
      const build = await layers(ws, "repo", ["build"], ENV);
      expect(build.code).toBe(0);
      const check = await layers(ws, "repo", ["check"], ENV);
      expect(check.code).toBe(1);
      expect(check.stderr).toContain(
        "set user.name to the value declared in layer 'client' (path:../client) (`alz build --adopt` adds it when unset)",
      );
      expect(check.stderr).not.toContain("git config --local user.name");
      expect(check.stderr).toContain("git config --local user.email 'owner@client.example'");
      expectNoWrongValue(check);

      const adopt = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(adopt.code).toBe(0);
      expect(sh(ws, "repo", ["config", "--local", "user.name"])).toBe("O'Example");
      expect((await layers(ws, "repo", ["check"], ENV)).code).toBe(0);
    });

    /** A bare repo of the client layer, committed at `dir` inside `repo`, tagged v1. */
    function bareClientIn(ws: Workspace, dir: string): void {
      if (!existsSync(`${ws.root}/client-src`)) {
        cpSync(`${ws.root}/client`, `${ws.root}/client-src`, { recursive: true });
        sh(ws, "client-src", ["init", "--quiet", "--initial-branch=main"]);
        sh(ws, "client-src", ["add", "."]);
        sh(ws, "client-src", [
          "-c",
          "user.name=Test",
          "-c",
          "user.email=test@example.com",
          "commit",
          "--quiet",
          "-m",
          "client",
        ]);
        sh(ws, "client-src", ["tag", "v1"]);
      }
      mkdirSync(join(`${ws.root}/repo`, dir, ".."), { recursive: true });
      sh(ws, "repo", ["clone", "--bare", "--quiet", "../client-src", dir]);
    }

    it.each([
      ["a local git: path", "git:./identity.git@v1"],
      ["a file:// URL", "git:file://REPO/identity.git@v1"],
    ])("rejects a client layer from %s inside the repository", async (_label, source) => {
      const ws = gitFixture();
      bareClientIn(ws, "identity.git");
      setStack(ws, [
        "path:../personal",
        source.replace("REPO", `${ws.root}/repo`),
        "path:./.layer",
      ]);
      await expectRejected(ws, "client:layer.yaml", "inside the repository");
    });

    it("rejects a client layer from an in-repo path that looks like host:path", async () => {
      const ws = gitFixture();
      bareClientIn(ws, "x:y");
      setStack(ws, ["path:../personal", "git:x:y@v1", "path:./.layer"]);
      await expectRejected(ws, "client:layer.yaml", "inside the repository");
    });

    it("rejects a file:/// URL whose relative spelling is a folder in the repo", async () => {
      const ws = gitFixture();
      bareClientIn(ws, "file:/idrepo");
      setStack(ws, ["path:../personal", "git:file:///idrepo@v1", "path:./.layer"]);
      // Whether git then clones the in-repo folder or refuses it, nothing is built or written.
      await expectRejected(ws);
    });

    it("rejects a local git: source whose text and disk disagree about '..'", async () => {
      const ws = gitFixture();
      mkdirSync(`${ws.root}/repo/d1/d2/d3`, { recursive: true });
      symlinkSync("d1/d2/d3", `${ws.root}/repo/link`);
      // As text, link/../../x.git is outside the repo and exists; on disk it is the repo's own d1/x.git.
      mkdirSync(`${ws.root}/x.git`);
      bareClientIn(ws, "d1/x.git");
      setStack(ws, ["path:../personal", "git:link/../../x.git@v1", "path:./.layer"]);
      await expectRejected(ws, "must not contain '..' segments");
    });

    it.each([
      ["a relative path", "git:../client-src@v1"],
      ["a file:// URL", "git:file://REPO/../client-src@v1"],
    ])("gives a clear error for '..' in %s", async (_label, source) => {
      const ws = gitFixture();
      bareClientIn(ws, "identity.git");
      setStack(ws, [
        "path:../personal",
        source.replace("REPO", `${ws.root}/repo`),
        "path:./.layer",
      ]);
      const result = await layers(ws, "repo", ["build"], ENV);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("error [source]");
      expect(result.stderr).toContain("a local git: path must not contain '..' segments");
      expect(result.stderr).toContain("@v1'");
    });

    it("checks the canonical path of a symlink, and clones that path", async () => {
      const ws = gitFixture();
      bareClientIn(ws, "identity.git");
      symlinkSync(`${ws.root}/repo/identity.git`, `${ws.root}/outlink`);
      setStack(ws, ["path:../personal", `git:${ws.root}/outlink@v1`, "path:./.layer"]);
      await expectRejected(ws, "client:layer.yaml", "inside the repository");
    });

    it("keeps a guarded clone from falling back to a local path, even with GIT_ALLOW_PROTOCOL=file", async () => {
      const ws = gitFixture();
      bareClientIn(ws, "x:y");
      setStack(ws, ["path:../personal", "git:x:y@v1", "path:./.layer"]);
      const config = gitConfig(ws);
      const tree = readTree(`${ws.root}/repo`, TREE_IGNORE);
      // The variable would let git clone the local folder `x:y`; alz must refuse it all the same.
      const result = await layers(ws, "repo", ["build"], {
        ...ENV,
        GIT_ALLOW_PROTOCOL: "file",
      });
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("client:layer.yaml");
      expect(result.stderr).toContain("inside the repository");
      expect(readTree(`${ws.root}/repo`, TREE_IGNORE)).toEqual(tree);
      expect(gitConfig(ws)).toBe(config);
      expectNoWrongValue(result);
    });

    it("treats a git: source as remote only when no such local path exists", async () => {
      const ws = gitFixture();
      const stack = ["path:../personal", "git:host.invalid:client.git@v1", "path:./.layer"];
      // A remote that no folder shadows is not inside the repository, so it is fetched (and
      // fails here, since the host does not exist).
      setStack(ws, stack);
      const remote = await layers(ws, "repo", ["build"], ENV);
      expect(remote.code).toBe(1);
      expect(remote.stderr).not.toContain("inside the repository");

      // The same name is inside as soon as the folder git would try first exists.
      bareClientIn(ws, "host.invalid:client.git");
      await expectRejected(ws, "client:layer.yaml", "inside the repository");
    });

    it.skipIf(!caseInsensitiveTmp())(
      "rejects a client layer in the repo reached through a case variant of its path",
      async () => {
        const ws = gitFixture();
        writeLayer(ws, "repo/clayer", CLIENT_DECLARES);
        setStack(ws, ["path:../personal", "path:../REPO/clayer", "path:./.layer"]);
        await expectRejected(ws, "client:layer.yaml", "inside the repository");
      },
    );

    it("rejects a client layer reached through a link outside the repo that points into it", async () => {
      const ws = gitFixture();
      writeLayer(ws, "repo/inner", CLIENT_DECLARES);
      symlinkSync(`${ws.root}/repo/inner`, `${ws.root}/link`);
      setStack(ws, ["path:../personal", "path:../link", "path:./.layer"]);
      await expectRejected(ws, "client:layer.yaml", "inside the repository");
    });

    it("never prints control characters from a layer's kind", async () => {
      const ws = gitFixture();
      writeLayer(
        ws,
        "group",
        `name: group\nkind: "gro\\u001b[31mup\\u202e"\nvisibility: private\ncommit_identity:\n${CLIENT_IDENTITY}`,
      );
      setStack(ws, ["path:../personal", "path:../client", "path:../group", "path:./.layer"]);
      const result = await layers(ws, "repo", ["build"], ENV);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("group:layer.yaml");
      expect(result.stderr).not.toContain("\u001b");
      expect(result.stderr).not.toContain("\u202e");
    });

    it("init shows a commit_identity schema error instead of a 'could not be resolved' note", async () => {
      const ws = gitFixture();
      writeLayer(
        ws,
        "bad",
        `name: bad\nkind: personal\nvisibility: private\ncommit_identity:\n${identityYaml("Owner\nExample", "owner@example.com")}`,
      );
      mkdirSync(`${ws.root}/fresh`);
      sh(ws, "fresh", ["init", "--quiet", "--initial-branch=main"]);
      const result = await layers(ws, "fresh", ["init", "--stack", "../bad"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [layer]");
      expect(result.stderr).toContain("commit_identity");
      expect(result.stderr).not.toContain("could not be resolved yet");
      expect(readFileSync(`${ws.root}/fresh/.git/config`, "utf8")).not.toContain("user");
      // build fails on the same problem.
      expect((await layers(ws, "fresh", ["build"], ENV)).code).toBe(1);
    });

    it("init warns about an ineligible declaration, writes nothing, and exits 0", async () => {
      const ws = gitFixture();
      writeLayer(
        ws,
        "group",
        `name: group\nkind: group\nvisibility: private\ncommit_identity:\n${CLIENT_IDENTITY}`,
      );
      mkdirSync(`${ws.root}/fresh`);
      sh(ws, "fresh", ["init", "--quiet", "--initial-branch=main"]);
      const result = await layers(
        ws,
        "fresh",
        ["init", "--stack", "../personal", "--stack", "../group"],
        ENV,
      );
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [identity] group:layer.yaml");
      expect(result.stdout).not.toContain("set user.");
      expect(readFileSync(`${ws.root}/fresh/.git/config`, "utf8")).not.toContain("user");
    });

    it("names the source of a wrong identity even when setup is blocked", async () => {
      const ws = gitFixture();
      sh(ws, "repo", ["add", "layer.yaml", ".layer"]);
      sh(ws, "repo", [
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "commit",
        "--quiet",
        "-m",
        "scaffold",
      ]);
      sh(ws, "repo", ["worktree", "add", "--quiet", "../wt", "-b", "wt"]);
      const result = await layers(ws, "wt", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [identity] .git is not a plain directory");
      expect(result.stderr).toContain("global config (~/.gitconfig)");
      expectNoWrongValue(result);
    });

    it("tells you to run git init, not git config, when the repo root is inside another work tree", async () => {
      const ws = fixture("26-commit-identity");
      writeFileSync(
        `${ws.home}/.gitconfig`,
        `[user]\n\tname = ${WRONG_NAME}\n\temail = ${WRONG_EMAIL}\n`,
      );
      sh(ws, ".", ["init", "--quiet", "--initial-branch=main"]);
      const result = await layers(ws, "repo", ["build", "--adopt"], ENV);
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("Run `git init` here first");
      expect(result.stderr).not.toContain("git config --local");
      expect(result.stderr).toContain("global config (~/.gitconfig)");
      expectNoWrongValue(result);
    });

    it("tells you to unset the config variables when the identity comes from the command scope", async () => {
      const ws = gitFixture("right");
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const result = await layers(ws, "repo", ["check"], {
        ...ENV,
        GIT_CONFIG_COUNT: "1",
        GIT_CONFIG_KEY_0: "user.email",
        GIT_CONFIG_VALUE_0: "cmd@example.invalid",
      });
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("command-line config");
      expect(result.stderr).toContain("unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS");
      expect(result.stderr).not.toContain("git config --local");
      expect(result.stderr).not.toContain("cmd@example.invalid");
    });

    it("fails check, instead of passing, when git cannot be used", async () => {
      const ws = gitFixture("right");
      expect((await layers(ws, "repo", ["build"], ENV)).code).toBe(0);
      const missing = await layers(ws, "repo", ["check"], { ...ENV, PATH: "/nonexistent" });
      expect(missing.code).toBe(1);
      expect(missing.stderr).toContain("error [identity] git is not available or failed");
      expect(missing.stderr).not.toContain("not a git repository");

      writeFileSync(`${ws.home}/.gitconfig`, "[user\n");
      const broken = await layers(ws, "repo", ["check"], ENV);
      expect(broken.code).toBe(1);
      expect(broken.stderr).toContain("error [identity] git is not available or failed");
    });

    it("lists every client layer when more than two declare", async () => {
      const ws = gitFixture();
      for (const name of ["client2", "client3"]) {
        writeLayer(ws, name, CLIENT_DECLARES.replace("name: client", `name: ${name}`));
      }
      setStack(ws, [
        "path:../personal",
        "path:../client",
        "path:../client2",
        "path:../client3",
        "path:./.layer",
      ]);
      await expectRejected(ws, "'client', 'client2' and 'client3' all declare");
    });

    it("init writes nothing when the stack itself has errors", async () => {
      const ws = gitFixture();
      // A public layer after a private one breaks the visibility order.
      writeLayer(ws, "late-public", "name: late-public\nkind: repo\nvisibility: public\n");
      mkdirSync(`${ws.root}/fresh`);
      sh(ws, "fresh", ["init", "--quiet", "--initial-branch=main"]);
      const result = await layers(
        ws,
        "fresh",
        ["init", "--stack", "../client", "--stack", "../late-public"],
        ENV,
      );
      expect(result.code).toBe(0);
      expect(result.stderr).toContain("warning [visibility]");
      expect(result.stdout).not.toContain("set user.");
      expect(readFileSync(`${ws.root}/fresh/.git/config`, "utf8")).not.toContain("user");
    });
  });
});
