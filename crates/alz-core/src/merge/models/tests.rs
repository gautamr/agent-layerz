use crate::item::{ConcreteModel, ModelAlias, Role};
use crate::merge::provenance::Action;
use crate::merge::testing::{layer, lines, merged};
use crate::stack::ResolvedLayer;
use crate::target::TargetName;

fn models_layer(name: &str, yaml: &str) -> ResolvedLayer {
    layer(name, "client", &[("models.yaml", yaml)])
}

fn alias(text: &str) -> ModelAlias {
    ModelAlias::try_from(text).expect("an alias")
}

fn role(text: &str) -> Role {
    Role::try_from(text).expect("a role")
}

const PERSONAL: &str = "aliases:\n  strong: { claude: opus, codex: big }\n  fast: { claude: sonnet }\nroles:\n  planning: strong\n  research: fast\ndefault: fast\n";

#[test]
fn roles_resolve_through_aliases_to_the_model_of_each_target() {
    let found = merged(vec![models_layer("personal", PERSONAL)]);
    assert_eq!(lines(&found), Vec::<String>::new());
    let models = found.merged().expect("merged").models();
    let model = |role_name: &str, target| {
        models
            .for_role(&role(role_name), target)
            .map(ConcreteModel::as_str)
    };
    assert_eq!(model("planning", TargetName::Claude), Some("opus"));
    assert_eq!(model("planning", TargetName::Codex), Some("big"));
    assert_eq!(model("research", TargetName::Claude), Some("sonnet"));
    // The alias has no model for this target; a role nobody routes has none at all.
    assert_eq!(model("research", TargetName::Codex), None);
    assert_eq!(model("unrouted", TargetName::Claude), None);
    assert_eq!(
        models.alias_of(&role("planning")).map(ModelAlias::as_str),
        Some("strong")
    );
    assert_eq!(
        models
            .for_default(TargetName::Claude)
            .map(ConcreteModel::as_str),
        Some("sonnet")
    );
    assert_eq!(
        models
            .model(&alias("strong"), TargetName::Claude)
            .map(ConcreteModel::as_str),
        Some("opus")
    );
}

#[test]
fn aliases_and_roles_are_deep_merged_and_the_most_specific_value_wins() {
    let found = merged(vec![
        models_layer("personal", PERSONAL),
        models_layer(
            "repo",
            "aliases:\n  strong: { codex: huge }\n  extra: { claude: haiku }\nroles:\n  research: strong\ndefault: strong\n",
        ),
    ]);
    assert_eq!(lines(&found), Vec::<String>::new());
    let models = found.merged().expect("merged").models();
    // The alias keeps the target the repo did not name and takes its own model for the other.
    assert_eq!(
        models
            .model(&alias("strong"), TargetName::Claude)
            .map(ConcreteModel::as_str),
        Some("opus")
    );
    assert_eq!(
        models
            .model(&alias("strong"), TargetName::Codex)
            .map(ConcreteModel::as_str),
        Some("huge")
    );
    assert!(models.aliases().contains_key(&alias("extra")));
    assert!(models.aliases().contains_key(&alias("fast")));
    assert_eq!(
        models.alias_of(&role("research")).map(ModelAlias::as_str),
        Some("strong")
    );
    assert_eq!(
        models.alias_of(&role("planning")).map(ModelAlias::as_str),
        Some("strong")
    );
    assert_eq!(
        models.default().map(|alias| alias.value().as_str()),
        Some("strong")
    );
}

#[test]
fn the_provenance_names_the_layers_that_set_an_alias_a_role_and_the_default() {
    let found = merged(vec![
        models_layer("personal", PERSONAL),
        models_layer(
            "repo",
            "aliases:\n  strong: { codex: huge }\nroles:\n  research: strong\ndefault: strong\n",
        ),
    ]);
    let models = found.merged().expect("merged").models();
    let history = |traced: &[(String, Action)]| traced.to_vec();
    let of = |provenance: &crate::merge::Provenance| {
        provenance
            .contributions()
            .iter()
            .map(|part| (part.layer().as_str().to_owned(), part.action()))
            .collect::<Vec<_>>()
    };
    let strong = models.aliases().get(&alias("strong")).expect("strong");
    assert_eq!(
        history(&of(strong.provenance())),
        [
            ("personal".to_owned(), Action::Define),
            ("repo".to_owned(), Action::Extend)
        ]
    );
    let research = models.roles().get(&role("research")).expect("research");
    assert_eq!(
        of(research.provenance()),
        [
            ("personal".to_owned(), Action::Define),
            ("repo".to_owned(), Action::Replace)
        ]
    );
    let default = models.default().expect("default");
    assert_eq!(
        of(default.provenance()),
        [
            ("personal".to_owned(), Action::Define),
            ("repo".to_owned(), Action::Replace)
        ]
    );
}

#[test]
fn a_role_or_default_that_names_an_unknown_alias_is_unresolved() {
    let found = merged(vec![models_layer(
        "personal",
        "aliases:\n  fast: { claude: sonnet }\nroles:\n  planning: strong\n  review: fast\ndefault: gone\n",
    )]);
    assert_eq!(
        lines(&found),
        [
            "error [unresolved] role 'planning' routes to unknown alias 'strong'",
            "error [unresolved] default 'gone' is not an alias",
        ]
    );
    assert!(found.merged().is_none());
}

#[test]
fn an_alias_of_a_more_general_layer_resolves_the_role_of_a_more_specific_one() {
    let found = merged(vec![
        models_layer("personal", "aliases:\n  strong: { claude: opus }\n"),
        models_layer("repo", "roles:\n  planning: strong\ndefault: strong\n"),
    ]);
    assert_eq!(lines(&found), Vec::<String>::new());
}

#[test]
fn an_alias_key_that_is_not_a_target_is_an_error_and_the_file_is_not_merged() {
    let found = merged(vec![models_layer(
        "repo",
        "aliases:\n  strong: { gemini: x, claude: opus }\nroles:\n  planning: strong\n",
    )]);
    let printed = lines(&found);
    // The alias of the refused file is not merged, so the role is unresolved too.
    assert_eq!(printed.len(), 1, "{printed:?}");
    assert!(
        printed[0].starts_with("error [models] repo:models.yaml:2:"),
        "{printed:?}"
    );
    assert!(found.merged().is_none());
}

#[test]
fn a_stack_without_models_has_none() {
    let found = merged(vec![layer("personal", "personal", &[])]);
    let models = found.merged().expect("merged").models();
    assert!(models.aliases().is_empty());
    assert!(models.roles().is_empty());
    assert!(models.default().is_none());
}
