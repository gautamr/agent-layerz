import { appendFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
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
