use super::{Built, Comparison};
use crate::probe::Probe;

fn built() -> Built {
    Built::from_names(
        ["architect".to_owned()],
        ["review".to_owned()],
        ["docs".to_owned()],
    )
}

fn probe(agents: &str, skills: &str, servers: &str) -> Probe {
    Probe::parse(
        &format!(
            r#"{{"type":"system","subtype":"init","agents":[{agents}],"skills":[{skills}],"mcp_servers":[{servers}]}}"#
        ),
        "warned\n",
    )
    .expect("an init event")
}

const DOCS_UP: &str = r#"{"name":"docs","status":"connected","source":"project"}"#;

#[test]
fn everything_loaded_matches_and_shows_what_is_not_from_layers() {
    let found = Comparison::of(
        &built(),
        &probe(
            r#""architect","Explore""#,
            r#""review","plug:x","extra""#,
            DOCS_UP,
        ),
    );
    assert!(found.matches());
    assert_eq!(
        found.to_string(),
        "claude loaded 2 agents, 3 skills and commands\n  not from layers, agents: Explore\n  not from layers, skills: extra\nls --live: Claude matches the build\n"
    );
    assert_eq!(found.diagnostics().warning_count(), 1);
    assert_eq!(found.diagnostics().to_string(), "warning [claude] warned\n");
}

#[test]
fn an_agent_that_did_not_load_is_an_error_naming_it() {
    let found = Comparison::of(&built(), &probe(r#""Explore""#, r#""review""#, DOCS_UP));
    assert!(!found.matches());
    assert!(
        found
            .diagnostics()
            .to_string()
            .contains("error [ls] agent 'architect' was built but Claude did not load it\n")
    );
    assert!(
        found
            .to_string()
            .ends_with("ls --live: Claude does not match the build\n")
    );
}

#[test]
fn a_skill_that_did_not_load_is_an_error() {
    let found = Comparison::of(&built(), &probe(r#""architect""#, "", DOCS_UP));
    assert!(
        found
            .diagnostics()
            .to_string()
            .contains("error [ls] skill 'review' was built but Claude did not load it\n")
    );
}

#[test]
fn a_server_claude_did_not_start_is_an_error_and_one_not_connected_a_warning() {
    let none = Comparison::of(&built(), &probe(r#""architect""#, r#""review""#, ""));
    assert!(
        none.diagnostics()
            .to_string()
            .contains("error [ls] mcp 'docs' is enabled but Claude did not start it\n")
    );
    let user_scope = r#"{"name":"docs","status":"connected","source":"user"}"#;
    let wrong_scope = Comparison::of(
        &built(),
        &probe(r#""architect""#, r#""review""#, user_scope),
    );
    assert!(!wrong_scope.matches());
    let failing = r#"{"name":"docs","status":"failed","source":"project"}"#;
    let down = Comparison::of(&built(), &probe(r#""architect""#, r#""review""#, failing));
    assert!(down.matches());
    assert!(
        down.diagnostics()
            .to_string()
            .contains("warning [ls] mcp 'docs' is failed\n")
    );
}
