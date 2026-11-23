import { appendFileSync, cpSync, existsSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { run } from "../../src/cli/main";
import type { Terminal } from "../../src/cli/terminal";
import { resolveAccounts } from "../../src/resolve/account-resolution";
import {
  accountFolder,
  alz,
  built,
  envFor,
  hubEntry,
  plain,
  remoteStack,
  writeAllowed,
} from "../account-workspace";
import { layersInProcess } from "./in-process";

// §7.13 'Who may declare it': once built, resolution ignores a declaration by a layer that may
// not make one and prints one warning. The client's allowed account still applies.
describe("a refused account declaration, at resolution", () => {
  const ignoredLines = (text: string) =>
    text.split("\n").filter((line) => line.startsWith("warning [account-declaration-ignored]"));

  function resolve(s: Awaited<ReturnType<typeof built>>) {
    return resolveAccounts({
      cwd: `${s.real}/repo`,
      home: s.ws.home,
      env: { HOME: s.ws.home, ...envFor(s) },
      uid: process.getuid?.(),
    });
  }

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

    const resolved = resolve(s);
    expect(
      resolved.warnings.filter((warning) => warning.code === "declaration-ignored"),
    ).toHaveLength(1);
    expect(resolved.targets.map((target) => target.account)).toEqual(["client-a", "client-a-api"]);
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

    const resolved = resolve(s);
    expect(
      resolved.warnings.filter((warning) => warning.code === "declaration-ignored"),
    ).toHaveLength(1);
    expect(resolved.targets.map((target) => target.account)).toEqual(["client-a", "client-a-api"]);
  });
});

// `alz account allow` shows repository text before the person confirms: it must be the
// reader's own text, quoted, and a declaration that may not apply is never offered.
describe("alz account allow with a repository that is not to be trusted", () => {
  const allowAt = (s: Awaited<ReturnType<typeof built>>, term: Terminal) =>
    layersInProcess(s.ws, "repo", ["account", "allow"], envFor(s), term);
  const FIXTURE = "32-account-allow";
  const answers = (...names: string[]): { term: Terminal; asked: string[] } => {
    const asked: string[] = [];
    return {
      asked,
      term: {
        ask: async (prompt) => {
          asked.push(prompt);
          return names.shift();
        },
      },
    };
  };
  const hasControl = (text: string): boolean =>
    [...text].some((char) => {
      const code = char.codePointAt(0) ?? 0;
      return code !== 0x0a && (code <= 0x1f || (code >= 0x7f && code <= 0x9f));
    });

  it("refuses a declaration the repository's own layer makes, and warns that it is ignored", async () => {
    const s = await built("v1", FIXTURE);
    // Only the repository's layer declares: the stack has no client layer.
    writeFileSync(
      `${s.real}/repo/layer.yaml`,
      "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
    );
    expect((await alz(s, ["build"])).code).toBe(0);
    appendFileSync(`${s.real}/repo/.layer/layer.yaml`, "account: repo-account\n");
    const prompt = answers("repo-account");
    const result = await allowAt(s, prompt.term);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("the layers name no account here");
    expect(result.stderr).toContain("warning [account-declaration-ignored]");
    expect(result.stderr).toContain("it is inside the repository");
    expect(prompt.asked).toEqual([]);
    expect(existsSync(`${s.ws.home}/.local`)).toBe(false);
  });

  it("shows the source an external client layer names, quoted, before it asks", async () => {
    const s = await built("v1", FIXTURE);
    let out = "";
    let seenAtPrompt = "";
    let prompt = "";
    const code = await run(["account", "allow"], {
      cwd: `${s.real}/repo`,
      env: { HOME: s.ws.home, PATH: process.env.PATH ?? "", ...envFor(s) },
      stdout: (text) => {
        out += text;
      },
      stderr: () => {},
      terminal: {
        ask: async (text) => {
          prompt = text;
          seenAtPrompt = out;
          return undefined;
        },
      },
    });
    expect(code).toBe(1);
    // Everything the person decides on was printed before the first question, and the prompt
    // repeats the source, because standard output may be redirected.
    expect(seenAtPrompt).toContain('source as written: "path:../hub/layer"');
    expect(seenAtPrompt).toContain(`recorded source: "${s.hubLayer}"`);
    expect(prompt).toContain(`(source "${s.hubLayer}")`);
  });

  it("prints no control character for a hub directory whose name holds ESC", async () => {
    const s = await built("v1", FIXTURE);
    const folder = "we\u001b[31mird";
    cpSync(`${s.ws.root}/hub/layer`, `${s.ws.root}/${folder}/layer`, { recursive: true });
    writeFileSync(
      `${s.ws.root}/repo/layer.yaml`,
      `stack:\n  - source: path:../personal\n  - source: ${JSON.stringify(`path:../${folder}/layer`)}\n  - source: path:./.layer\ntargets: [claude]\n`,
    );
    const build = await alz(s, ["build"]);
    expect(build.code, build.stderr).toBe(0);
    const prompt = answers("client-a", "client-a-api");
    const result = await allowAt(s, prompt.term);
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("we\\u{001B}[31mird");
    expect(hasControl(result.stdout + result.stderr)).toBe(false);
  });
});
