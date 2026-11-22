/**
 * The trace shared by the tests that prove a command only reads (tests 30 and 31). Every
 * function of `node:fs` and `node:child_process` records its name and the paths it was given
 * while `trace.on` is set. A test file installs it with two `vi.mock` calls whose factories
 * call `intercept`, proves it works (the positive control) and then uses `traced` or
 * `tracedAsync` around the code under test.
 */

export interface TraceCall {
  api: string;
  fn: string;
  paths: string[];
  flags?: unknown;
}

export const trace: { on: boolean; calls: TraceCall[]; fds: Map<number, string> } = {
  on: false,
  calls: [],
  fds: new Map(),
};

export function intercept(api: string, actual: Record<string, unknown>): Record<string, unknown> {
  const wrapped: Record<string, unknown> = {};
  const wrap = (name: string, original: (...args: unknown[]) => unknown) =>
    function (this: unknown, ...args: unknown[]) {
      if (trace.on) {
        const two = ["renameSync", "copyFileSync", "cpSync", "linkSync", "symlinkSync"];
        const paths = args
          .slice(0, two.includes(name) ? 2 : 1)
          .filter((arg) => typeof arg === "string" || Buffer.isBuffer(arg))
          .map(String);
        if (typeof args[0] === "number") {
          const known = trace.fds.get(args[0]);
          if (known !== undefined) paths.unshift(known);
        }
        const options = args[1] as { flag?: unknown } | string | number | undefined;
        const flags =
          name === "openSync"
            ? args[1]
            : typeof options === "object" && options !== null
              ? options.flag
              : undefined;
        trace.calls.push({ api, fn: name, paths, flags });
      }
      const result = original.apply(this, args);
      if (trace.on && name === "openSync" && typeof result === "number") {
        trace.fds.set(result, String(args[0]));
      }
      return result;
    };
  for (const [name, value] of Object.entries(actual)) {
    // Functions only: classes (`Stats`, `Dirent`, …) and constants pass through.
    if (typeof value !== "function" || !/^[a-z]/.test(name)) {
      wrapped[name] = value;
      continue;
    }
    const fn = wrap(name, value as (...args: unknown[]) => unknown);
    if (name === "realpathSync") {
      const native = (value as unknown as { native: (...args: unknown[]) => unknown }).native;
      (fn as unknown as { native: unknown }).native = wrap("realpathSync.native", native);
    }
    wrapped[name] = fn;
  }
  wrapped.default = wrapped;
  return wrapped;
}

export function traced<T>(action: () => T): { result: T; calls: TraceCall[] } {
  trace.calls = [];
  trace.fds.clear();
  trace.on = true;
  try {
    return { result: action(), calls: [...trace.calls] };
  } finally {
    trace.on = false;
  }
}

export async function tracedAsync<T>(
  action: () => Promise<T>,
): Promise<{ result: T; calls: TraceCall[] }> {
  trace.calls = [];
  trace.fds.clear();
  trace.on = true;
  try {
    return { result: await action(), calls: [...trace.calls] };
  } finally {
    trace.on = false;
  }
}
