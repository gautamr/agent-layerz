import { type SpawnSyncOptions, spawn, spawnSync } from "node:child_process";
import { closeSync, mkdtempSync, openSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpRoot } from "./run-root";

/**
 * Every child the tests start goes through this module: alz (`spawnCaptured`) and the tests' own
 * git, sh and mkfifo (`tool`). Each runs in its own process group with a timeout below the test
 * timeout, and a child that outlives it fails the test with its argv, the elapsed time, its stderr
 * and a listing of the group, so a stall names what it waits on instead of timing out silently.
 */

export const DEFAULT_TIMEOUT_MS = 15_000;

export interface CliResult {
  code: number;
  stdout: string;
  stderr: string;
}

export interface SpawnOptions {
  cwd: string;
  /** Exactly this environment, plus TMPDIR for `tool`. */
  env: Record<string, string>;
  /** Text for the child's standard input; without it the child gets none (the null device). */
  input?: string;
  /** Raise it for a case that legitimately takes longer. */
  timeoutMs?: number;
}

const PS_COLUMNS = "pid,ppid,pgid,stat,etime,command";

/** The header and the lines of a `ps -ax -o pgid...` listing that belong to one process group. Pure. */
export function groupListing(ps: string, pgid: number): string {
  const [header = "", ...rows] = ps.split("\n");
  const own = rows.filter((row) => row.trim().split(/\s+/)[2] === String(pgid));
  return [header, ...own].join("\n");
}

const describeArgv = (file: string, args: readonly string[]): string =>
  JSON.stringify([file, ...args]);

/** The failure message of a child that outlived its timeout. Pure. */
export function timeoutMessage(
  file: string,
  args: readonly string[],
  elapsedMs: number,
  stderr: string,
  listing: string,
): string {
  return [
    `${describeArgv(file, args)} did not end within ${elapsedMs} ms and was killed`,
    `stderr so far: ${JSON.stringify(stderr)}`,
    `process group (ps -o ${PS_COLUMNS}):`,
    listing,
  ].join("\n");
}

function listGroup(pgid: number): string {
  const ps = spawnSync("ps", ["-ax", "-o", PS_COLUMNS], { encoding: "utf8", timeout: 5000 });
  return ps.status === 0 ? groupListing(ps.stdout, pgid) : `ps failed: ${ps.stderr ?? ps.error}`;
}

/**
 * Is this the pid of a child that really started? Node reports `pid: 0` (or no pid) after a failed
 * start, and the group ids 0 and 1 name our own group and every process: never signal those.
 */
export const isStartedPid = (pid: number | undefined): pid is number =>
  pid !== undefined && Number.isInteger(pid) && pid > 1;

/** Kills a process group; refuses anything that is not the pid of a started child (see `isStartedPid`). */
export function killGroup(pgid: number | undefined): void {
  if (!isStartedPid(pgid)) return;
  try {
    process.kill(-pgid, "SIGKILL");
  } catch {
    // The group is already gone.
  }
}

/** Runs a binary and captures its output. Rejects when it outlives the timeout or ends on a signal. */
export function spawnCaptured(
  file: string,
  args: readonly string[],
  options: SpawnOptions,
): Promise<CliResult> {
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const child = spawn(file, args, {
      cwd: options.cwd,
      env: options.env,
      stdio: [options.input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
      detached: true,
    });
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    const text = (chunks: Buffer[]) => Buffer.concat(chunks).toString("utf8");
    child.stdout?.on("data", (chunk: Buffer) => out.push(chunk));
    child.stderr?.on("data", (chunk: Buffer) => err.push(chunk));
    if (options.input !== undefined && child.stdin !== null) {
      child.stdin.on("error", () => {
        // The child ended without reading its input: its exit code says why.
      });
      child.stdin.end(options.input);
    }
    const pgid = child.pid;
    const timer = setTimeout(() => {
      const listing = isStartedPid(pgid) ? listGroup(pgid) : "(no pid)";
      killGroup(pgid);
      reject(new Error(timeoutMessage(file, args, Date.now() - started, text(err), listing)));
    }, timeoutMs);
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(new Error(`${describeArgv(file, args)} could not start: ${error.message}`));
    });
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      killGroup(pgid);
      if (code === null) {
        reject(
          new Error(
            `${describeArgv(file, args)} ended on signal ${signal} after ${Date.now() - started} ms\nstderr: ${JSON.stringify(text(err))}`,
          ),
        );
        return;
      }
      resolve({ code, stdout: text(out), stderr: text(err) });
    });
  });
}

