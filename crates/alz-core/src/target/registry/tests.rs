//! Spec §8.0 **Rendering.**: the registry holds the adapters the binary registers, and refuses one
//! built for a model version the core cannot run.

use alz_adapter::{
    Adapter, Context, Coverage, Descriptor, MODEL_VERSION, Model, ModelVersion, Output, TargetId,
    ToolVersion, ToolVersionRange,
};

use super::{RegisterError, Registry};

/// An adapter for the target `toy` built for `version`, which renders nothing.
struct Toy(Descriptor);

impl Toy {
    fn built_for(version: ModelVersion) -> Box<Self> {
        Box::new(Self(Descriptor::new(
            TargetId::try_from("toy").expect("a target id"),
            version,
            ToolVersionRange::new(ToolVersion::new(1, 0, 0), ToolVersion::new(2, 0, 0)),
            "docs/toy-facts.md",
        )))
    }
}

impl Adapter for Toy {
    fn descriptor(&self) -> &Descriptor {
        &self.0
    }

    fn render(&self, _model: &Model, _context: &Context) -> Output {
        Output::new(Vec::new(), Vec::new(), Coverage::default())
    }
}

#[test]
fn the_built_in_registry_holds_the_claude_adapter_and_its_descriptor() {
    let registry = Registry::builtin();
    let targets: Vec<&str> = registry.targets().map(TargetId::as_str).collect();
    assert_eq!(targets, ["claude"]);
    let descriptor = registry
        .descriptor(crate::target::claude())
        .expect("a descriptor");
    assert_eq!(
        descriptor.managed_folders,
        [
            ".claude/agents",
            ".claude/skills",
            ".claude/rules",
            ".claude/hooks"
        ]
    );
    assert_eq!(
        descriptor.protected_patterns,
        [".claude/agent-memory/**", ".claude/agent-memory-local/**"]
    );
    assert_eq!(
        registry.adopted_files(),
        [".claude/settings.json", ".mcp.json"]
    );
    assert!(descriptor.territory.files.contains(&"CLAUDE.md".to_owned()));
    assert_eq!(descriptor.tool_versions.to_string(), ">=2.1.311 <2.2.0");
}

#[test]
fn an_adapter_for_the_cores_model_version_is_registered_and_found_by_its_target() {
    let registry = Registry::new()
        .with(Toy::built_for(MODEL_VERSION))
        .expect("registered");
    let toy = TargetId::try_from("toy").expect("a target id");
    assert!(registry.adapter(&toy).is_some());
    assert!(registry.adapter(crate::target::claude()).is_none());
}

#[test]
fn an_adapter_built_for_another_major_or_a_later_minor_version_is_refused() {
    for built in [
        ModelVersion::new(MODEL_VERSION.major + 1, 0),
        ModelVersion::new(MODEL_VERSION.major, MODEL_VERSION.minor + 1),
    ] {
        let refused = Registry::new().with(Toy::built_for(built));
        assert!(
            matches!(refused, Err(RegisterError::ModelVersion { built: seen, .. }) if seen == built),
            "{built}"
        );
    }
}

#[test]
fn a_second_adapter_for_one_target_is_refused() {
    let refused = Registry::new()
        .with(Toy::built_for(MODEL_VERSION))
        .and_then(|registry| registry.with(Toy::built_for(MODEL_VERSION)));
    assert!(matches!(refused, Err(RegisterError::Twice(_))));
}

#[test]
fn a_target_without_an_adapter_is_the_error_target_at_layer_yaml_naming_what_is_available() {
    let registry = Registry::builtin();
    let codex = registry.refusal(crate::target::codex());
    assert_eq!(
        codex.to_string(),
        "error [target] layer.yaml: the 'codex' adapter is not implemented yet (implemented: claude)"
    );
    let other = registry.refusal(&crate::target::id("gemini"));
    assert_eq!(
        other.to_string(),
        "error [target] layer.yaml: no adapter for 'gemini' (available: claude)"
    );
}
