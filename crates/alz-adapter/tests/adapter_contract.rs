//! Spec §8.0: an adapter is a pure function from the model and a context to its output, behind a
//! trait that anyone may implement, and an output holds files of four kinds of content, diagnostics
//! and a coverage report.

#[cfg(test)]
mod tests {
    use alz_adapter::{
        Adapter, Bytes, Content, Context, Coverage, Descriptor, Diagnostic, Enforcement,
        FloorField, FloorFieldItem, Frontmatter, HookEvent, JsonObject, JsonValue, LayerName,
        LockedHook, MODEL_VERSION, Markdown, Model, ModelVersion, NeutralEvent, OnFail, Output,
        OutputFile, SecurityItemId, SettingLayers, Source, TargetId, TomlValue, ToolVersion,
        ToolVersionRange,
    };
    use alz_adapter::{DenyRule, Level};

    /// An adapter that writes one file of each kind and reports every item as expressed.
    struct Toy {
        descriptor: Descriptor,
    }

    fn target(text: &str) -> TargetId {
        TargetId::try_from(text).expect("a target id")
    }

    fn layer(text: &str) -> LayerName {
        LayerName::try_from(text).expect("a layer name")
    }

    impl Toy {
        fn new() -> Self {
            Self {
                descriptor: Descriptor::new(
                    target("toy"),
                    MODEL_VERSION,
                    ToolVersionRange::new(ToolVersion::new(1, 0, 0), ToolVersion::new(2, 0, 0)),
                    "docs/target-facts.md",
                ),
            }
        }
    }

    impl Adapter for Toy {
        fn descriptor(&self) -> &Descriptor {
            &self.descriptor
        }

        fn render(&self, model: &Model, context: &Context) -> Output {
            let directories = context
                .additional_directories
                .iter()
                .cloned()
                .map(JsonValue::Text)
                .collect();
            let settings = JsonObject::from_entries(vec![(
                "additionalDirectories".to_owned(),
                JsonValue::Array(directories),
            )])
            .expect("one key");
            let source = Source::new(layer("personal"), "instructions/a.md");
            let files = vec![
                OutputFile::new(
                    "toy/settings.json",
                    Content::Json(JsonValue::Object(settings)),
                ),
                OutputFile::new("toy/config.toml", Content::Toml(TomlValue::Boolean(true))),
                OutputFile::new(
                    "toy/notes.md",
                    Content::Markdown(
                        Markdown::new("body\n")
                            .with_frontmatter(Frontmatter::Kept("name: x".to_owned()))
                            .with_sources(vec![source.clone()]),
                    ),
                ),
                OutputFile::new(
                    "toy/run.sh",
                    Content::Bytes(Bytes::new(b"#!/bin/sh\n".to_vec()).executable(true)),
                ),
            ];
            let diagnostics =
                vec![Diagnostic::new(Level::Warning, "toy-note", "one thing").about(source)];
            Output::new(
                files,
                diagnostics,
                Coverage::all(model.security.ids(), Enforcement::Expressed),
            )
        }
    }

    fn model_with_every_kind_of_item() -> Model {
        let mut model = Model::new(target("toy"));
        model.security.deny_rules = vec![DenyRule::new(
            "bash:rm -rf *",
            SettingLayers::new(layer("personal")).and(layer("repo")),
        )];
        model.security.floor_fields = vec![FloorFieldItem::new(
            FloorField::DisableBypassMode,
            SettingLayers::new(layer("personal")),
        )];
        model.security.locked_hooks = vec![LockedHook::new(
            "gate",
            HookEvent::Neutral(NeutralEvent::PreTool),
            OnFail::Deny,
            layer("personal"),
        )];
        model
    }

    #[test]
    fn an_adapter_is_used_through_the_trait_object() {
        let adapter: Box<dyn Adapter> = Box::new(Toy::new());
        assert_eq!(adapter.descriptor().target.as_str(), "toy");
        assert!(MODEL_VERSION.runs(adapter.descriptor().model_version));
    }

    #[test]
    fn the_same_model_and_context_give_the_same_output() {
        let adapter = Toy::new();
        let model = model_with_every_kind_of_item();
        let context = Context::new(vec!["../hub".to_owned()]);
        assert_eq!(
            adapter.render(&model, &context),
            adapter.render(&model, &context)
        );
    }

    #[test]
    fn an_output_holds_a_file_of_each_of_the_four_kinds_of_content() {
        let output = Toy::new().render(&Model::new(target("toy")), &Context::default());
        let kinds: Vec<&str> = output
            .files
            .iter()
            .map(|file| match &file.content {
                Content::Json(_) => "json",
                Content::Toml(_) => "toml",
                Content::Markdown(_) => "markdown",
                Content::Bytes(_) => "bytes",
            })
            .collect();
        assert_eq!(kinds, ["json", "toml", "markdown", "bytes"]);
    }

    #[test]
    fn the_context_reaches_the_adapter_in_its_order() {
        let context = Context::new(vec!["../hub".to_owned(), "../shared".to_owned()]);
        let output = Toy::new().render(&Model::new(target("toy")), &context);
        let Some(OutputFile {
            content: Content::Json(JsonValue::Object(settings)),
            ..
        }) = output.files.first()
        else {
            panic!("the first file is JSON");
        };
        assert_eq!(
            settings.get("additionalDirectories"),
            Some(&JsonValue::Array(vec![
                JsonValue::Text("../hub".to_owned()),
                JsonValue::Text("../shared".to_owned()),
            ]))
        );
    }

    #[test]
    fn the_model_holds_the_four_kinds_of_security_item_with_their_layers() {
        let model = model_with_every_kind_of_item();
        let ids: Vec<SecurityItemId> = model.security.ids().collect();
        assert_eq!(
            ids,
            [
                SecurityItemId::DenyRule("bash:rm -rf *".to_owned()),
                SecurityItemId::FloorField(FloorField::DisableBypassMode),
                SecurityItemId::LockedHook("gate".to_owned()),
            ]
        );
        let layers: Vec<&str> = model.security.deny_rules[0]
            .set_by
            .iter()
            .map(LayerName::as_str)
            .collect();
        assert_eq!(layers, ["personal", "repo"]);
        assert_eq!(model.security.locked_hooks[0].on_fail, OnFail::Deny);
    }

    #[test]
    fn a_report_that_expresses_every_item_names_each_item_once() {
        let model = model_with_every_kind_of_item();
        let output = Toy::new().render(&model, &Context::default());
        let reported: Vec<SecurityItemId> = output
            .coverage
            .entries()
            .iter()
            .map(|entry| entry.item.clone())
            .collect();
        assert_eq!(reported, model.security.ids().collect::<Vec<_>>());
        assert!(
            output
                .coverage
                .entries()
                .iter()
                .all(|entry| entry.enforcement == Enforcement::Expressed)
        );
    }

    #[test]
    fn model_1_0_is_the_version_of_a_new_model() {
        assert_eq!(MODEL_VERSION, ModelVersion::new(1, 0));
        assert_eq!(Model::new(target("toy")).version(), MODEL_VERSION);
    }
}
