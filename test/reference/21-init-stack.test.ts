import { describe, expect, it } from "vitest";
import { parseStackOption } from "../../src/cli/commands/init";

describe("§10 alz init --stack --hub", () => {
  it("reads --stack values as path, git or github sources", () => {
    expect(parseStackOption("../personal")).toEqual({ source: "path:../personal" });
    expect(parseStackOption("path:../hub/layer@layer-v1.0.0")).toEqual({
      source: "path:../hub/layer",
      ref: "layer-v1.0.0",
    });
    expect(parseStackOption("github:me/agent-layerz-personal@v1.2.0")).toEqual({
      source: "github:me/agent-layerz-personal@v1.2.0",
    });
  });
});
