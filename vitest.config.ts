import { defineConfig } from "vitest/config";

// `acceptance` is the language-neutral oracle: it must not import src/ (test/guards enforces it).
// `reference` holds the TypeScript-only tests and the guards; it goes with src/ at the cutover.
// One global setup makes the run root both projects keep their temp folders in, and the binary
// the acceptance project spawns (ALZ_BIN, or the TypeScript reference built from src/).
export default defineConfig({
  test: {
    testTimeout: 20_000,
    globalSetup: ["test/setup/run-root.ts"],
    projects: [
      {
        extends: true,
        test: { name: "acceptance", include: ["test/acceptance/**/*.test.ts"] },
      },
      {
        extends: true,
        test: {
          name: "reference",
          include: ["test/reference/**/*.test.ts", "test/guards/**/*.test.ts"],
        },
      },
    ],
  },
});
