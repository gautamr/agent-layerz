use crate::account::PROTECTED_VARIABLES;
use crate::diagnostic::{Diagnostics, Where};
use crate::item::{PassthroughFile, PassthroughItems};
use crate::merge::testing::layer;
use crate::stack::ResolvedLayer;
use crate::target::TargetName;
use crate::untrusted_text::Stripped;

use super::check;

const VALUE: &str = "canary-value-77";

fn personal(files: &[(&str, &str)]) -> ResolvedLayer {
    layer("personal", "personal", files)
}

/// What the layer `personal` with one file gives: its items and its diagnostics, one line each.
fn read(files: &[(&str, &str)]) -> (PassthroughItems, Vec<String>) {
    let (items, found) = PassthroughItems::read(&personal(files));
    (items, found.iter().map(ToString::to_string).collect())
}

fn errors(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .filter(|line| line.starts_with("error "))
        .collect()
}

/// The refusal of `name` at `path`, as one line.
fn names(file: &str, path: &str, variable: &str) -> String {
    format!(
        "error [protected-variable] personal:{file}: {path} names {variable}; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)"
    )
}

/// One passthrough form of the acceptance cases of §7.9 (test 24), written for the variable `name`.
type Form = (&'static str, &'static str, fn(&str) -> String);

fn forms() -> Vec<Form> {
    json_forms()
        .into_iter()
        .chain(toml_and_agent_forms())
        .collect()
}

fn json_forms() -> Vec<Form> {
    vec![
        (
            "passthrough settings.json",
            "targets/claude/.claude/settings.json",
            |v| format!("{{\"env\": {{\"{v}\": \"{VALUE}\"}}}}\n"),
        ),
        (
            "passthrough settings.local.json",
            "targets/claude/.claude/settings.local.json",
            |v| format!("{{\"env\": {{\"{v}\": \"{VALUE}\"}}}}\n"),
        ),
        ("passthrough .mcp.json", "targets/claude/.mcp.json", |v| {
            format!("{{\"mcpServers\": {{\"s\": {{\"env\": {{\"{v}\": \"{VALUE}\"}}}}}}}}\n")
        }),
        (
            "passthrough with a case-variant path",
            "targets/claude/.Claude/Settings.JSON",
            |v| format!("{{\"env\": {{\"{v}\": \"{VALUE}\"}}}}\n"),
        ),
        (
            "passthrough config.toml, policy",
            "targets/codex/.codex/config.toml",
            |v| format!("[shell_environment_policy.set]\n{v} = \"{VALUE}\"\n"),
        ),
        (
            "passthrough config.toml, MCP environment",
            "targets/codex/.codex/config.toml",
            |v| format!("[mcp_servers.s.env]\n{v} = \"{VALUE}\"\n"),
        ),
        (
            "passthrough config.toml, case-variant path",
            "targets/codex/.Codex/Config.toml",
            |v| format!("[mcp_servers.s.env]\n{} = \"{VALUE}\"\n", v.to_lowercase()),
        ),
        (
            "passthrough settings.json at any depth",
            "targets/claude/sub/.claude/settings.json",
            |v| format!("{{\"env\": {{\"{v}\": \"{VALUE}\"}}}}\n"),
        ),
        (
            "passthrough .mcp.json at any depth",
            "targets/claude/sub/dir/.mcp.json",
            |v| format!("{{\"mcpServers\": {{\"s\": {{\"env\": {{\"{v}\": \"{VALUE}\"}}}}}}}}\n"),
        ),
        (
            "passthrough plugin manifest",
            "targets/claude/plugin/.claude-plugin/plugin.json",
            |v| {
                format!(
                    "{{\"name\": \"p\", \"mcpServers\": {{\"s\": {{\"command\": \"x\", \"env\": {{\"{v}\": \"{VALUE}\"}}}}}}}}\n"
                )
            },
        ),
        (
            "passthrough plugin file with a nested env",
            "targets/claude/plugin/.claude-plugin/marketplace.json",
            |v| {
                format!(
                    "{{\"plugins\": [{{\"name\": \"p\", \"mcpServers\": {{\"s\": {{\"env\": {{\"{v}\": \"{VALUE}\"}}}}}}}}]}}\n"
                )
            },
        ),
    ]
}

