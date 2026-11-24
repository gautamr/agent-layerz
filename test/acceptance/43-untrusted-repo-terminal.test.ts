import { appendFileSync, cpSync, existsSync, symlinkSync, writeFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import { alz } from "../account-workspace";
import { atTerminal, hasControl, onTerminal, ready, TERMINAL_CASE_MS } from "../allow-terminal";
import { referenceDrift } from "../drift";

vi.setConfig({ testTimeout: TERMINAL_CASE_MS });

/**
 * `alz account allow` for a person at a terminal, when the repository is not to be trusted
 * (spec 7.13 "Who may declare it", TASK-50.11, R1.8 TASK-55.7.1): what it prints before it asks is
 * the reader's own text, quoted, and a declaration that may not apply is never offered. These are
 * the terminal cases of file 24 that stayed in test/reference/24 until the pty driver existed.
 */
describe("alz account allow with a repository that is not to be trusted, through a terminal", () => {
  it("refuses a declaration the repository's own layer makes, and warns that it is ignored", async () => {
    const s = await ready();
    // Only the repository's layer declares: the stack has no client layer.
    writeFileSync(
      `${s.real}/repo/layer.yaml`,
      "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
    );
    expect((await alz(s, ["build"])).code).toBe(0);
    appendFileSync(`${s.real}/repo/.layer/layer.yaml`, "account: repo-account\n");
    const result = await atTerminal(s, ["account", "allow"]);
    expect(result.code, result.stderr).toBe(1);
    expect(result.stderr).toContain("the layers name no account here");
    expect(result.stderr).toContain("warning [account-declaration-ignored]");
    expect(result.stderr).toContain("it is inside the repository");
    expect(result.stderr).not.toContain("Type ");
    expect(existsSync(`${s.ws.home}/.local`)).toBe(false);
  });

  it("shows the source an external client layer names, quoted, before it asks", async () => {
    const s = await ready();
    const session = onTerminal(s, ["account", "allow"]);
    await session.waitForStderr("Type client-a to allow");
    session.endInput();
    const result = await session.done();
    expect(result.code, result.stderr).toBe(1);
    // Everything the person decides on is printed before the first question, and the prompt
    // repeats the source, because standard output may be redirected.
    expect(result.stdout).toContain('source as written: "path:../hub/layer"');
    expect(result.stdout).toContain(`recorded source: "${s.hubLayer}"`);
    expect(result.stderr).toContain(`(source "${s.hubLayer}")`);
  });

  // The reference records an ESC in the source and exits 0. Spec 5.2 and 7.13: alz account allow
  // writes a path with a C1 control character, U+2028 or U+2029, but refuses one with another
  // control character, as ESC (U+001B) is, so it writes nothing and exits 1.
  referenceDrift(
    "TASK-55.3",
    "prints no control character for a hub directory whose name holds ESC, and writes nothing",
    async () => {
      const s = await ready();
      const folder = "we\u001b[31mird";
      cpSync(`${s.ws.root}/hub/layer`, `${s.ws.root}/${folder}/layer`, { recursive: true });
      // layer.yaml may not hold a control character, so the repository reaches the folder by a
      // link with a plain name; the name with ESC is the real path of the layer.
      symlinkSync(`${s.ws.root}/${folder}`, `${s.ws.root}/hub-link`);
      writeFileSync(
        `${s.ws.root}/repo/layer.yaml`,
        "stack:\n  - source: path:../personal\n  - source: path:../hub-link/layer\n  - source: path:./.layer\ntargets: [claude]\n",
      );
      const build = await alz(s, ["build"]);
      expect(build.code, build.stderr).toBe(0);
      const session = onTerminal(s, ["account", "allow"]);
      await session.waitForStderr("Type client-a to allow");
      session.endInput();
      const result = await session.done();
      expect(result.code, result.stderr).toBe(1);
      expect(result.stdout + result.stderr).toContain("we\\u{001B}[31mird");
      expect(hasControl(result.stdout + result.stderr)).toBe(false);
      expect(existsSync(`${s.ws.home}/.local/state/agent-layerz/allowed.yaml`)).toBe(false);
    },
  );
});
