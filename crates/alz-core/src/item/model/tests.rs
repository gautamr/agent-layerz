use crate::item::{ConcreteModel, ModelAlias, ModelItems, Models};
use crate::merge::testing::layer;
use alz_adapter::TargetId;

fn read(yaml: &str) -> (Option<Models>, Vec<String>) {
    let (items, found) = ModelItems::read(&layer("personal", "personal", &[("models.yaml", yaml)]));
    (
        items.into_file().map(|(_, models)| models),
        found.iter().map(ToString::to_string).collect(),
    )
}

fn alias(text: &str) -> ModelAlias {
    ModelAlias::try_from(text).expect("an alias")
}

#[test]
fn aliases_roles_and_default_are_read_into_ordered_maps() {
    let (models, found) = read(
        "aliases:\n  strong: { codex: big, claude: opus }\n  fast: { claude: sonnet }\nroles:\n  review: strong\n  planning: strong\ndefault: fast\n",
    );
    assert_eq!(found, Vec::<String>::new());
    let models = models.expect("models");
    let aliases: Vec<&str> = models.aliases().keys().map(ModelAlias::as_str).collect();
    assert_eq!(aliases, ["fast", "strong"]);
    let strong = models.aliases().get(&alias("strong")).expect("strong");
    let targets: Vec<(TargetId, &str)> = strong
        .iter()
        .map(|(target, model)| (target.clone(), model.as_str()))
        .collect();
    assert_eq!(
        targets,
        [
            (crate::target::claude().clone(), "opus"),
            (crate::target::codex().clone(), "big")
        ]
    );
    let roles: Vec<(&str, &str)> = models
        .roles()
        .iter()
        .map(|(role, alias)| (role.as_str(), alias.as_str()))
        .collect();
    assert_eq!(roles, [("planning", "strong"), ("review", "strong")]);
    assert_eq!(models.default().map(ModelAlias::as_str), Some("fast"));
}

#[test]
fn a_layer_without_models_yaml_or_with_an_empty_one_has_no_models() {
    assert_eq!(read(""), (None, Vec::new()));
    let (items, found) = ModelItems::read(&layer("personal", "personal", &[]));
    assert!(items.into_file().is_none());
    assert_eq!(found.iter().count(), 0);
}

#[test]
fn an_alias_key_that_is_not_a_target_is_refused_naming_the_file_whatever_the_key() {
    for key in [
        "__proto__",
        "Constructor",
        "Claude",
        "CLAUDE",
        "Gemini",
        "",
        "claude ",
    ] {
        let (models, found) = read(&format!(
            "aliases:\n  strong:\n    claude: opus\n    \"{key}\": x\n"
        ));
        assert_eq!(models, None, "{key:?}");
        assert_eq!(found.len(), 1, "{key:?}: {found:?}");
        assert!(
            found[0].starts_with("error [models] personal:models.yaml:4:"),
            "{found:?}"
        );
        assert!(
            found[0].contains("alias 'strong' has a key that is not a target: '"),
            "{found:?}"
        );
    }
}

#[test]
fn a_key_that_is_not_text_is_refused_by_its_kind() {
    let (models, found) = read("aliases:\n  strong:\n    1: x\n");
    assert_eq!(models, None);
    assert_eq!(
        found,
        [
            "error [models] personal:models.yaml:3:5: alias 'strong' has a key that is not a target, but a number"
        ]
    );
}

#[test]
fn the_shape_of_each_part_is_checked() {
    for (yaml, expected) in [
        ("aliases: x\n", "aliases must be a mapping"),
        (
            "aliases:\n  strong: x\n",
            "an alias must map each target to a model id",
        ),
        (
            "aliases:\n  strong: { claude: [a] }\n",
            "alias 'strong': the model of claude must be a model id, text that is not empty",
        ),
        (
            "aliases:\n  strong: { claude: ' ' }\n",
            "alias 'strong': the model of claude must be a model id, text that is not empty",
        ),
        ("roles: x\n", "roles must map each role to an alias"),
        (
            "roles:\n  review: 1\n",
            "a role must route to an alias, text that is not empty",
        ),
        (
            "default: [a]\n",
            "default must be an alias, text that is not empty",
        ),
        ("extra: 1\n", "unknown field 'extra'"),
    ] {
        let (models, found) = read(yaml);
        assert_eq!(models, None, "{yaml}");
        assert_eq!(found.len(), 1, "{yaml}: {found:?}");
        assert!(
            found[0].starts_with("error [models] personal:models.yaml:"),
            "{found:?}"
        );
        assert!(found[0].ends_with(expected), "{yaml}: {found:?}");
    }
}

#[test]
fn every_problem_of_the_file_is_reported() {
    let (models, found) = read("aliases:\n  a: { X: y }\nroles:\n  r: 1\nextra: 1\n");
    assert_eq!(models, None);
    assert_eq!(found.len(), 3, "{found:?}");
}

#[test]
fn a_model_id_is_text_as_written() {
    let model = ConcreteModel::try_from("claude-opus-4").expect("a model");
    assert_eq!(model.as_str(), "claude-opus-4");
    assert!(ConcreteModel::try_from("  ").is_err());
}
