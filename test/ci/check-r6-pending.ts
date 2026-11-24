/**
 * The acceptance job against the Rust binary is blocking, except for the cases R6 (TASK-55.7)
 * delivers: accounts and commit identity. They are listed in `r6-pending.json`, each with its
 * reason. This script reads the JSON report of a vitest run and exits 1 when
 *   - a case fails that the list does not name (a regression), or
 *   - a case the list names passes (the marker is stale: delete the entry).
 * Run it as `node --experimental-strip-types test/ci/check-r6-pending.ts <report.json>`.
 */
import { readFileSync } from "node:fs";

interface Pending {
  readonly test: string;
  readonly reason: string;
}

interface Report {
  readonly testResults: ReadonlyArray<{
    readonly name: string;
    readonly assertionResults: ReadonlyArray<{
      readonly fullName: string;
      readonly status: string;
    }>;
  }>;
}

const reportPath = process.argv[2];
if (reportPath === undefined) {
  process.stderr.write("usage: check-r6-pending.ts <vitest json report>\n");
  process.exit(2);
}

const pending = JSON.parse(
  readFileSync(new URL("./r6-pending.json", import.meta.url), "utf8"),
) as Record<string, readonly Pending[]>;
const report = JSON.parse(readFileSync(reportPath, "utf8")) as Report;

const key = (file: string, test: string): string => `${file}::${test}`;
const listed = new Set(
  Object.entries(pending).flatMap(([file, rows]) => rows.map((row) => key(file, row.test))),
);

const results = report.testResults.flatMap((file) => {
  const relative = file.name.slice(file.name.indexOf("test/acceptance/"));
  return file.assertionResults.map((result) => ({
    id: key(relative, result.fullName),
    status: result.status,
  }));
});

const unexpected = results.filter((r) => r.status === "failed" && !listed.has(r.id));
const stale = results.filter((r) => r.status === "passed" && listed.has(r.id));
const ran = new Set(results.map((r) => r.id));
const missing = [...listed].filter((id) => !ran.has(id));

const show = (title: string, rows: readonly string[]): void => {
  if (rows.length > 0) process.stderr.write(`${title} (${rows.length}):\n  ${rows.join("\n  ")}\n`);
};
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
  `acceptance: ${results.length} cases, ${failed} failing, ${listed.size} listed as R6 pending\n`,
);
process.exit(unexpected.length + stale.length + missing.length === 0 ? 0 : 1);
