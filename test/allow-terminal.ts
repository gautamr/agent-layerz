import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect } from "vitest";
import {
  built,
  envFor,
  PERSONAL,
  REPO_LAYER,
  alz as run,
  type Setup,
  sh,
  stateDir,
  useStack,
} from "./account-workspace";
import type { CliResult } from "./exec";
import { type PtySession, startOnPty } from "./pty";
import { alzBin } from "./run-root";

/**
 * What the `alz account allow` cases that need a person at a terminal share (files 40 to 42): the
 * set-up of the fixture `32-account-allow`, the driver call with an allow-listed environment, and
 * the typing of answers. The prompt is on standard error; the cases wait for it, change files while
 * it is shown, and then answer, as a person would.
 */

/**
 * How long one terminal case may take, on a loaded machine too (vitest's 20 s default is not
 * enough there). A session's deadline is shorter, so the driver ends a stuck command with its own
 * message before vitest abandons the case. Each file sets it with `vi.setConfig`; a case that runs
 * several sessions or waits for the 10 s lock raises it explicitly.
 */
export const TERMINAL_CASE_MS = 60_000;
export const TERMINAL_SESSION_MS = 50_000;

export const FIXTURE = "32-account-allow";
export const HEADER =
  "# written by alz account allow; deleting the file withdraws every approval\n";
export const NAMES = ["client-a", "client-a-api"];
export const MAP_V3 = "account:\n  claude: client-c\n  codex: client-a-api\n";
export const CLAUDE_ONLY = "account:\n  claude: client-a\n";
export const ready = () => built("v1", FIXTURE);

/** The agent-session markers (spec 7.13), written out so a change to the list fails here. */
export const AGENT_SESSION_MARKERS = [
  "CLAUDECODE",
  "CLAUDE_CODE_CHILD_SESSION",
  "AI_AGENT",
  "CODEX_THREAD_ID",
  "CODEX_SESSION_ID",
  "CODEX_CI",
];

/** `alz <args>` on a pseudo-terminal, with the environment `layers` gives and nothing of the test's own. */
export function onTerminal(
  s: Setup,
  args: string[],
  extra: Record<string, string> = {},
  dir = "repo",
  timeoutMs = TERMINAL_SESSION_MS,
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

/** Runs a command at a terminal that is never asked anything, to its end. */
export const atTerminal = (
  s: Setup,
  args: string[],
  extra: Record<string, string> = {},
  dir = "repo",
): Promise<CliResult> => onTerminal(s, args, extra, dir).done();

/** Waits for the question about each name, in order, and types the answer given for it. */
export async function answerEach(
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

/** Runs allow to the end with the right answers, from `dir`. */
export async function allowed(
  s: Setup,
  names: readonly string[] = NAMES,
  extra: Record<string, string> = {},
  dir = "repo",
): Promise<CliResult> {
  const session = onTerminal(s, ["account", "allow"], extra, dir);
  await answerEach(session, s, "allow", names, names);
  const result = await session.done();
  expect(result.code, result.stderr).toBe(0);
  return result;
}

/** Runs `allow --remove` to the end with the given answers. */
export async function removed(
  s: Setup,
  names: readonly string[] = NAMES,
  extra: Record<string, string> = {},
): Promise<CliResult> {
  const session = onTerminal(s, ["account", "allow", "--remove"], extra);
  await answerEach(session, s, "remove", names, names);
  return session.done();
}

/** Nothing was asked: no confirmation question appears in what the command wrote. */
export function expectNoQuestion(result: CliResult): void {
  expect(result.stderr + result.stdout).not.toMatch(/Type \S+ to (allow|remove) it/);
}

/** `alz account which` without a terminal. */
export const which = (s: Setup, extra: Record<string, string> = {}) =>
  run(s, ["account", "which"], extra);

export const allowedPath = (s: Setup) => `${stateDir(s)}/allowed.yaml`;
export const readAllowed = (s: Setup) => readFileSync(allowedPath(s), "utf8");
export const modeOf = (path: string) => (statSync(path).mode & 0o777).toString(8);

/** The state directory holds nothing but allowed.yaml: no lock, no temporary file. */
export function expectNoLeftovers(s: Setup) {
  expect(readdirSync(stateDir(s)).filter((name) => name !== "allowed.yaml")).toEqual([]);
}

/** Any C0, DEL or C1 character other than a newline. */
export const hasControl = (text: string): boolean =>
  [...text].some((char) => {
    const code = char.codePointAt(0) ?? 0;
    return code !== 0x0a && (code <= 0x1f || (code >= 0x7f && code <= 0x9f));
  });

/** The `note [account-allow]` lines of a command's standard error. */
export const hintLines = (text: string) =>
  text.split("\n").filter((line) => line.startsWith("note [account-allow]"));

export function addHubV3(s: Setup, account: string) {
  const head = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\n";
  writeFileSync(`${s.hub}/layer/layer.yaml`, `${head}${account}`);
  sh(s.ws, s.hub, ["add", "."]);
  sh(s.ws, s.hub, ["commit", "--quiet", "-m", "hub v3"]);
  sh(s.ws, s.hub, ["tag", "v3"]);
}

export async function rebuildAt(s: Setup, ref: string) {
  useStack(s, PERSONAL, ["path:../hub/layer", ref], REPO_LAYER);
  const result = await run(s, ["build"]);
  expect(result.code, result.stderr).toBe(0);
}
