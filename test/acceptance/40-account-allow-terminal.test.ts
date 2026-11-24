import {
  cpSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { describe, expect, it, vi } from "vitest";
import YAML from "yaml";
import { accountFolder, alz, hubEntry, stateDir, writeAllowed } from "../account-workspace";
import {
  addHubV3,
  allowed,
  allowedPath,
  answerEach,
  CLAUDE_ONLY,
  expectNoLeftovers,
  expectNoQuestion,
  HEADER,
  hasControl,
  modeOf,
  NAMES,
  onTerminal,
  readAllowed,
  ready,
  rebuildAt,
  removed,
  TERMINAL_CASE_MS,
  TERMINAL_SESSION_MS,
} from "../allow-terminal";
import { snapshot } from "../helpers";
import { CTRL_C } from "../pty";

vi.setConfig({ testTimeout: TERMINAL_CASE_MS });

/**
 * `alz account allow` for a person at a terminal, driven through a pseudo-terminal (R6.1,
 * TASK-55.7.1, R6.1b TASK-55.7.17): `pty-run` (crates/alz-test-support) gives the binary a
 * terminal as standard input and leaves standard output and standard error as pipes. These are
 * the cases test 32 could not run without a terminal; they keep their reference counterparts in
 * test/reference/32. The prompt is on standard error, and the cases wait for it, change files
 * while it is shown, and then answer, as a person would. Files 41 (what is refused) and 42 (the
 * hint on build and check) hold the rest.
 */

describe("§13.20 a pseudo-terminal is a terminal, and a plain spawn is not (positive control)", () => {
  it("asks under the driver and refuses without it", async () => {
    const s = await ready();
    const plain = await alz(s, ["account", "allow"]);
    expect(plain.code).toBe(1);
    expect(plain.stderr).toContain("standard input is not a terminal");
    expect(existsSync(stateDir(s))).toBe(false);

    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", NAMES);
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(result.stderr).not.toContain("standard input is not a terminal");
    expect(existsSync(allowedPath(s))).toBe(true);
  });
});

describe("§13.20 the prompt and what is written, through a terminal (AC 1, AC 2)", () => {
  it("shows the root, the accounts, the layer and the source, asks once per name on standard error, and writes the exact file", async () => {
    const s = await ready();
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", [" client-a\t", "client-a-api  "]);
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(`repository: ${s.real}/repo\n`);
    expect(result.stdout).toContain("claude: client-a");
    expect(result.stdout).toContain('declared by the layer "client", stack entry 2');
    expect(result.stdout).toContain(`revision: ${s.v1}`);
    expect(result.stderr).toContain(
      `Type client-a to allow it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    );
    expect(result.stderr).toContain(
      `Type client-a-api to allow it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    );
    expect(readAllowed(s)).toBe(
      `${HEADER}- root: ${s.real}/repo\n  account:\n    claude: client-a\n    codex: client-a-api\n  source: ${s.hubLayer}\n`,
    );
    expect(modeOf(allowedPath(s))).toBe("600");
    expect(modeOf(stateDir(s))).toBe("700");
  });

  it.each([
    ["y", ["y"]],
    ["an empty line", [""]],
    ["a wrong name", ["client-b"]],
    ["a name in another case", ["CLIENT-A"]],
    ["a name inside other text", ["client-a now"]],
    ["a wrong second answer", ["client-a", "y"]],
  ])("writes nothing for %s", async (_label, answers) => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", answers);
    const result = await session.done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not confirmed; nothing changed");
    expect(snapshot(s.ws.home)).toEqual(before);
    expect(existsSync(stateDir(s))).toBe(false);
  });

  it("writes nothing when the input ends at the first question", async () => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const session = onTerminal(s, ["account", "allow"]);
    await session.waitForStderr("Type client-a to allow");
    session.endInput();
    const result = await session.done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not confirmed; nothing changed");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("answers no to Ctrl-C at the question: not confirmed, nothing changed, exit 1", async () => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const session = onTerminal(s, ["account", "allow"]);
    await session.waitForStderr("Type client-a to allow");
    session.type(CTRL_C);
    const result = await session.done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not confirmed; nothing changed");
    expect(snapshot(s.ws.home)).toEqual(before);
    expect(existsSync(stateDir(s))).toBe(false);
  });

  it("is already allowed when the entry is the same: exit 0, no question, the file untouched", async () => {
    const s = await ready();
    const first = onTerminal(s, ["account", "allow"]);
    await answerEach(first, s, "allow", NAMES);
    expect((await first.done()).code).toBe(0);
    const before = snapshot(s.ws.home);
    const inode = statSync(allowedPath(s)).ino;
    const second = onTerminal(s, ["account", "allow"]);
    const result = await second.done();
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("already allowed");
    expect(result.stderr).not.toContain("Type client-a");
    expect(statSync(allowedPath(s)).ino).toBe(inode);
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("asks one question for one name, whatever the number of targets", async () => {
    const s = await ready();
    addHubV3(s, "account: client-a\n");
    await rebuildAt(s, "v3");
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", ["client-a"], ["client-a"]);
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(result.stderr.split("Type client-a to allow").length - 1).toBe(1);
    expect(readAllowed(s)).toContain("claude: client-a\n    codex: client-a\n");
  });
});

describe("§13.20 the repository is read again after the confirmation, through a terminal", () => {
  it("refuses when the declaration changes while the person confirms", async () => {
    const s = await ready();
    addHubV3(s, "account:\n  claude: client-c\n  codex: client-a-api\n");
    const before = snapshot(s.ws.home);
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", NAMES, NAMES, () => rebuildAt(s, "v3"));
    const result = await session.done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("the declaration changed while you were confirming");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it.each([
    ["layer.yaml no longer parses", "stack: [\n"],
    [
      "the stack drops the declaring layer",
      "stack:\n  - source: path:../personal\n  - source: path:./.layer\ntargets: [claude]\n",
    ],
  ])("refuses when %s while the person confirms", async (_label, text) => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", NAMES, NAMES, () =>
      writeFileSync(`${s.real}/repo/layer.yaml`, text),
    );
    const result = await session.done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("the declaration changed while you were confirming");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("says nothing to remove when the entry went away while the person confirms", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s, { account: { claude: "client-a" } })]);
    const session = onTerminal(s, ["account", "allow", "--remove"]);
    await answerEach(session, s, "remove", ["client-a"], ["client-a"], () => {
      writeAllowed(s, []);
    });
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain("nothing to remove");
    expect(result.stdout).not.toContain("removed the entry");
  });
});

