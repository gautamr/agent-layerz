use proptest::collection::vec;
use proptest::prelude::{prop_assert_eq, proptest};

use crate::item::{McpServerId, Transport};
use crate::merge::provenance::Action;
use crate::merge::testing::{layer, layer_from_yaml, lines, merged_placed, merged_with};
use crate::stack::ResolvedLayer;

// The files of acceptance case 09 (§13.9), read from the fixture itself: the personal layer
// defines two servers and the repository's manifest enables one of them.
const CASE_09_PERSONAL_YAML: &str =
    include_str!("../../../../../test/fixtures/09-mcp-allowlist/personal/layer.yaml");
const CASE_09_PERSONAL_MCP: &str =
    include_str!("../../../../../test/fixtures/09-mcp-allowlist/personal/mcp.yaml");
const CASE_09_ENABLE: &str = "mcp:\n  enable: [context7]\n";

fn id(text: &str) -> McpServerId {
    McpServerId::try_from(text).expect("a server id")
}

fn ids(found: &[McpServerId]) -> Vec<&str> {
    found.iter().map(McpServerId::as_str).collect()
}

fn mcp_layer(name: &str, yaml: &str) -> ResolvedLayer {
    layer(name, "client", &[("mcp.yaml", yaml)])
}

fn http(url: &str) -> String {
    format!("  transport: http\n  url: {url}\n")
}

#[test]
fn a_server_defined_but_not_enabled_is_not_in_the_enabled_list() {
    let found = merged_with(
        vec![layer_from_yaml(
            CASE_09_PERSONAL_YAML,
            &[("mcp.yaml", CASE_09_PERSONAL_MCP)],
        )],
        CASE_09_ENABLE,
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    let mcp = found.merged().expect("merged").mcp();
    assert_eq!(ids(mcp.enabled()), ["context7"]);
    let emitted: Vec<&str> = mcp.enabled_servers().map(|(id, _)| id.as_str()).collect();
    assert_eq!(emitted, ["context7"]);
    // Defined, and so known, but inactive.
    assert!(mcp.defined().get(&id("playwright")).is_some());
    assert_eq!(mcp.defined().len(), 2);
}

#[test]
fn nothing_is_enabled_when_the_manifest_enables_nothing() {
    let found = merged_with(
        vec![mcp_layer(
            "personal",
            &format!("a:\n{}", http("https://a.invalid/")),
        )],
        "",
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    let mcp = found.merged().expect("merged").mcp();
    assert_eq!(mcp.enabled(), Vec::<McpServerId>::new());
    assert_eq!(mcp.defined().len(), 1);
}

#[test]
fn the_enabled_ids_keep_the_order_of_the_manifest_without_repeats() {
    let yaml = format!("a:\n{u}b:\n{u}c:\n{u}", u = http("https://a.invalid/"));
    let found = merged_with(
        vec![mcp_layer("personal", &yaml)],
        "mcp:\n  enable: [c, a, c, b, a]\n",
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    let mcp = found.merged().expect("merged").mcp();
    assert_eq!(ids(mcp.enabled()), ["c", "a", "b"]);
    let emitted: Vec<&str> = mcp.enabled_servers().map(|(id, _)| id.as_str()).collect();
    assert_eq!(emitted, ["c", "a", "b"]);
}

#[test]
fn the_most_specific_definition_replaces_the_whole_entry_and_the_provenance_marks_it() {
    let personal = format!(
        "srv:\n  transport: stdio\n  command: old\n  args: [--a]\n  env_from: [FOO]\nother:\n{}",
        http("https://o.invalid/")
    );
    let repo = "srv:\n  transport: sse\n  url: https://new.invalid/sse\n";
    let found = merged_with(
        vec![mcp_layer("personal", &personal), mcp_layer("repo", repo)],
        "mcp:\n  enable: [srv]\n",
    );
    assert_eq!(lines(&found), Vec::<String>::new());
    let mcp = found.merged().expect("merged").mcp();
    assert_eq!(mcp.defined().len(), 2);
    let srv = mcp.defined().get(&id("srv")).expect("srv");
    assert!(matches!(
        srv.value().transport(),
        Transport::Sse { url } if url == "https://new.invalid/sse"
    ));
    let history: Vec<(&str, Action)> = srv
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action()))
        .collect();
    assert_eq!(
        history,
        [("personal", Action::Define), ("repo", Action::Replace)]
    );
}

#[test]
fn an_enabled_id_that_no_layer_defines_is_an_error() {
    let found = merged_with(
        vec![mcp_layer(
            "personal",
            &format!("a:\n{}", http("https://a.invalid/")),
        )],
        "mcp:\n  enable: [a, missing]\n",
    );
    assert_eq!(
        lines(&found),
        ["error [mcp] layer.yaml: mcp.enable lists missing, but no layer defines it"]
    );
    assert!(found.merged().is_none());
}

#[test]
fn a_server_refused_for_a_protected_variable_adds_no_second_error_for_the_allowlist() {
    let found = merged_with(
        vec![mcp_layer(
            "personal",
            "srv:\n  transport: stdio\n  command: run-it\n  env_from: [HOME]\n",
        )],
        "mcp:\n  enable: [srv]\n",
    );
    let found = lines(&found);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("error [protected-variable] personal:mcp.yaml:"));
    assert!(found.iter().all(|line| !line.contains("mcp.enable lists")));
}