fn toml_and_agent_forms() -> Vec<Form> {
    vec![
        (
            "passthrough Codex role file",
            "targets/codex/.codex/agents/worker.toml",
            |v| format!("[mcp_servers.s.env]\n{v} = \"{VALUE}\"\n"),
        ),
        (
            "passthrough .codex/config.toml under the claude target",
            "targets/claude/.codex/config.toml",
            |v| format!("[shell_environment_policy.set]\n{v} = \"{VALUE}\"\n"),
        ),
        (
            "passthrough toml of any name under the claude target",
            "targets/claude/deep/worker.toml",
            |v| format!("[mcp_servers.s.env]\n{v} = \"{VALUE}\"\n"),
        ),
        (
            "passthrough .claude/agents file under the codex target",
            "targets/codex/.claude/agents/helper.md",
            |v| {
                format!(
                    "---\nname: helper\nmcpServers:\n  s:\n    env:\n      {v}: {VALUE}\n---\nBody\n"
                )
            },
        ),
        (
            "passthrough .claude/agents file at any depth",
            "targets/claude/.claude/agents/team/helper.md",
            |v| {
                format!(
                    "---\nname: helper\nmcpServers:\n  s:\n    env:\n      {v}: {VALUE}\n---\nBody\n"
                )
            },
        ),
        (
            "passthrough Codex toml outside .codex",
            "targets/codex/worker.toml",
            |v| format!("[mcp_servers.s.env]\n{v} = \"{VALUE}\"\n"),
        ),
        (
            "passthrough Claude agent, mcpServers list",
            "targets/claude/.claude/agents/helper.md",
            |v| {
                format!(
                    "---\nname: helper\nmcpServers:\n  - plain-name\n  - s:\n      env:\n        {v}: {VALUE}\n---\nBody\n"
                )
            },
        ),
    ]
}

#[test]
fn every_passthrough_form_is_refused_for_every_protected_variable_and_never_prints_a_value() {
    for (label, file, text) in forms() {
        for variable in PROTECTED_VARIABLES {
            let (items, lines) = read(&[(file, &text(variable))]);
            let found = errors(&lines);
            assert!(
                found
                    .iter()
                    .any(|line| line.contains(&format!(" names {variable}; "))),
                "{label}, {variable}: {lines:?}"
            );
            assert!(
                found.iter().all(|line| line
                    .starts_with(&format!("error [protected-variable] personal:{file}: "))),
                "{label}, {variable}: {lines:?}"
            );
            assert!(
                lines.iter().all(|line| !line.contains("canary")),
                "{label}, {variable}: {lines:?}"
            );
            assert_eq!(items.files(), [], "{label}, {variable}");
        }
    }
}

#[test]
fn the_variable_is_named_in_its_fixed_spelling_whatever_the_layer_wrote() {
    let (_, lines) = read(&[(
        "targets/claude/.claude/settings.json",
        "{\"env\": {\"codex_home\": \"x\", \"xdg_Cache_Home\": \"x\"}}",
    )]);
    assert_eq!(
        lines,
        [
            names("targets/claude/.claude/settings.json", "env", "CODEX_HOME"),
            names(
                "targets/claude/.claude/settings.json",
                "env",
                "XDG_CACHE_HOME"
            ),
        ]
    );
}

#[test]
fn a_variable_name_that_is_not_a_plain_name_is_refused_without_printing_it() {
    for (file, text) in [
        (
            "targets/claude/.claude/settings.json",
            "{\"env\": {\"CODEX_HOME=/x\": \"v\"}}",
        ),
        (
            "targets/claude/.claude/settings.json",
            "{\"env\": {\"CODEX_HOME\\u0000X\": \"v\"}}",
        ),
        (
            "targets/claude/.mcp.json",
            "{\"mcpServers\": {\"s\": {\"env\": {\"CODEX_HOME=/x\": \"v\"}}}}",
        ),
    ] {
        let (items, lines) = read(&[(file, text)]);
        let found = errors(&lines);
        assert_eq!(found.len(), 1, "{file}: {lines:?}");
        assert!(
            found
                .iter()
                .all(|line| line.contains("not a plain variable name"))
        );
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("/x") && !line.contains('='))
        );
        assert_eq!(items.files(), []);
    }
}

