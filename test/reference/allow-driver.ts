import { existsSync } from "node:fs";
import { updateAllowed } from "../../src/resolve/account-allow-writer";

/**
 * Run by test 32 in separate processes (bundled with esbuild): waits for a start file so that
 * every process begins together, then records one entry and prints the result as JSON.
 */
const [json] = process.argv.slice(2);
const input = JSON.parse(json ?? "{}") as {
  go: string;
  root: string;
  home: string;
  env: Record<string, string>;
  account: Record<string, string>;
  source: string;
};
const pause = new Int32Array(new SharedArrayBuffer(4));
while (!existsSync(input.go)) Atomics.wait(pause, 0, 0, 5);
let waited = false;
updateAllowed({
  change: {
    kind: "set",
    entry: { root: input.root, account: input.account, source: input.source },
  },
  repoRoot: input.root,
  home: input.home,
  env: input.env,
  uid: process.getuid?.(),
  waitMs: 30_000,
  onLockBusy: () => {
    waited = true;
  },
}).then((result) => {
  process.stdout.write(JSON.stringify({ ...result, waited }));
  process.exitCode = result.kind === "refused" ? 1 : 0;
});
