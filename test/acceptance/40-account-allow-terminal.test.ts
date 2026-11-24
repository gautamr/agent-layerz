import { existsSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import YAML from "yaml";
import {
  alz,
  built,
  envFor,
  hubEntry,
  PERSONAL,
  REPO_LAYER,
  type Setup,
  sh,
  stateDir,
  useStack,
  writeAllowed,
} from "../account-workspace";
import { snapshot } from "../helpers";
import { CTRL_C, type PtySession, startOnPty } from "../pty";
import { alzBin } from "../run-root";

/**
 * `alz account allow` for a person at a terminal, driven through a pseudo-terminal (R6.1,
 * TASK-55.7.1): `pty-run` (crates/alz-test-support) gives the binary a terminal as standard input
 * and leaves standard output and standard error as pipes. These are the cases test 32 could not
 * run without a terminal; they keep their reference counterparts in test/reference/32 until the
 * rest of them move here. The prompt is on standard error, and the cases wait for it, change
 * files while it is shown, and then answer, as a person would.
 */

const FIXTURE = "32-account-allow";
const HEADER = "# written by alz account allow; deleting the file withdraws every approval\n";
const NAMES = ["client-a", "client-a-api"];
const ready = () => built("v1", FIXTURE);

/** `alz <args>` on a pseudo-terminal, with the environment `layers` gives. */
function onTerminal(
  s: Setup,
  args: string[],
  extra: Record<string, string> = {},
  dir = "repo",
  timeoutMs?: number,
): PtySession {
  return startOnPty(alzBin(), args, {
    cwd: join(s.ws.root, dir),
    env: {
      HOME: s.ws.home,
      PATH: process.env.PATH ?? "",
      ALZ_CLAUDE_BIN: "/nonexistent",
      ...envFor(s, extra),
    },
    timeoutMs,
  });
}

/** Waits for the question about each name, in order, and types the answer given for it. */
async function answerEach(
  session: PtySession,
  s: Setup,
  verb: "allow" | "remove",
  answers: readonly string[],
  names: readonly string[] = NAMES,
  between: () => void | Promise<void> = () => undefined,
) {
  for (const [index, answer] of answers.entries()) {
    await session.waitForStderr(`Type ${names[index]} to ${verb} it for "${s.real}/repo"`);
    if (index === 0) await between();
    session.type(`${answer}\n`);
  }
}

const allowedPath = (s: Setup) => `${stateDir(s)}/allowed.yaml`;
const readAllowed = (s: Setup) => readFileSync(allowedPath(s), "utf8");
const modeOf = (path: string) => (statSync(path).mode & 0o777).toString(8);

function addHubV3(s: Setup, account: string) {
  const head = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\n";
  writeFileSync(`${s.hub}/layer/layer.yaml`, `${head}${account}`);
  sh(s.ws, s.hub, ["add", "."]);
  sh(s.ws, s.hub, ["commit", "--quiet", "-m", "hub v3"]);
  sh(s.ws, s.hub, ["tag", "v3"]);
}

async function rebuildAt(s: Setup, ref: string) {
  useStack(s, PERSONAL, ["path:../hub/layer", ref], REPO_LAYER);
  const result = await alz(s, ["build"]);
  expect(result.code, result.stderr).toBe(0);
}

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
