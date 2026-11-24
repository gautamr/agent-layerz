use super::Probe;
use crate::untrusted_text::Stripped;

const INIT: &str = r#"{"type":"system","subtype":"init","agents":["architect","Explore"],"skills":["review"],"slash_commands":["review","deploy"],"mcp_servers":[{"name":"docs","status":"connected","source":"project"},{"name":"x","status":"failed","source":"user"},{"status":"bad"}]}"#;

#[test]
fn the_init_event_gives_agents_skills_and_servers() {
    let probe = Probe::parse(&format!("{INIT}\n"), "").expect("an init event");
    assert_eq!(
        probe
            .agents()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["Explore", "architect"]
    );
    assert_eq!(
        probe
            .skills()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["deploy", "review"]
    );
    assert_eq!(probe.mcp().len(), 2);
    assert!(
        probe
            .mcp()
            .first()
            .is_some_and(|server| server.is_project() && server.is_connected())
    );
    assert!(
        probe
            .mcp()
            .get(1)
            .is_some_and(|server| !server.is_project() && !server.is_connected())
    );
}

#[test]
fn lines_that_are_not_json_are_warnings_standard_error_first() {
    let stdout = format!("noise on stdout\n{INIT}\n{{\"type\":\"result\"}}\n");
    let probe = Probe::parse(&stdout, "  Ignoring one entry  \n\n").expect("an init event");
    let lines: Vec<&str> = probe.warnings().iter().map(Stripped::as_str).collect();
    assert_eq!(lines, ["Ignoring one entry", "noise on stdout"]);
}

#[test]
fn escape_sequences_never_survive_in_a_warning_or_a_status() {
    let event = "{\"type\":\"system\",\"subtype\":\"init\",\"mcp_servers\":[{\"name\":\"a\",\"status\":\"ok\\u001b[31m\\u202e\",\"source\":\"project\"}]}";
    let probe = Probe::parse(event, "\u{1b}[2J\u{1b}]0;title\u{7}hello\u{200b}").expect("init");
    assert_eq!(
        probe.mcp().first().map(|server| server.status().as_str()),
        Some("ok[31m")
    );
    assert_eq!(
        probe.warnings().first().map(Stripped::as_str),
        Some("[2J]0;titlehello")
    );
}

#[test]
fn output_without_an_init_event_is_refused_with_the_first_line_of_standard_error() {
    let error = Probe::parse(
        "{\"type\":\"result\"}\nnot json",
        "\u{1b}[1mNot logged in\nsecond",
    )
    .expect_err("no init event");
    assert_eq!(error.first_line().as_str(), "[1mNot logged in");
}

#[test]
fn an_init_event_of_the_wrong_shape_loads_nothing_and_does_not_fail() {
    let probe = Probe::parse(
        r#"{"type":"system","subtype":"init","agents":"x","skills":[1,null,"s"],"mcp_servers":7}"#,
        "",
    )
    .expect("an init event");
    assert!(probe.agents().is_empty());
    assert_eq!(probe.skills().len(), 1);
    assert_eq!(probe.mcp(), []);
}