#[test]
fn a_server_refused_for_another_reason_still_gets_the_allowlist_error() {
    let found = merged_with(
        vec![mcp_layer("personal", "srv:\n  transport: tcp\n")],
        "mcp:\n  enable: [srv]\n",
    );
    let found = lines(&found);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("error [mcp] personal:mcp.yaml:"));
    assert!(found[1].contains("mcp.enable lists srv, but no layer defines it"));
}

#[test]
fn a_defined_server_of_a_more_general_layer_stands_when_a_more_specific_one_is_refused() {
    let found = merged_with(
        vec![
            mcp_layer("personal", &format!("srv:\n{}", http("https://a.invalid/"))),
            mcp_layer(
                "repo",
                "srv:\n  transport: stdio\n  command: x\n  env_from: [CODEX_HOME]\n",
            ),
        ],
        "mcp:\n  enable: [srv]\n",
    );
    let found = lines(&found);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("error [protected-variable] repo:mcp.yaml:"));
}

#[test]
fn a_key_outside_the_grammar_is_an_error_naming_the_layer_and_the_file_and_defines_no_server() {
    let found = merged_with(
        vec![mcp_layer(
            "repo",
            &format!("\"a b\":\n{u}a:\n{u}", u = http("https://a.invalid/")),
        )],
        "mcp:\n  enable: [a]\n",
    );
    let printed = lines(&found);
    assert_eq!(printed.len(), 1, "{printed:?}");
    assert!(printed[0].starts_with("error [mcp] repo:mcp.yaml:1:1: 'a b' is not a server id"));
    assert!(found.merged().is_none());
}

proptest! {
    /// Law: the enabled list is the manifest's list without repeats, whatever the stack, and
    /// the most specific layer that defines an id gives its definition.
    #[test]
    fn enabled_is_the_manifest_list_without_repeats_and_the_last_definition_wins(
        layers in vec(proptest::collection::btree_set(0_usize..3, 0..4), 1..4),
        enable in vec(0_usize..3, 0..5),
    ) {
        let names = ["a", "b", "c"];
        let stack: Vec<ResolvedLayer> = layers
            .iter()
            .enumerate()
            .map(|(index, servers)| {
                let yaml: String = servers
                    .iter()
                    .fold(String::new(), |yaml, server| {
                        let url = format!("https://layer{index}.invalid/");
                        yaml + &format!("{}:\n{}", names[*server], http(&url))
                    });
                mcp_layer(&format!("layer{index}"), &yaml)
            })
            .collect();
        let enable_ids: Vec<&str> = enable.iter().map(|index| names[*index]).collect();
        let found = merged_with(stack, &format!("mcp:\n  enable: [{}]\n", enable_ids.join(", ")));
        let defined_by = |server: usize| layers.iter().rposition(|servers| servers.contains(&server));
        let all_defined = enable.iter().all(|server| defined_by(*server).is_some());
        prop_assert_eq!(found.merged().is_some(), all_defined);
        if let Some(merged) = found.merged() {
            let expected: Vec<&str> = enable_ids.iter().fold(Vec::new(), |kept: Vec<&str>, name| {
                if kept.contains(name) { kept } else { kept.into_iter().chain([*name]).collect() }
            });
            prop_assert_eq!(ids(merged.mcp().enabled()), expected);
            for (server, name) in names.iter().enumerate() {
                let winner = merged.mcp().defined().get(&id(name)).map(|traced| match traced.value().transport() {
                    Transport::Http { url } | Transport::Sse { url } => url.clone(),
                    Transport::Stdio { command, .. } => command.clone(),
                });
                prop_assert_eq!(winner, defined_by(server).map(|index| format!("https://layer{index}.invalid/")));
            }
        }
    }
}

#[test]
fn a_layer_inside_the_repository_that_replaces_an_outside_server_gets_a_warning_naming_both() {
    let personal = format!("srv:\n{}", http("https://old.invalid/"));
    let repo = format!("srv:\n{}", http("https://new.invalid/"));
    let found = merged_placed(
        vec![mcp_layer("personal", &personal), mcp_layer("repo", &repo)],
        &["repo"],
    );
    assert_eq!(
        lines(&found),
        [
            "warning [mcp] repo:mcp.yaml: server srv replaces the definition of layer personal, which lies outside the repository"
        ]
    );
    assert!(found.merged().is_some());
}

#[test]
fn a_replacement_between_layers_on_one_side_of_the_repository_gives_no_warning() {
    let first = format!("srv:\n{}", http("https://old.invalid/"));
    let second = format!("srv:\n{}", http("https://new.invalid/"));
    for inside in [&[][..], &["personal", "repo"][..]] {
        let found = merged_placed(
            vec![mcp_layer("personal", &first), mcp_layer("repo", &second)],
            inside,
        );
        assert_eq!(lines(&found), Vec::<String>::new(), "{inside:?}");
    }
    let found = merged_placed(
        vec![mcp_layer("personal", &first), mcp_layer("repo", &second)],
        &["personal"],
    );
    assert_eq!(lines(&found), Vec::<String>::new());
}
