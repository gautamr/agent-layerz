/**
 * The acceptance job against the Rust binary is blocking: this script reads the JSON report of
 * a vitest run and exits 1 when
 *   - any case fails, or
 *   - a test file failed to load or collect (status "failed" with no failed case), or the report
 *     counts more failed suites than the files and describe blocks that hold a failed case, or
 *   - a file name does not contain `test/acceptance/`, or
 *   - fewer cases ran than the floor in `case-floor.json` (raise the floor when cases are added).
 * Run it as `node --experimental-strip-types test/ci/check-acceptance-report.ts <report.json> [floor dir]`;
 * the floor directory (default: this folder) holds `case-floor.json`.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

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
  process.stderr.write("usage: check-acceptance-report.ts <vitest json report> [floor dir]\n");
  process.exit(2);
}
const floorDir = process.argv[3] ?? fileURLToPath(new URL(".", import.meta.url));

const readJson = <T>(path: string): T => JSON.parse(readFileSync(path, "utf8")) as T;
const { floor } = readJson<{ floor: number }>(join(floorDir, "case-floor.json"));
const report = readJson<Report>(reportPath);

const key = (file: string, test: string): string => `${file}::${test}`;
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

const failing = results.filter((r) => r.status === "failed");
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
  "failing cases",
  failing.map((r) => r.id),
);

process.stdout.write(
  `acceptance: ${results.length} cases (floor ${floor}), ${failing.length} failing\n`,
);
const problems =
  failing.length + notLoaded.length + outside.length + suiteFailures + (belowFloor ? 1 : 0);
process.exit(problems === 0 ? 0 : 1);
