import { describe, expect } from "vitest";
import { referenceDrift } from "../drift";
import { fixture, layers, readFile } from "../helpers";

// Spec §10.2 (TASK-73): where alz's YAML emitter writes other bytes than the reference, so that
// what alz generates reads back as it was written. These cases check alz's own bytes through
// generated agent frontmatter. The escapes for U+007F to U+009F, U+2028 and U+2029 and the
// quoting of text that starts with U+FEFF cannot be reached through a build: alz's reader refuses
// those characters in layer files, and adoption refuses them before writing (see below), so those
// bytes are pinned by the golden samples of crates/alz-core (cargo test) until `alz account allow`
// writes a path that holds them (R6).
describe("§10.2 emitter drift", () => {
  referenceDrift(
    "TASK-73",
    "quotes text the reference writes plain and alz's reader would read as an integer",
    async () => {
      const ws = fixture("38-emitter-drift");
      const result = await layers(ws, "repo", ["build"]);
      expect(result.code).toBe(0);
      const description = (name: string) =>
        readFile(ws, `repo/.claude/agents/${name}.md`)
          .split("\n")
          .find((line) => line.startsWith("description:"));
      expect(description("n0b101")).toBe('description: "0b101"');
      expect(description("n0b1")).toBe('description: "+0b1"');
      expect(description("n0b11")).toBe('description: "0b11"');
      expect(description("n0o17")).toBe('description: "+0o17"');
      expect(description("n0x1f")).toBe('description: "-0x1f"');
    },
  );

  referenceDrift(
    "TASK-73",
    "writes a space between line feeds in long double-quoted text as one escaped space",
    async () => {
      const ws = fixture("38-emitter-drift");
      const result = await layers(ws, "repo-long", ["build"]);
      expect(result.code).toBe(0);
      const agent = readFile(ws, "repo-long/.claude/agents/long.md");
      expect(agent).toContain(
        `description: "${"a".repeat(44)}\n\n  \\ \n\n  b\n\n  \\ \n\n  \\ "\n---\n`,
      );
    },
  );
});
