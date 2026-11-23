use proptest::prelude::{Strategy, any, prop_assert_eq, proptest};

use crate::item::{Glob, RuleBase, RuleEdit, RuleExtension, RuleName};
use crate::merge::action::Action;
use crate::merge::algebra::OrderedSet;
use crate::merge::provenance::Traced;
use crate::merge::testing::{layer, lines, merged};
use crate::merge::{Merged, MergedRule, RuleChain};
use crate::stack::ResolvedLayer;

// The files of acceptance case 17 (§13.17), read from the fixture itself.
const PERSONAL_BACKEND: &str =
    include_str!("../../../../../test/fixtures/17-rules/personal/rules/backend.md");
const PERSONAL_TESTING: &str =
    include_str!("../../../../../test/fixtures/17-rules/personal/rules/testing.md");
const REPO_BACKEND: &str =
    include_str!("../../../../../test/fixtures/17-rules/repo/.layer/rules/backend.md");
const REPO_TESTING: &str =
    include_str!("../../../../../test/fixtures/17-rules/repo/.layer/rules/testing.md");

fn case_17() -> Vec<ResolvedLayer> {
    vec![
        layer(
            "personal",
            "personal",
            &[
                ("rules/backend.md", PERSONAL_BACKEND),
                ("rules/testing.md", PERSONAL_TESTING),
            ],
        ),
        layer(
            "shop-api",
            "repo",
            &[
                ("rules/backend.md", REPO_BACKEND),
                ("rules/testing.md", REPO_TESTING),
            ],
        ),
    ]
}

fn merged_model(layers: Vec<ResolvedLayer>) -> Merged {
    let found = merged(layers);
    assert_eq!(lines(&found), Vec::<String>::new());
    found.merged().expect("no error was found").clone()
}

fn rule<'a>(model: &'a Merged, name: &str) -> &'a Traced<MergedRule> {
    model
        .rules()
        .get(&RuleName::try_from(name).expect("a rule name"))
        .expect("the rule exists")
}

fn glob_of(text: &str) -> Glob {
    Glob::try_from(text).expect("a glob")
}

fn globs(rule: &MergedRule) -> Vec<String> {
    rule.paths()
        .iter()
        .map(|glob| glob.as_str().to_owned())
        .collect()
}

fn makers(rule: &Traced<MergedRule>) -> Vec<(&str, Action)> {
    rule.provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action().clone()))
        .collect()
}

// Acceptance case 17, first case: "a repo rules/backend.md replaces the personal one".
#[test]
fn a_repo_rule_replaces_the_personal_one_whole() {
    let model = merged_model(case_17());
    let backend = rule(&model, "backend");
    assert_eq!(globs(backend.value()), ["server/**/*.ts"]);
    assert_eq!(
        backend.value().body(),
        "REPO-BACKEND: use the service layer in server/services."
    );
    assert_eq!(backend.value().extensions().len(), 0);
    assert_eq!(
        makers(backend),
        [("personal", Action::Define), ("shop-api", Action::Replace)]
    );
}

// Acceptance case 17, second case: "an extend rule appends a section and combines paths", and the
// explain line "extend: paths +[e2e/**]" reads the extension's own globs.
#[test]
fn an_extend_rule_appends_a_section_and_unions_the_paths() {
    let model = merged_model(case_17());
    let testing = rule(&model, "testing");
    assert_eq!(globs(testing.value()), ["**/*.test.ts", "e2e/**"]);
    assert_eq!(
        testing.value().body(),
        "PERSONAL-TESTING: one behaviour per test.\n\n## Repo: shop-api\n\nREPO-TESTING: e2e tests run against the dev server."
    );
    assert_eq!(
        makers(testing),
        [
            ("personal", Action::Define),
            (
                "shop-api",
                Action::ExtendRule {
                    paths: vec![glob_of("e2e/**")]
                }
            )
        ]
    );
    let added: Vec<Vec<&str>> = testing
        .value()
        .extensions()
        .iter()
        .map(|section| {
            section
                .extension()
                .paths()
                .iter()
                .map(Glob::as_str)
                .collect()
        })
        .collect();
    assert_eq!(added, [vec!["e2e/**"]]);
}

#[test]
fn rules_are_kept_by_name_in_code_point_order() {
    let model = merged_model(case_17());
    let names: Vec<&str> = model.rules().keys().map(RuleName::as_str).collect();
    assert_eq!(names, ["backend", "testing"]);
}

#[test]
fn an_extension_without_text_adds_its_paths_and_no_section() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[("rules/a.md", "---\npaths: [\"src/**\"]\n---\nbase\n")],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "rules/a.md",
                "---\nmode: extend\npaths: [\"lib/**\", \"src/**\"]\n---\n",
            )],
        ),
    ]);
    let found = rule(&model, "a");
    assert_eq!(globs(found.value()), ["src/**", "lib/**"]);
    assert_eq!(found.value().body(), "base");
}