#[test]
fn a_path_with_a_non_ascii_character_is_refused_for_every_target() {
    for file in [
        "targets/claude/.claude/\u{17f}ettings.json",
        "targets/claude/.claude/caf\u{e9}.md",
        "targets/codex/.codex/caf\u{e9}.toml",
        "targets/claude/\u{e9}/.mcp.json",
        "targets/claude/.mcp.j\u{17f}on",
        "targets/claude/docs/caf\u{e9}.md",
        "targets/codex/docs/caf\u{e9}.md",
    ] {
        let (items, lines) = read(&[(file, "{}\n")]);
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("error [protected-variable] ")
                    && line
                        .contains("this path has a non-ASCII character, so it cannot be checked")),
            "{file}: {lines:?}"
        );
        assert_eq!(items.files(), [], "{file}");
    }
}

#[test]
fn a_non_ascii_target_folder_is_an_unknown_folder_and_a_non_ascii_path() {
    let (items, lines) = read(&[("targets/cl\u{e4}ude/x.md", "{}\n")]);
    assert_eq!(items.files(), []);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("error [passthrough] "))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("error [protected-variable] ")
                && line.contains("has a non-ASCII character"))
    );
}

#[test]
fn a_covered_file_that_cannot_be_parsed_is_refused_because_it_cannot_be_checked() {
    for (label, file, text) in [
        (
            "a JSON settings file that does not parse",
            "targets/claude/.claude/settings.json",
            "not json",
        ),
        (
            "a JSON file with a duplicate key",
            "targets/claude/.mcp.json",
            "{\"env\": {}, \"env\": {}}",
        ),
        (
            "a passthrough settings env that is a list",
            "targets/claude/.claude/settings.json",
            "{\"env\": [\"HOME\"]}\n",
        ),
        (
            "a TOML \\u escape",
            "targets/codex/.codex/config.toml",
            "\"\\u0048OME\" = \"x\"\n",
        ),
        (
            "a TOML \\U escape",
            "targets/codex/.codex/config.toml",
            "\"\\U00000048OME\" = \"x\"\n",
        ),
        (
            "a TOML \\x escape",
            "targets/codex/.codex/config.toml",
            "\"\\x48OME\" = \"x\"\n",
        ),
        (
            "a TOML line-ending backslash that joins a name",
            "targets/codex/.codex/config.toml",
            "x = \"\"\"CODEX_\\\nHOME\"\"\"\n",
        ),
        (
            "a Claude agent file whose frontmatter does not parse",
            "targets/claude/.claude/agents/helper.md",
            "---\nname: [\n---\nBody\n",
        ),
        (
            "a Claude agent file whose frontmatter is not closed",
            "targets/claude/.claude/agents/helper.md",
            "---\nname: helper\n",
        ),
    ] {
        let (items, lines) = read(&[(file, text)]);
        assert!(
            errors(&lines).iter().any(|line| line
                .starts_with(&format!("error [protected-variable] personal:{file}: "))
                && line.contains("so it cannot be checked")),
            "{label}: {lines:?}"
        );
        assert_eq!(items.files(), [], "{label}");
    }
}

#[test]
fn a_toml_file_that_is_not_utf8_cannot_be_checked() {
    let at = Where::layer_file(
        Stripped::from_untrusted("personal"),
        Stripped::from_untrusted("targets/codex/.codex/config.toml"),
    );
    let found: Diagnostics = check::problems(".codex/config.toml", &[0xff, 0xfe], &at);
    assert_eq!(
        found.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "error [protected-variable] personal:targets/codex/.codex/config.toml: this .toml file is not valid UTF-8, so it cannot be checked (\u{a7}7.9)"
        ]
    );
}

#[test]
fn files_that_name_none_of_the_variables_still_build_with_one_warning_each() {
    for (file, text) in [
        (
            "targets/claude/.claude/settings.json",
            "{\"env\": {\"FOO\": \"x\"}}\n",
        ),
        (
            "targets/codex/.codex/config.toml",
            "model = \"x\"\nurl = \"https://example.invalid/homepage\"\n",
        ),
        // A plugin manifest may name the file that holds its servers; that file is not read.
        (
            "targets/claude/p/.claude-plugin/plugin.json",
            "{\"name\": \"p\", \"mcpServers\": \"./servers.json\"}\n",
        ),
        // A JSON file that is not an object holds nothing to check.
        ("targets/claude/.mcp.json", "[1, 2]\n"),
        (
            "targets/claude/.claude/agents/helper.md",
            "---\nname: helper\n---\nBody\n",
        ),
        (
            "targets/claude/.claude/agents/plain.md",
            "No frontmatter.\n",
        ),
        ("targets/claude/.claude/output-styles/x.md", "Anything\n"),
    ] {
        let (items, lines) = read(&[(file, text)]);
        let inner = file
            .strip_prefix("targets/")
            .and_then(|rest| rest.split_once('/'))
            .map(|(_, inner)| inner);
        assert_eq!(
            lines,
            [format!(
                "warning [passthrough] personal:{file}: raw passthrough to {}; consider turning it into a first-class item",
                inner.unwrap_or_default()
            )],
            "{file}"
        );
        assert_eq!(items.files().len(), 1, "{file}");
    }
}