describe("§13.20 alz account allow --remove, through a terminal", () => {
  it("removes only this root's entry, keeps the others, and leaves [] after the last", async () => {
    const s = await ready();
    const other = { root: `${s.real}/repo-b`, account: { claude: "b-account" }, source: "/x/b" };
    writeAllowed(s, [other, hubEntry(s)]);
    const session = onTerminal(s, ["account", "allow", "--remove"]);
    await answerEach(session, s, "remove", NAMES);
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(result.stdout).toContain(
      "removing it sends this repository back to the default account",
    );
    expect(YAML.parse(readAllowed(s))).toEqual([other]);

    writeAllowed(s, [hubEntry(s)]);
    const last = onTerminal(s, ["account", "allow", "--remove"]);
    await answerEach(last, s, "remove", NAMES);
    expect((await last.done()).code).toBe(0);
    expect(readAllowed(s)).toBe(`${HEADER}[]\n`);
  });
});

describe("§13.20 the lock file, through twelve terminals", () => {
  it("is safe across real processes: twelve runs at once leave one entry for the root", async () => {
    const s = await ready();
    const sessions = Array.from({ length: 12 }, () =>
      onTerminal(s, ["account", "allow"], {}, "repo", 80_000),
    );
    // A run that reads allowed.yaml after another run wrote it says "already allowed" and asks
    // nothing, so each run either answers both prompts or ends with that line.
    const results = await Promise.all(
      sessions.map(async (session) => {
        try {
          await session.waitForStderr("Type client-a to allow");
        } catch {
          const ended = await session.done();
          expect(ended.stdout, ended.stderr).toContain("already allowed:");
          return { ...ended, prompted: false };
        }
        session.type("client-a\n");
        await session.waitForStderr("Type client-a-api to allow");
        session.type("client-a-api\n");
        return { ...(await session.done()), prompted: true };
      }),
    );
    for (const result of results) expect(result.code, result.stderr).toBe(0);
    expect(results.some((result) => result.prompted)).toBe(true);
    const entries = YAML.parse(readAllowed(s)) as { root: string }[];
    expect(entries.map((entry) => entry.root)).toEqual([`${s.real}/repo`]);
    expect(
      (await import("node:fs")).readdirSync(stateDir(s)).filter((name) => name !== "allowed.yaml"),
    ).toEqual([]);
  }, 90_000);
});

