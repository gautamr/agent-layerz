import { describe, expect, it } from "vitest";
import { lockedEntryFor, verifyCachedLayer } from "../../src/resolve/cache-integrity";

// The units of test/acceptance/28-cache-integrity.test.ts that call the TypeScript implementation
// directly. Their CLI side is in the acceptance file: the cached layer that was edited, added to
// or untouched (verifyCachedLayer), and the lock that names another URL or has no entry
// (lockedEntryFor). Deleted with src/ at the cutover.
describe("cache integrity", () => {
  describe("verifyCachedLayer", () => {
    const locked = (sha: string | undefined, contentHash: string) => ({
      name: "x",
      source: "git:y@main",
      sha,
      contentHash,
    });

    it("is ok for equal content hashes and a miss for different ones", () => {
      // The lock writes the hash with a `sha256:` prefix.
      expect(verifyCachedLayer({ contentHash: "aa" }, { contentHash: "sha256:aa" })).toBe("ok");
      expect(verifyCachedLayer({ contentHash: "aa" }, { contentHash: "sha256:bb" })).toBe("miss");
    });

    it("selects a lock entry only for the same source and the same commit", () => {
      const entries = [locked("1".repeat(40), "aa")];
      expect(lockedEntryFor(entries, "git:y@main", "1".repeat(40))).toBe(entries[0]);
      expect(lockedEntryFor(entries, "git:y@main", "2".repeat(40))).toBeUndefined();
      expect(lockedEntryFor(entries, "git:z@main", "1".repeat(40))).toBeUndefined();
      expect(lockedEntryFor(entries, "git:y@main", undefined)).toBeUndefined();
    });
  });
});