#[test]
fn a_rule_defined_again_drops_the_extensions_before_it() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[("rules/a.md", "---\npaths: [x]\n---\nbase\n")],
        ),
        layer(
            "acme",
            "client",
            &[("rules/a.md", "---\nmode: extend\npaths: [y]\n---\nmore\n")],
        ),
        layer(
            "shop-api",
            "repo",
            &[("rules/a.md", "---\npaths: [z]\n---\nfresh\n")],
        ),
    ]);
    let found = rule(&model, "a");
    assert_eq!(globs(found.value()), ["z"]);
    assert_eq!(found.value().body(), "fresh");
    assert_eq!(
        makers(found),
        [
            ("personal", Action::Define),
            (
                "acme",
                Action::ExtendRule {
                    paths: vec![glob_of("y")]
                }
            ),
            ("shop-api", Action::Replace)
        ]
    );
}

#[test]
fn two_extensions_keep_stack_order_and_each_glob_once() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[("rules/a.md", "---\npaths: [x]\n---\nbase\n")],
        ),
        layer(
            "acme",
            "client",
            &[(
                "rules/a.md",
                "---\nmode: extend\npaths: [y, x]\n---\nfrom client\n",
            )],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "rules/a.md",
                "---\nmode: extend\npaths: [y, z]\n---\nfrom repo\n",
            )],
        ),
    ]);
    let found = rule(&model, "a");
    assert_eq!(globs(found.value()), ["x", "y", "z"]);
    assert_eq!(
        found.value().body(),
        "base\n\n## Client: acme\n\nfrom client\n\n## Repo: shop-api\n\nfrom repo"
    );
}

#[test]
fn an_extension_with_no_rule_before_it_is_unresolved() {
    let found = merged(vec![
        layer("personal", "personal", &[]),
        layer(
            "shop-api",
            "repo",
            &[("rules/a.md", "---\nmode: extend\npaths: [x]\n---\ntext\n")],
        ),
    ]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [unresolved] shop-api:rules/a.md: mode: extend, but no more general layer has rules/a.md"
        ]
    );
}

#[test]
fn a_rule_that_breaks_its_schema_is_a_rule_error_that_names_the_file() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[
            ("rules/Bad_Name.md", "---\npaths: [x]\n---\n"),
            ("rules/colour.md", "---\ncolour: red\npaths: [x]\n---\n"),
            ("rules/mode.md", "---\nmode: append\npaths: [x]\n---\n"),
            ("rules/empty-glob.md", "---\npaths: [x, \"  \"]\n---\n"),
            ("rules/no-paths.md", "---\n---\nalways loaded\n"),
            ("rules/not-list.md", "---\npaths: src/**\n---\n"),
            ("rules/not-text.md", "---\npaths: [1]\n---\n"),
            ("rules/extend-ok.md", "---\nmode: extend\n---\n"),
        ],
    )]);
    assert_eq!(
        lines(&found),
        [
            "error [rule] personal:rules/Bad_Name.md: rule name 'Bad_Name' must be kebab-case: lower-case letters and digits in groups joined by '-'",
            "error [rule] personal:rules/colour.md:2:1: unknown field 'colour'",
            "error [rule] personal:rules/empty-glob.md:2:12: paths[1] must be a non-empty glob",
            "error [rule] personal:rules/mode.md:2:7: mode must be replace or extend",
            "error [rule] personal:rules/no-paths.md: paths is required; always-loaded text belongs in instructions/",
            "error [rule] personal:rules/not-list.md:2:8: paths must be a list",
            "error [rule] personal:rules/not-text.md:2:9: paths[0] must be text; put the value in quotes to keep it as text",
            "error [unresolved] personal:rules/extend-ok.md: mode: extend, but no more general layer has rules/extend-ok.md",
        ]
    );
}

#[test]
fn a_rule_whose_block_is_not_closed_is_a_frontmatter_error() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[("rules/a.md", "---\npaths: [x]\n")],
    )]);
    assert_eq!(
        lines(&found),
        ["error [frontmatter] personal:rules/a.md: frontmatter is not closed with '---'"]
    );
}

fn glob() -> impl Strategy<Value = Glob> {
    "[a-z*/.]{1,4}".prop_map(|text| Glob::try_from(text.as_str()).expect("a glob"))
}

fn paths() -> impl Strategy<Value = OrderedSet<Glob>> {
    proptest::collection::vec(glob(), 0..4).prop_map(|globs| globs.into_iter().collect())
}

fn edit() -> impl Strategy<Value = RuleEdit> {
    let body = "[a-z ]{0,6}";
    proptest::prop_oneof![
        (paths(), body).prop_filter_map("a rule has a glob", |(paths, body)| {
            RuleBase::new(paths, body).map(RuleEdit::Replace)
        }),
        (paths(), body).prop_map(|(paths, body)| RuleEdit::Extend(RuleExtension::new(paths, body))),
    ]
}