export interface ToolResult {
  status: number;
  stdout: string;
  stderr: string;
}

/**
 * Runs one of the tests' own tools (git, sh, mkfifo, the install script) to the end. The
 * environment is PATH, then the one given, then TMPDIR under the run root. A signal, a failure to
 * start and the timeout throw with the argv and the child's stderr; on the timeout the message
 * also holds the listing of what is left in the group, taken before the group is killed. A
 * non-zero exit is a result, for the cases that expect one.
 *
 * The child's output goes to files under the run root, not to pipes: `spawnSync` waits for the
 * pipes to close, and a grandchild that keeps them open (a background `sleep`) would hold the call
 * long after the timeout killed the direct child. With files, the call returns when the child ends.
 * The group is then listed and killed, so nothing the child left behind survives.
 */
export function capture(
  file: string,
  args: readonly string[],
  options: Partial<SpawnOptions> = {},
): ToolResult {
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const started = Date.now();
  const scratch = mkdtempSync(join(tmpRoot(), "capture-"));
  const outPath = join(scratch, "stdout");
  const errPath = join(scratch, "stderr");
  const outFd = openSync(outPath, "w");
  const errFd = openSync(errPath, "w");
  try {
    // `detached` makes the child a process-group leader, which Node honours in spawnSync although
    // its type lacks the option.
    const spawnOptions: SpawnSyncOptions & { detached: true } = {
      cwd: options.cwd,
      env: { PATH: process.env.PATH ?? "", ...options.env, TMPDIR: tmpRoot() },
      input: options.input,
      stdio: [options.input === undefined ? "ignore" : "pipe", outFd, errFd],
      detached: true,
      timeout: timeoutMs,
      killSignal: "SIGKILL",
    };
    const result = spawnSync(file, args, spawnOptions);
    const timedOut = (result.error as NodeJS.ErrnoException | undefined)?.code === "ETIMEDOUT";
    const listing = timedOut && isStartedPid(result.pid) ? listGroup(result.pid) : "";
    killGroup(result.pid);
    const stdout = readFileSync(outPath, "utf8");
    const stderr = readFileSync(errPath, "utf8");
    if (timedOut) {
      throw new Error(timeoutMessage(file, args, Date.now() - started, stderr, listing));
    }
    if (result.error) {
      throw new Error(`${describeArgv(file, args)} failed: ${result.error.message}`);
    }
    if (result.status === null) {
      throw new Error(
        `${describeArgv(file, args)} ended on signal ${result.signal}\nstderr: ${JSON.stringify(stderr)}`,
      );
    }
    return { status: result.status, stdout, stderr };
  } finally {
    closeSync(outFd);
    closeSync(errFd);
    rmSync(scratch, { recursive: true, force: true });
  }
}

/** `capture`, for a tool that must succeed: returns its standard output, throws on a non-zero exit. */
export function tool(
  file: string,
  args: readonly string[],
  options: Partial<SpawnOptions> = {},
): string {
  const result = capture(file, args, options);
  if (result.status !== 0) {
    throw new Error(
      `${describeArgv(file, args)} ended with status ${result.status}\nstderr: ${JSON.stringify(result.stderr)}`,
    );
  }
  return result.stdout;
}
