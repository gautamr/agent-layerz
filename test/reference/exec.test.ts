import { describe, expect, it } from "vitest";
import { capture, groupListing, spawnCaptured, timeoutMessage, tool } from "../exec";
import { tempDir } from "../run-root";

// The spawn helpers of the acceptance suite (TASK-55.2.7, TASK-54 AC 2): a child that outlives its
// timeout fails the test at that timeout, naming the command and what is running.

const PS = [
  "  PID  PPID  PGID STAT ELAPSED COMMAND",
  "  100     1   100 Ss   00:05 /bin/sh -c work",
  "  101   100   100 S    00:04 git fetch origin",
  "  200     1   200 S    00:09 unrelated",
].join("\n");

describe("the listing of a stuck process group", () => {
  it("keeps the header and exactly the rows of that group", () => {
    expect(groupListing(PS, 100).split("\n")).toEqual([
      "  PID  PPID  PGID STAT ELAPSED COMMAND",
      "  100     1   100 Ss   00:05 /bin/sh -c work",
      "  101   100   100 S    00:04 git fetch origin",
    ]);
  });

  it("names the argv, the elapsed time, the stderr and the listing", () => {
    const message = timeoutMessage("git", ["fetch", "x y"], 1234, "waiting", "LISTING");
    expect(message).toContain('["git","fetch","x y"]');
    expect(message).toContain("1234 ms");
    expect(message).toContain('"waiting"');
    expect(message).toContain("pid,ppid,pgid,stat,etime,command");
    expect(message).toContain("LISTING");
  });
});

describe("a spawned child that outlives its timeout", () => {
  it("fails within the timeout, with the argv and the process listing", async () => {
    const started = Date.now();
    const failure = await spawnCaptured("sleep", ["30"], {
      cwd: tempDir("alz-exec-"),
      env: { PATH: process.env.PATH ?? "" },
      timeoutMs: 1000,
    }).then(
      () => undefined,
      (error: Error) => error,
    );
    expect(Date.now() - started).toBeLessThan(5000);
    expect(failure?.message).toContain('["sleep","30"]');
    expect(failure?.message).toMatch(/PID +PPID +PGID +STAT +ELAPSED +COMMAND/i);
    expect(failure?.message).toContain("sleep 30");
  });

  it("is killed with its whole group, so no grandchild is left running", async () => {
    const failure = await spawnCaptured("sh", ["-c", "sleep 31 & wait"], {
      cwd: tempDir("alz-exec-"),
      env: { PATH: process.env.PATH ?? "" },
      timeoutMs: 1000,
    }).then(
      () => undefined,
      (error: Error) => error,
    );
    expect(failure?.message).toContain("sleep 31");
    // The kernel takes a moment to reap what the kill ended: look for up to two seconds.
    const survivors = async (tries: number): Promise<string[]> => {
      const rows = tool("ps", ["-ax", "-o", "command"])
        .split("\n")
        .filter((row) => row.includes("sleep 31"));
      if (rows.length === 0 || tries === 0) return rows;
      await new Promise((done) => setTimeout(done, 100));
      return survivors(tries - 1);
    };
    expect(await survivors(20)).toEqual([]);
  });

  it("fails a blocking tool call the same way", () => {
    expect(() =>
      capture("sleep", ["30"], { cwd: tempDir("alz-exec-"), env: {}, timeoutMs: 500 }),
    ).toThrow(/\["sleep","30"\] did not end within/);
  });
});

describe("a child that ends on a signal", () => {
  it("fails clearly instead of returning a code", async () => {
    await expect(
      spawnCaptured("sh", ["-c", "kill -9 $$"], {
        cwd: tempDir("alz-exec-"),
        env: { PATH: process.env.PATH ?? "" },
      }),
    ).rejects.toThrow(/ended on signal SIGKILL/);
  });
});

describe("a child that ends normally", () => {
  it("returns its code and both streams as text", async () => {
    const result = await spawnCaptured("sh", ["-c", "printf out; printf err >&2; exit 3"], {
      cwd: tempDir("alz-exec-"),
      env: { PATH: process.env.PATH ?? "" },
    });
    expect(result).toEqual({ code: 3, stdout: "out", stderr: "err" });
  });

  it("gets the run root's tmp folder as TMPDIR from the blocking helper", () => {
    expect(tool("sh", ["-c", 'printf %s "$TMPDIR"'], { cwd: tempDir("alz-exec-") })).toMatch(
      /alz-suite-[^/]+\/tmp$/,
    );
  });
});
