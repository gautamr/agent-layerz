#!/usr/bin/env node
import { run } from "./main";
import { processTerminal } from "./terminal";

const terminal = processTerminal();

run(process.argv.slice(2), {
  cwd: process.cwd(),
  env: process.env,
  stdout: (text) => process.stdout.write(text),
  stderr: (text) => process.stderr.write(text),
  terminal,
})
  .then(
    (code) => {
      process.exitCode = code;
    },
    (error: unknown) => {
      process.stderr.write(`internal error: ${(error as Error).stack ?? String(error)}\n`);
      process.exitCode = 70;
    },
  )
  .finally(() => terminal?.close?.());
