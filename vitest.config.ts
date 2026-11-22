import { defineConfig } from "vitest/config";

// `acceptance` is the language-neutral oracle: it must not import src/ (test/guards enforces it).
// `reference` holds the TypeScript-only tests and the guards; it goes with src/ at the cutover.
export default defineConfig({
  test: {
    testTimeout: 20_000,
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
