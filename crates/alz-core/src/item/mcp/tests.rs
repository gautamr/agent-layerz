use crate::item::{EnvVarName, McpItems, McpServer, McpServerId, Transport};
use crate::merge::testing::layer;

/// What the layer `personal` with this `mcp.yaml` gives.
fn read(yaml: &str) -> (McpItems, Vec<String>) {
    let (items, found) = McpItems::read(&layer("personal", "personal", &[("mcp.yaml", yaml)]));
    (items, found.iter().map(ToString::to_string).collect())
}

fn id(text: &str) -> McpServerId {
    McpServerId::try_from(text).expect("a server id")
}

const STDIO: &str = "  transport: stdio\n  command: run-it\n";

#[test]
fn servers_are_read_into_an_ordered_map_of_ids() {
    let (items, found) = read(
        "playwright:\n  transport: stdio\n  command: npx\n  args: [\"@playwright/mcp@latest\"]\n  env_from: [FOO]\n\
         context7:\n  transport: http\n  url: https://mcp.context7.com/mcp\n\
         events:\n  transport: sse\n  url: https://example.invalid/sse\n",
    );
    assert_eq!(found, Vec::<String>::new());
    let ids: Vec<&str> = items.servers().keys().map(McpServerId::as_str).collect();
    assert_eq!(ids, ["context7", "events", "playwright"]);
    assert!(matches!(
        items.servers().get(&id("context7")).map(McpServer::transport),
        Some(Transport::Http { url }) if url == "https://mcp.context7.com/mcp"
    ));
    assert!(matches!(
        items.servers().get(&id("events")).map(McpServer::transport),
        Some(Transport::Sse { .. })
    ));
    match items
        .servers()
        .get(&id("playwright"))
        .map(McpServer::transport)
    {
        Some(Transport::Stdio {
            command,
            args,
            env_from,
        }) => {
            assert_eq!(command, "npx");
            assert_eq!(args, &Some(vec!["@playwright/mcp@latest".to_owned()]));
            assert_eq!(
                env_from.iter().map(EnvVarName::as_str).collect::<Vec<_>>(),
                ["FOO"]
            );
        }
        other => panic!("not a stdio server: {other:?}"),
    }
}

#[test]
fn a_layer_without_mcp_yaml_or_with_an_empty_one_has_no_servers() {
    let (items, found) = McpItems::read(&layer("personal", "personal", &[]));
    assert!(items.servers().is_empty());
    assert_eq!(found.iter().count(), 0);
    assert_eq!(read("").1, Vec::<String>::new());
}

#[test]
fn a_key_that_is_not_a_server_id_is_refused_naming_the_file_whatever_the_key() {
    let long = "a".repeat(65);
    for key in [
        "__proto__",
        "_x",
        "-x",
        "a b",
        "a.b",
        "a/b",
        "\u{e9}x",
        "Claude ",
        long.as_str(),
        "",
    ] {
        let yaml = format!("srv:\n{STDIO}srv2:\n{STDIO}\"{key}\":\n{STDIO}");
        let (items, found) = read(&yaml);
        assert_eq!(found.len(), 1, "{key:?}: {found:?}");
        assert!(
            found[0].starts_with("error [mcp] personal:mcp.yaml:"),
            "{key:?}: {found:?}"
        );
        assert!(found[0].contains(" is not a server id"), "{found:?}");
        assert!(found[0].contains('\''), "printed quoted: {found:?}");
        // The other servers of the file are still read; the refused key is never one.
        let ids: Vec<&str> = items.servers().keys().map(McpServerId::as_str).collect();
        assert_eq!(ids, ["srv", "srv2"], "{key:?}");
    }
}

#[test]
fn a_scalar_key_that_is_not_text_is_refused_by_its_kind() {
    for key in ["1", "true", "~"] {
        let (items, found) = read(&format!("{key}:\n{STDIO}"));
        assert!(items.servers().is_empty(), "{key}");
        assert_eq!(found.len(), 1, "{key}: {found:?}");
        assert!(
            found[0].starts_with("error [mcp] personal:mcp.yaml:1:1: a key that is "),
            "{key}: {found:?}"
        );
    }
}

#[test]
fn a_file_that_is_not_a_mapping_is_refused() {
    for yaml in ["- a\n", "text\n", "1\n"] {
        let (items, found) = read(yaml);
        assert!(items.servers().is_empty());
        assert_eq!(
            found,
            ["error [mcp] personal:mcp.yaml:1:1: the document must be a mapping"],
            "{yaml}"
        );
    }
}

#[test]
fn env_and_headers_are_refused_because_no_value_is_stored() {
    for field in ["env", "headers"] {
        let (items, found) = read(&format!("srv:\n{STDIO}  {field}:\n    TOKEN_NAME: value\n"));
        assert!(items.servers().is_empty(), "{field}");
        assert_eq!(found.len(), 1, "{field}: {found:?}");
        assert!(
            found[0].starts_with("error [mcp] personal:mcp.yaml:4:"),
            "{found:?}"
        );
        assert!(found[0].contains("server srv: values are never stored here"));
        assert!(!found[0].contains("value\n"));
    }
}

#[test]
fn a_field_the_schema_does_not_know_is_refused() {
    let (items, found) = read(&format!("srv:\n{STDIO}  extra: 1\n"));
    assert!(items.servers().is_empty());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("error [mcp] personal:mcp.yaml:"));
    assert!(found[0].contains("unknown field 'extra'"), "{found:?}");
}