describe("§13.20 what is shown, asked and written, through a terminal (R6.1b)", () => {
  it("shows the root, the accounts, the layer, its source, ref and revision, and the recorded source", async () => {
    const s = await ready();
    accountFolder(s, ".claude-client-a");
    const result = await allowed(s);
    const text = result.stdout;
    expect(text).toContain(`repository: ${s.real}/repo\n`);
    expect(text).toContain("claude: client-a (the account folder exists)");
    expect(text).toContain("codex: client-a-api (the account folder does not exist yet)");
    expect(text).toContain('declared by the layer "client", stack entry 2');
    expect(text).toContain('source as written: "path:../hub/layer"');
    expect(text).toContain('ref: "v1"');
    expect(text).toContain(`revision: ${s.v1}`);
    expect(text).toContain(`recorded source: "${s.hubLayer}"`);
    expect(text).toContain("will use these accounts' logins and MCP connections");
    // Standard output only: on a terminal the line editor writes its own cursor controls to standard error.
    expect(hasControl(text)).toBe(false);
    expect(hasControl("a\u001b[31mb")).toBe(true);
  });

  it("writes private folders and a private file, leaves no lock or temporary file, and says what to do next", async () => {
    const s = await ready();
    const result = await allowed(s);
    expect(modeOf(allowedPath(s))).toBe("600");
    expect(modeOf(stateDir(s))).toBe("700");
    // The directories alz created on the way are private too.
    expect(modeOf(`${s.ws.home}/.local/state`)).toBe("700");
    expect(modeOf(`${s.ws.home}/.local`)).toBe("700");
    expectNoLeftovers(s);
    expect(result.stdout).toContain(
      `allowed: ${s.real}/repo uses claude=client-a, codex=client-a-api`,
    );
    expect(result.stdout).toContain("alz account init client-a");
  });

  it("writes nothing for yes", async () => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", ["yes"]);
    const result = await session.done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("not confirmed; nothing changed");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("reads two answers that arrive in one chunk, one for each question", async () => {
    const s = await ready();
    const session = onTerminal(s, ["account", "allow"]);
    await session.waitForStderr("Type client-a to allow");
    session.type("client-a\nclient-a-api\n");
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toContain("claude: client-a\n    codex: client-a-api\n");
  });

  it("records the real root when run from a subdirectory or from a symbolic link", async () => {
    const s = await ready();
    mkdirSync(`${s.real}/repo/sub/deeper`, { recursive: true });
    symlinkSync(`${s.real}/repo`, `${s.ws.root}/link-to-repo`);
    await allowed(s, NAMES, {}, "link-to-repo/sub/deeper");
    expect(readAllowed(s)).toContain(`- root: ${s.real}/repo\n`);
    expect(readAllowed(s)).not.toContain("link-to-repo");
  });

  it("replaces the entry for the same root and keeps every other one, sorted, unresolved roots included", async () => {
    const s = await ready();
    const other = { root: `${s.real}/repo-b`, account: { claude: "b-account" }, source: "/x/b" };
    const gone = {
      root: "/zzz/does/not/exist",
      account: { codex: "z" },
      source: "https://example.invalid/z.git",
    };
    const old = hubEntry(s, { account: { claude: "old-name" }, source: "/old/source" });
    writeAllowed(s, [gone, old, other]);
    const result = await allowed(s);
    const entries = YAML.parse(readAllowed(s)) as { root: string }[];
    expect(entries.map((entry) => entry.root)).toEqual(
      [`${s.real}/repo`, `${s.real}/repo-b`, "/zzz/does/not/exist"].sort(),
    );
    expect(entries.find((entry) => entry.root === `${s.real}/repo`)).toEqual(hubEntry(s));
    expect(entries.find((entry) => entry.root === other.root)).toEqual(other);
    expect(entries.find((entry) => entry.root === gone.root)).toEqual(gone);
    expect(result.stdout).toContain("allowed:");
    expectNoLeftovers(s);
  });

  it("records only the targets the declaration names", async () => {
    const s = await ready();
    addHubV3(s, CLAUDE_ONLY);
    await rebuildAt(s, "v3");
    await allowed(s, ["client-a"]);
    expect(readAllowed(s)).toBe(
      `${HEADER}- root: ${s.real}/repo\n  account:\n    claude: client-a\n  source: ${s.hubLayer}\n`,
    );
  });

  it("keeps a linked allowed.yaml a link and writes its target; a dangling link refuses", async () => {
    const s = await ready();
    const store = `${s.ws.home}/store`;
    mkdirSync(store, { mode: 0o700 });
    writeFileSync(`${store}/target.yaml`, "[]\n", { mode: 0o600 });
    mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
    symlinkSync(`${store}/target.yaml`, allowedPath(s));
    await allowed(s);
    expect(lstatSync(allowedPath(s)).isSymbolicLink()).toBe(true);
    expect(readFileSync(`${store}/target.yaml`, "utf8")).toContain(`- root: ${s.real}/repo\n`);
    expect(readdirSync(store)).toEqual(["target.yaml"]);

    const dangling = await ready();
    mkdirSync(stateDir(dangling), { recursive: true, mode: 0o700 });
    symlinkSync(`${dangling.ws.home}/nowhere.yaml`, allowedPath(dangling));
    const before = snapshot(dangling.ws.home);
    const result = await onTerminal(dangling, ["account", "allow"]).done();
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("it is a link that does not resolve");
    expectNoQuestion(result);
    expect(snapshot(dangling.ws.home)).toEqual(before);
  });
});

describe("§13.20 alz account allow --remove, the rest, through a terminal (R6.1b)", () => {
  it("shows the recorded entry, asks with the recorded source, and keeps the file private", async () => {
    const s = await ready();
    const other = { root: `${s.real}/repo-b`, account: { claude: "b-account" }, source: "/x/b" };
    writeAllowed(s, [other, hubEntry(s)]);
    const session = onTerminal(s, ["account", "allow", "--remove"]);
    await session.waitForStderr(
      `Type client-a to remove it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    );
    session.type("client-a\n");
    await session.waitForStderr(
      `Type client-a-api to remove it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    );
    session.type("client-a-api\n");
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s).startsWith(HEADER)).toBe(true);
    expect(modeOf(allowedPath(s))).toBe("600");
    expectNoLeftovers(s);

    // Only the other root's entry is left, so removing the repository's again changes nothing.
    const again = await onTerminal(s, ["account", "allow", "--remove"]).done();
    expect(again.code).toBe(0);
    expect(again.stdout).toContain("nothing to remove");
    expect(YAML.parse(readAllowed(s))).toEqual([other]);
  });

  it("does nothing, and creates nothing, when there is no file or no entry", async () => {
    const s = await ready();
    const before = snapshot(s.ws.home);
    const none = await onTerminal(s, ["account", "allow", "--remove"]).done();
    expect(none.code).toBe(0);
    expect(none.stdout).toContain("nothing to remove");
    expectNoQuestion(none);
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  it("asks for the recorded names, and a wrong answer changes nothing", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s, { account: { claude: "one-name" } })]);
    const before = snapshot(s.ws.home);
    const wrong = onTerminal(s, ["account", "allow", "--remove"]);
    await wrong.waitForStderr(
      `Type one-name to remove it for "${s.real}/repo" (source "${s.hubLayer}"): `,
    );
    wrong.type("client-a\n");
    const refused = await wrong.done();
    expect(refused.code).toBe(1);
    expect(snapshot(s.ws.home)).toEqual(before);
    const right = await removed(s, ["one-name"]);
    expect(right.code, right.stderr).toBe(0);
  });

  it("works on a cache miss", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s)]);
    rmSync(`${s.real}/repo/layer.lock`);
    const result = await removed(s);
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toBe(`${HEADER}[]\n`);
  });
});