/// A chain of the files of layers `a0`, `a1`, ... one file each, folded left.
fn chain(edits: &[(RuleEdit, u8)]) -> RuleChain {
    edits
        .iter()
        .fold(RuleChain::default(), |chain, (edit, which)| {
            let owner = layer(&format!("layer-{which}"), "repo", &[]);
            chain.merge(RuleChain::of(edit, owner.meta()))
        })
}

fn group() -> impl Strategy<Value = Vec<(RuleEdit, u8)>> {
    proptest::collection::vec((edit(), 0_u8..3), 0..4)
}

proptest! {
    // The merge of two layers' say on a rule is associative, with `default()` as its identity.
    #[test]
    fn the_rule_merge_is_associative_and_has_an_identity(
        first in group(),
        second in group(),
        third in group(),
    ) {
        let (a, b, c) = (chain(&first), chain(&second), chain(&third));
        prop_assert_eq!(
            a.clone().merge(b.clone()).merge(c.clone()),
            a.clone().merge(b.merge(c))
        );
        prop_assert_eq!(a.clone().merge(RuleChain::default()), a.clone());
        prop_assert_eq!(RuleChain::default().merge(a.clone()), a);
    }

    // Folding the files one at a time is the same as folding groups of them: the stack can be
    // merged in any grouping.
    #[test]
    fn folding_a_stack_in_groups_equals_folding_it_whole(
        first in group(),
        second in group(),
    ) {
        let whole: Vec<(RuleEdit, u8)> = first.iter().chain(&second).cloned().collect();
        prop_assert_eq!(chain(&whole), chain(&first).merge(chain(&second)));
    }

    // The globs of a merged rule are a union: adding an extension twice adds nothing the second
    // time, and every glob of every extension is there, once.
    #[test]
    fn path_unions_are_idempotent(
        base in paths().prop_filter("a glob", |paths| !paths.is_empty()),
        added in paths(),
        body in any::<bool>(),
    ) {
        let owner = layer("layer-0", "repo", &[]);
        let text = if body { "text" } else { "" }.to_owned();
        let base = RuleChain::of(
            &RuleEdit::Replace(RuleBase::new(base.clone(), String::new()).expect("a glob")),
            owner.meta(),
        );
        let extension = RuleChain::of(
            &RuleEdit::Extend(RuleExtension::new(added.clone(), text)),
            owner.meta(),
        );
        let once = base.clone().merge(extension.clone());
        let twice = once.clone().merge(extension);
        let paths_of = |chain: RuleChain| chain.into_rule().expect("defined").paths();
        let once_paths = paths_of(once);
        prop_assert_eq!(&paths_of(twice), &once_paths);
        prop_assert_eq!(
            once_paths.len(),
            once_paths.iter().collect::<std::collections::BTreeSet<_>>().len()
        );
        let all: std::collections::BTreeSet<&Glob> = once_paths.iter().collect();
        for glob in added.iter().chain(paths_of(base).iter()) {
            assert!(all.contains(glob));
        }
    }
}

const PERSONAL_YAML: &str =
    include_str!("../../../../../test/fixtures/17-rules/personal/layer.yaml");
const REPO_YAML: &str =
    include_str!("../../../../../test/fixtures/17-rules/repo/.layer/layer.yaml");

// Acceptance case 17, explain: the typed contributions give the strings the suite asserts, and
// the same sources the generated header names.
#[test]
fn the_contributions_of_a_rule_give_the_lines_of_alz_explain_of_case_17() {
    use crate::render::action_word::ActionWord;
    use crate::render::header::SourceRef;

    let model = merged_model(vec![
        crate::merge::testing::layer_from_yaml(
            PERSONAL_YAML,
            &[
                ("rules/backend.md", PERSONAL_BACKEND),
                ("rules/testing.md", PERSONAL_TESTING),
            ],
        ),
        crate::merge::testing::layer_from_yaml(
            REPO_YAML,
            &[
                ("rules/backend.md", REPO_BACKEND),
                ("rules/testing.md", REPO_TESTING),
            ],
        ),
    ]);
    let line = |index: usize, part: &crate::merge::Contribution| {
        format!(
            "{}. {} — {}",
            index + 1,
            SourceRef::of_contribution(part).expect("a source"),
            ActionWord::new(part.action(), "rule")
        )
    };
    let lines: Vec<String> = rule(&model, "testing")
        .provenance()
        .contributions()
        .iter()
        .enumerate()
        .map(|(index, part)| line(index, part))
        .collect();
    assert_eq!(
        lines,
        [
            "1. personal@1.0.0:rules/testing.md — rule",
            "2. shop-api:rules/testing.md — extend: paths +[e2e/**]"
        ]
    );
}