#[test]
fn a_definition_that_is_not_a_mapping_is_refused() {
    let (items, found) = read("srv: run-it\n");
    assert!(items.servers().is_empty());
    assert_eq!(
        found,
        ["error [mcp] personal:mcp.yaml:1:6: server srv: the definition must be a mapping"]
    );
}

#[test]
fn the_transport_decides_which_fields_are_needed() {
    for (yaml, expected) in [
        ("srv:\n  command: x\n", "transport is required"),
        (
            "srv:\n  transport: tcp\n",
            "transport must be stdio, http or sse",
        ),
        ("srv:\n  transport: stdio\n", "command is required"),
        ("srv:\n  transport: http\n", "url is required"),
        ("srv:\n  transport: sse\n  command: x\n", "url is required"),
        (
            "srv:\n  transport: stdio\n  command: x\n  args: [a, {b: c}]\n",
            "args[1] must be text",
        ),
        (
            "srv:\n  transport: stdio\n  command: x\n  args: a\n",
            "args must be a list",
        ),
    ] {
        let (items, found) = read(yaml);
        assert!(items.servers().is_empty(), "{yaml}");
        assert_eq!(found.len(), 1, "{yaml}: {found:?}");
        assert!(
            found[0].starts_with("error [mcp] personal:mcp.yaml:"),
            "{found:?}"
        );
        assert!(found[0].ends_with(expected), "{yaml}: {found:?}");
    }
}

#[test]
fn env_from_belongs_to_a_stdio_server_only() {
    for transport in ["http", "sse"] {
        let (items, found) = read(&format!(
            "srv:\n  transport: {transport}\n  url: https://example.invalid/x\n  env_from: [FOO]\n"
        ));
        assert!(items.servers().is_empty());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("server srv: 'env_from' only applies to stdio servers"));
    }
    let (items, found) =
        read("srv:\n  transport: http\n  url: https://example.invalid/x\n  env_from: []\n");
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(items.servers().len(), 1);
}

#[test]
fn a_protected_variable_in_env_from_is_refused_and_the_server_is_remembered() {
    for name in ["HOME", "home", "CODEX_HOME", "CLAUDE_CONFIG_DIR"] {
        let (items, found) = read(&format!("srv:\n{STDIO}  env_from: [{name}]\n"));
        assert!(items.servers().is_empty(), "{name}");
        assert!(items.refused().contains(&id("srv")), "{name}");
        // The error names the file without a line and a column (spec 7.9, 10.1).
        assert_eq!(
            found,
            [format!(
                "error [protected-variable] personal:mcp.yaml: mcp.env_from names {}; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)",
                name.to_uppercase()
            )],
            "{name}"
        );
    }
}

#[test]
fn an_env_from_entry_that_is_not_a_plain_name_is_refused_as_a_variable_problem() {
    for name in ["\"CODEX_HOME=/x\"", "\"A B\"", "\"1A\"", "\"\""] {
        let (items, found) = read(&format!("srv:\n{STDIO}  env_from: [{name}]\n"));
        assert!(items.servers().is_empty(), "{name}");
        assert!(items.refused().contains(&id("srv")), "{name}");
        assert_eq!(found.len(), 1, "{name}: {found:?}");
        assert!(
            found[0].starts_with("error [protected-variable] "),
            "{found:?}"
        );
        assert!(!found[0].contains("/x"), "{found:?}");
    }
}

#[test]
fn an_env_from_of_other_variables_is_accepted() {
    let (items, found) = read(&format!("srv:\n{STDIO}  env_from: [FOO, bar_2]\n"));
    assert_eq!(found, Vec::<String>::new());
    assert!(items.refused().is_empty());
    assert_eq!(items.servers().len(), 1);
}

/// A value that looks like a token, assembled at run time.
fn token() -> String {
    format!("{}_{}", "ghp", "a".repeat(36))
}

#[test]
fn a_value_that_looks_like_a_token_is_refused_naming_the_kind_and_never_the_value() {
    let secret = token();
    for yaml in [
        format!("srv:\n  transport: http\n  url: https://example.invalid/?k={secret}\n"),
        format!("srv:\n{STDIO}  args: [--key, {secret}]\n"),
        format!("srv:\n  transport: stdio\n  command: {secret}\n"),
        format!("srv:\n{STDIO}  env_from: [{secret}]\n"),
    ] {
        let (items, found) = read(&yaml);
        assert!(items.servers().is_empty(), "{yaml}");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("error [leak-secret] personal:mcp.yaml:"));
        assert!(found[0].ends_with("server srv: a value looks like a GitHub token"));
        assert!(!found[0].contains(&secret), "{found:?}");
        // A token in env_from is a leak, not a protected variable: no mcp.enable suppression.
        assert!(items.refused().is_empty());
    }
}

#[test]
fn one_server_with_a_problem_does_not_hide_the_others() {
    let (items, found) = read(&format!(
        "bad:\n  transport: tcp\ngood:\n{STDIO}other:\n{STDIO}  env_from: [HOME]\n"
    ));
    let ids: Vec<&str> = items.servers().keys().map(McpServerId::as_str).collect();
    assert_eq!(ids, ["good"]);
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn a_duplicate_key_is_a_yaml_error_and_no_server_is_read() {
    let (items, found) = read(&format!("srv:\n{STDIO}srv:\n{STDIO}"));
    assert!(items.servers().is_empty());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("error [yaml] personal:mcp.yaml:"),
        "{found:?}"
    );
}