describe("§13.20 the lock file, the rest, through a terminal (R6.1b)", () => {
  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

  it("waits for a lock that goes away, then succeeds", async () => {
    const s = await ready();
    writeAllowed(s, []);
    const lock = `${allowedPath(s)}.lock`;
    writeFileSync(lock, "999999\n", { mode: 0o600 });
    const session = onTerminal(s, ["account", "allow"]);
    await answerEach(session, s, "allow", NAMES);
    await sleep(300);
    expect(existsSync(lock)).toBe(true);
    rmSync(lock);
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(readAllowed(s)).toContain(`- root: ${s.real}/repo\n`);
    expect(existsSync(lock)).toBe(false);
  });

  it(
    "never removes a lock it did not create: it refuses, names it and leaves it",
    async () => {
      const s = await ready();
      writeAllowed(s, []);
      const lock = `${allowedPath(s)}.lock`;
      writeFileSync(lock, "999999\n", { mode: 0o600 });
      const before = snapshot(s.ws.home);
      const session = onTerminal(s, ["account", "allow"], {}, "repo", TERMINAL_SESSION_MS);
      await answerEach(session, s, "allow", NAMES);
      const result = await session.done();
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("allowed.yaml.lock");
      expect(result.stderr).toContain("delete that file");
      expect(snapshot(s.ws.home)).toEqual(before);
    },
    2 * TERMINAL_CASE_MS,
  );

  it(
    "does not follow a lock that is a link",
    async () => {
      const s = await ready();
      writeAllowed(s, []);
      const victim = `${s.ws.home}/victim`;
      writeFileSync(victim, "keep", { mode: 0o600 });
      symlinkSync(victim, `${allowedPath(s)}.lock`);
      const session = onTerminal(s, ["account", "allow"], {}, "repo", TERMINAL_SESSION_MS);
      await answerEach(session, s, "allow", NAMES);
      const result = await session.done();
      expect(result.code).toBe(1);
      expect(readFileSync(victim, "utf8")).toBe("keep");
      expect(lstatSync(`${allowedPath(s)}.lock`).isSymbolicLink()).toBe(true);
    },
    2 * TERMINAL_CASE_MS,
  );

  it("is safe across real processes: eight distinct roots and four runs on a shared one leave one sorted entry per root", async () => {
    const s = await ready();
    // Each root is a copy of the repository beside it, so the stack's relative sources still resolve.
    const names = [...Array.from({ length: 8 }, (_, index) => `r${index}`), "repo"];
    for (const name of names.slice(0, 8)) {
      cpSync(`${s.real}/repo`, `${s.real}/${name}`, { recursive: true });
    }
    const dirs = [...names.slice(0, 8), ...Array.from({ length: 4 }, () => "repo")];
    const sessions = dirs.map((dir) => onTerminal(s, ["account", "allow"], {}, dir, 80_000));
    // A run on the shared root that reads allowed.yaml after another run wrote it says "already
    // allowed" and asks nothing, so each run either answers both questions or ends with that line.
    const results = await Promise.all(
      sessions.map(async (session) => {
        try {
          await session.waitForStderr("Type client-a to allow");
        } catch {
          const ended = await session.done();
          expect(ended.stdout, ended.stderr).toContain("already allowed");
          return { ...ended, prompted: false };
        }
        session.type("client-a\n");
        await session.waitForStderr("Type client-a-api to allow");
        session.type("client-a-api\n");
        return { ...(await session.done()), prompted: true };
      }),
    );
    for (const result of results) expect(result.code, result.stderr).toBe(0);
    expect(results.filter((result) => result.prompted).length).toBeGreaterThanOrEqual(9);
    const entries = YAML.parse(readAllowed(s)) as { root: string }[];
    const roots = entries.map((entry) => entry.root);
    expect(roots).toHaveLength(9);
    expect(roots).toEqual([...roots].sort());
    expect(new Set(roots).size).toBe(9);
    expect(roots).toEqual(names.map((name) => `${s.real}/${name}`).sort());
    expectNoLeftovers(s);
  }, 120_000);
});

