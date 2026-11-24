/**
 * The acceptance job against the Rust binary is blocking, except for the cases R6 (TASK-55.7)
 * delivers: accounts and commit identity. They are listed in `r6-pending.json`, each with its
 * reason. This script reads the JSON report of a vitest run and exits 1 when
 *   - a case fails that the list does not name (a regression), or
 *   - a case the list names passes (the marker is stale: delete the entry).
 * It also fails when the report cannot be trusted to hold every case:
 *   - a test file failed to load or collect (status "failed" with no failed case), or the report
 *     counts more failed suites than the files and describe blocks that hold a failed case,
 *   - a file name does not contain `test/acceptance/`, or
 *   - fewer cases ran than the floor in `case-floor.json` (raise the floor when cases are added).
 * Run it as `node --experimental-strip-types test/ci/check-r6-pending.ts <report.json> [lists dir]`;
 * the lists directory (default: this folder) holds `r6-pending.json` and `case-floor.json`.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

interface Pending {
  readonly test: string;
  readonly reason: string;
}

interface Report {
  readonly numFailedTestSuites?: number;
  readonly testResults: ReadonlyArray<{
    readonly name: string;
    readonly status?: string;
    readonly assertionResults: ReadonlyArray<{
      readonly fullName: string;
      readonly status: string;
      readonly ancestorTitles?: readonly string[];
    }>;
  }>;
}

const ACCEPTANCE = "test/acceptance/";

const reportPath = process.argv[2];
if (reportPath === undefined) {
  process.stderr.write("usage: check-r6-pending.ts <vitest json report> [lists dir]\n");
  process.exit(2);
}
const lists = process.argv[3] ?? fileURLToPath(new URL(".", import.meta.url));

const readJson = <T>(path: string): T => JSON.parse(readFileSync(path, "utf8")) as T;
const pending = readJson<Record<string, readonly Pending[]>>(join(lists, "r6-pending.json"));
const { floor } = readJson<{ floor: number }>(join(lists, "case-floor.json"));
const report = readJson<Report>(reportPath);

const key = (file: string, test: string): string => `${file}::${test}`;
const listed = new Set(
  Object.entries(pending).flatMap(([file, rows]) => rows.map((row) => key(file, row.test))),
);

const relativeName = (name: string): string | undefined => {
  const at = name.indexOf(ACCEPTANCE);
  return at < 0 ? undefined : name.slice(at);
};

const files = report.testResults.map((file) => ({
  file,
  relative: relativeName(file.name),
  hasFailedCase: file.assertionResults.some((r) => r.status === "failed"),
}));

const outside = files.filter((f) => f.relative === undefined).map((f) => f.file.name);
// A file that failed with no failed case did not load or collect: its cases never ran.
const notLoaded = files
  .filter((f) => f.file.status === "failed" && !f.hasFailedCase)
  .map((f) => f.file.name);
// vitest counts a failed file and each describe block that holds a failed case; a failed suite
// beyond those is a suite that failed in a hook or while it was collected.
const failedSuites = report.numFailedTestSuites ?? 0;
const describesWithFailedCase = new Set(
  files.flatMap(({ file }) =>
    file.assertionResults
      .filter((result) => result.status === "failed")
      .flatMap((result) => {
        const titles = result.ancestorTitles ?? [];
        return titles.map((_, depth) => `${file.name}|${titles.slice(0, depth + 1).join(">")}`);
      }),
  ),
).size;
const explainedSuites = files.filter((f) => f.hasFailedCase).length + describesWithFailedCase;
const suiteFailures = failedSuites > explainedSuites ? failedSuites : 0;

const results = files.flatMap(({ file, relative }) =>
  file.assertionResults.map((result) => ({
    id: key(relative ?? file.name, result.fullName),
    status: result.status,
  })),
);

const unexpected = results.filter((r) => r.status === "failed" && !listed.has(r.id));
const stale = results.filter((r) => r.status === "passed" && listed.has(r.id));
const ran = new Set(results.map((r) => r.id));
const missing = [...listed].filter((id) => !ran.has(id));
const belowFloor = results.length < floor;

const show = (title: string, rows: readonly string[]): void => {
  if (rows.length > 0) process.stderr.write(`${title} (${rows.length}):\n  ${rows.join("\n  ")}\n`);
};
show("test files that failed to load or collect", notLoaded);
show(`files that are not under ${ACCEPTANCE}`, outside);
if (suiteFailures > 0) {
  process.stderr.write(
    `the report counts ${failedSuites} failed test suites, more than the ${explainedSuites} that hold a failed case: a suite failed in a hook or while it was collected\n`,
  );
}
if (belowFloor) {
  process.stderr.write(
    `only ${results.length} cases ran, below the floor of ${floor} in test/ci/case-floor.json: a file did not run, or cases were removed; lower the floor only on purpose\n`,
  );
}
show(
  "failing cases that are not R6's",
  unexpected.map((r) => r.id),
);
show(
  "R6 cases that pass now; remove them from test/ci/r6-pending.json",
  stale.map((r) => r.id),
);
show("listed cases that did not run; fix or remove the entry", missing);

const failed = results.filter((r) => r.status === "failed").length;
process.stdout.write(
  `acceptance: ${results.length} cases (floor ${floor}), ${failed} failing, ${listed.size} listed as R6 pending\n`,
);
const problems =
  unexpected.length +
  stale.length +
  missing.length +
  notLoaded.length +
  outside.length +
  suiteFailures +
  (belowFloor ? 1 : 0);
process.exit(problems === 0 ? 0 : 1);
