import type { YamlMap } from "../core/yaml";
import type { AgentSpec } from "../merge/agents";
import type { DeclaredIdentity } from "../merge/commit-identity";
import type { HookSpec } from "../merge/hooks";
import type { InstructionBlock } from "../merge/instructions";
import type { McpServer } from "../merge/mcp";
import type { ModelsSpec } from "../merge/models";
import type { ClaudeExtra, PassthroughFile } from "../merge/passthrough";
import type { PermissionsSpec } from "../merge/permissions";
import type { RuleSpec } from "../merge/rules";
import type { SkillSpec } from "../merge/skills";
import type { Layer } from "./layer";
import type { Manifest } from "./manifest";

/** The merged, target-neutral model of one repo's stack. */
export interface ResolvedConfig {
  manifest: Manifest;
  layers: Layer[];
  instructions: InstructionBlock[];
  agents: Map<string, AgentSpec>;
  skills: Map<string, SkillSpec>;
  rules: Map<string, RuleSpec>;
  hooks: HookSpec[];
  permissions: PermissionsSpec;
  mcp: { all: Map<string, McpServer>; enabled: McpServer[] };
  models: ModelsSpec;
  settings: Record<string, YamlMap>;
  passthrough: Map<string, Map<string, PassthroughFile>>;
  claudeExtras: ClaudeExtra[];
  /** §7.12: the winning commit identity and its layer; undefined when no layer declares one. */
  commitIdentity?: DeclaredIdentity;
}