describe("§13.20 a repository root with line and paragraph separators, through a terminal (TASK-73)", () => {
  // U+0085, U+2028 and U+2029 are control characters that a YAML reader may take for line ends:
  // the root is recorded as it is, read back as the same root, and the layer's account applies.
  it("allows a repository whose real root holds U+0085, U+2028 and U+2029, and then the layer's account applies", async () => {
    const s = await ready();
    accountFolder(s, ".claude-client-a", ".codex-client-a-api");
    const folder = "re\u0085po\u2028x\u2029y";
    cpSync(`${s.real}/repo`, `${s.real}/${folder}`, { recursive: true });
    const session = onTerminal(s, ["account", "allow"], {}, folder);
    for (const name of NAMES) {
      await session.waitForStderr(`Type ${name} to allow`);
      session.type(`${name}\n`);
    }
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    // The prompt escapes what the person decides on; the offer's root line only strips it.
    expect(result.stderr).toContain("re\\u{0085}po\\u{2028}x\\u{2029}y");
    const entries = YAML.parse(readAllowed(s)) as { root: string }[];
    expect(entries.map((entry) => entry.root)).toEqual([`${s.real}/${folder}`]);
    const which = await alz(s, ["account", "which"], {}, folder);
    expect(which.code, which.stderr).toBe(0);
    expect(which.stdout).toContain("claude: client-a (source: layer)");
    expect(which.stdout).toContain("codex: client-a-api (source: layer)");
    expect(which.stdout).toContain("(read; this repository is allowed)");
  });
});