#[test]
fn a_file_is_kept_byte_for_byte_with_its_target_and_paths() {
    let (items, _) = read(&[
        ("targets/codex/.codex/prompts/a.md", "Hello\r\n"),
        ("targets/claude/.claude/output-styles/x.md", "Style\n"),
    ]);
    let found: Vec<(&str, &str, &str, &[u8])> = items
        .files()
        .iter()
        .map(|file| {
            (
                file.target().as_str(),
                file.path().as_str(),
                file.source().as_str(),
                file.bytes(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            (
                "claude",
                ".claude/output-styles/x.md",
                "targets/claude/.claude/output-styles/x.md",
                b"Style\n".as_slice()
            ),
            (
                "codex",
                ".codex/prompts/a.md",
                "targets/codex/.codex/prompts/a.md",
                b"Hello\r\n".as_slice()
            ),
        ]
    );
    assert_eq!(
        items
            .files()
            .iter()
            .map(PassthroughFile::target)
            .collect::<Vec<_>>(),
        [TargetName::Claude, TargetName::Codex]
    );
}

#[test]
fn a_file_below_a_folder_that_is_not_a_target_is_an_error_naming_the_file() {
    let (items, lines) = read(&[("targets/claud/x.json", "{}\n")]);
    assert_eq!(
        lines,
        [
            "error [passthrough] personal:targets/claud/x.json: this file is below a folder of targets/ that is not a target: claude or codex (\u{a7}7.11)"
        ]
    );
    assert_eq!(items.files(), []);
}

#[test]
fn every_file_below_an_unknown_folder_is_named_even_deep_down() {
    let (_, lines) = read(&[
        ("targets/claud/a/b/c.md", "x"),
        ("targets/codex/ok.md", "x"),
        ("targets/other/y.md", "x"),
    ]);
    let errs: Vec<&String> = errors(&lines);
    assert_eq!(errs.len(), 2, "{lines:?}");
    assert!(errs[0].contains("personal:targets/claud/a/b/c.md:"));
    assert!(errs[1].contains("personal:targets/other/y.md:"));
}

#[test]
fn a_file_directly_in_targets_is_no_item_and_gives_no_diagnostic() {
    let (items, lines) = read(&[
        ("targets/README.md", "notes"),
        ("targets/claude", "a file named like a target"),
    ]);
    assert_eq!(lines, Vec::<String>::new());
    assert_eq!(items.files(), []);
}

#[test]
fn claude_extra_and_other_paths_are_not_passthrough_files() {
    let (items, lines) = read(&[
        ("targets/claude/CLAUDE.extra.md", "extra"),
        ("README.md", "notes"),
        ("notes/targets/x.md", "notes"),
    ]);
    assert_eq!(lines, Vec::<String>::new());
    assert_eq!(items.files(), []);
}

#[test]
fn claude_extra_below_the_codex_target_is_an_ordinary_passthrough_file() {
    let (items, lines) = read(&[("targets/codex/CLAUDE.extra.md", "extra")]);
    assert_eq!(items.files().len(), 1);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("warning [passthrough] "));
}

#[test]
fn every_problem_of_every_file_is_reported_in_one_pass() {
    let (items, lines) = read(&[
        (
            "targets/claude/.claude/settings.json",
            "{\"env\": {\"HOME\": \"x\"}}",
        ),
        ("targets/claude/.mcp.json", "not json"),
        ("targets/codex/ok.md", "fine"),
    ]);
    assert_eq!(errors(&lines).len(), 2, "{lines:?}");
    assert_eq!(items.files().len(), 1);
}
