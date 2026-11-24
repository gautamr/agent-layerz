import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { capture } from "../exec";
import { tempDir } from "../run-root";

/**
 * The CI check of the acceptance job against the Rust binary (test/ci/check-r6-pending.ts) is what
 * keeps that job blocking, so it is tested with small reports: it must fail on a file that did not
 * load, on a file outside test/acceptance/, on a count under the floor, on a regression and on a
 * stale R6 entry, and pass on a clean report.
 */

const SCRIPT = join(process.cwd(), "test/ci/check-r6-pending.ts");
const FILE = "/work/repo/test/acceptance/01-x.test.ts";

interface Case {
  fullName: string;
  status: "passed" | "failed" | "skipped";
  ancestorTitles?: string[];
}
interface File {
  name: string;
  status: "passed" | "failed";
  assertionResults: Case[];
}

const passed = (fullName: string): Case => ({ fullName, status: "passed" });
const failed = (fullName: string, ancestorTitles: string[] = []): Case => ({
  fullName,
  status: "failed",
  ancestorTitles,
});
const file = (name: string, cases: Case[], status?: "passed" | "failed"): File => ({
  name,
  status: status ?? (cases.some((c) => c.status === "failed") ? "failed" : "passed"),
  assertionResults: cases,
});

/** Runs the check over a report, an R6 list and a floor; returns its exit status and stderr. */
function check(
  testResults: File[],
  options: { pending?: Record<string, { test: string; reason: string }[]>; floor?: number } = {},
  extra: Record<string, unknown> = {},
): { status: number; stdout: string; stderr: string } {
  const dir = tempDir("r6-check-");
  const lists = join(dir, "lists");
  mkdirSync(lists);
  writeFileSync(join(lists, "r6-pending.json"), JSON.stringify(options.pending ?? {}));
  writeFileSync(join(lists, "case-floor.json"), JSON.stringify({ floor: options.floor ?? 1 }));
  const report = join(dir, "report.json");
  // As vitest counts: each failed file, and each describe block that holds a failed case.
  const describes = new Set(
    testResults.flatMap((f) =>
      f.assertionResults
        .filter((c) => c.status === "failed")
        .flatMap((c) =>
          (c.ancestorTitles ?? []).map(
            (_, i) => `${f.name}|${c.ancestorTitles?.slice(0, i + 1).join(">")}`,
          ),
        ),
    ),
  );
  const failedSuites = testResults.filter((f) => f.status === "failed").length + describes.size;
  writeFileSync(
    report,
    JSON.stringify({ numFailedTestSuites: failedSuites, testResults, ...extra }),
  );
  return capture("node", ["--experimental-strip-types", SCRIPT, report, lists]);
}

describe("the CI check of the acceptance job against the Rust binary", () => {
  it("passes a report whose cases all pass and reach the floor", () => {
    const result = check([file(FILE, [passed("a"), passed("b")])], { floor: 2 });
    expect(result.status).toBe(0);
    expect(result.stdout).toContain("2 cases");
  });

  it("fails a test file that did not load: failed with no failed case", () => {
    const result = check([
      file(FILE, [passed("a")]),
      file("/work/repo/test/acceptance/02-y.test.ts", [], "failed"),
    ]);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("02-y.test.ts");
  });

  it("fails when the report counts a failed test suite", () => {
    const result = check([file(FILE, [passed("a")])], {}, { numFailedTestSuites: 1 });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("failed test suite");
  });

  it("counts a describe block that holds a failed case as explained, not as a load failure", () => {
    const pending = { "test/acceptance/01-x.test.ts": [{ test: "b", reason: "R6" }] };
    const result = check([file(FILE, [passed("a"), failed("b", ["group", "inner"])])], { pending });
    expect(result.status).toBe(0);
  });

  it("fails a file whose name does not hold test/acceptance/", () => {
    const result = check([
      file(FILE, [passed("a")]),
      file("/work/repo/test/reference/z.test.ts", [passed("b")]),
    ]);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("test/reference/z.test.ts");
  });

  it("fails when fewer cases ran than the floor", () => {
    const result = check([file(FILE, [passed("a"), passed("b")])], { floor: 3 });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("floor");
  });

  it("fails a failing case the R6 list does not name", () => {
    const result = check([file(FILE, [passed("a"), failed("b")])]);
    expect(result.status).toBe(1);
    expect(result.stderr).toContain("01-x.test.ts::b");
  });

  it("passes a failing case the R6 list names, and fails one that passes now", () => {
    const pending = { "test/acceptance/01-x.test.ts": [{ test: "b", reason: "R6" }] };
    expect(check([file(FILE, [passed("a"), failed("b")])], { pending }).status).toBe(0);
    const stale = check([file(FILE, [passed("a"), passed("b")])], { pending });
    expect(stale.status).toBe(1);
    expect(stale.stderr).toContain("remove them");
  });
});
