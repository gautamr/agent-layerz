use proptest::prelude::{Strategy, any, prop_assert_eq, proptest};

use super::{
    AgentItems, AgentName, AgentRefusal, Isolation, Memory, OverlayMode, PermissionMode, SkillName,
};
use crate::markdown::MarkdownDoc;
use crate::merge::testing::layer;
use crate::merge::{ListEdit, OrderedSet};

fn read(files: &[(&str, &str)]) -> (AgentItems, Vec<String>) {
    let (items, problems) = AgentItems::read(&layer("personal", "personal", files));
    (items, problems.iter().map(ToString::to_string).collect())
}

fn skill(text: &str) -> SkillName {
    SkillName::try_from(text).expect("a skill name")
}

fn set(items: &[&str]) -> OrderedSet<SkillName> {
    items.iter().copied().map(skill).collect()
}

/// The edit of the `skills` field of an overlay whose frontmatter holds `field`.
fn edit_of(field: &str) -> Result<Option<ListEdit<SkillName>>, Vec<String>> {
    let doc = MarkdownDoc::parse(format!("---\n{field}\n---\n").as_bytes())
        .map_err(|error| vec![format!("{error:?}")])?;
    crate::item::list_edit::refine_edit::<SkillName>(
        &doc.front().section(),
        "skills",
        crate::merge::ListEdit::Replace,
    )
    .map_err(|errors| {
        errors
            .iter()
            .map(|error| error.message().to_string())
            .collect()
    })
}

fn apply(field: &str, base: &[&str]) -> Vec<String> {
    edit_of(field)
        .expect("an edit")
        .expect("a field")
        .apply(&set(base))
        .iter()
        .map(|skill| skill.as_str().to_owned())
        .collect()
}

#[test]
fn a_name_is_kebab_case() {
    assert!(AgentName::try_from("architect").is_ok());
    assert!(AgentName::try_from("a1-b").is_ok());
    for text in ["", "Architect", "a_b", "a--b", "-a", "a b"] {
        assert_eq!(
            AgentName::try_from(text),
            Err(AgentRefusal::Name),
            "{text:?}"
        );
    }
}

#[test]
fn the_words_of_the_closed_fields_are_those_of_the_spec_and_nothing_else() {
    assert_eq!(Isolation::try_from("worktree"), Ok(Isolation::Worktree));
    assert_eq!(Isolation::try_from("none"), Ok(Isolation::None));
    assert!(Isolation::try_from("Worktree").is_err());
    for word in ["none", "project", "user", "local"] {
        assert!(Memory::try_from(word).is_ok(), "{word}");
    }
    assert!(Memory::try_from("team").is_err());
    assert_eq!(
        PermissionMode::try_from("accept_edits"),
        Ok(PermissionMode::AcceptEdits)
    );
    assert!(PermissionMode::try_from("acceptEdits").is_err());
    assert_eq!(OverlayMode::try_from("replace"), Ok(OverlayMode::Replace));
    assert!(OverlayMode::try_from("append").is_err());
}

#[test]
fn a_base_and_an_overlay_are_read_with_their_fields() {
    let (items, problems) = read(&[
        (
            "agents/architect.md",
            "---\nname: architect\ndescription: Plans\ntools: [read, read, grep]\nskills: [a, b]\ntargets: [claude]\nclaude:\n  effort: high\n---\n\n  Plan.  \n",
        ),
        (
            "agents/architect.overlay.md",
            "---\nextends: architect\nallow_widen: true\nskills: +[c]\n---\nMore.\n",
        ),
        ("agents/notes.txt", "ignored"),
    ]);
    assert_eq!(problems, Vec::<String>::new());
    let [base] = items.bases() else {
        panic!("one base");
    };
    assert_eq!(base.name().as_str(), "architect");
    assert_eq!(base.description().as_str(), "Plans");
    assert_eq!(base.tools().map(OrderedSet::len), Some(2));
    assert_eq!(base.skills().len(), 2);
    assert_eq!(base.body(), "Plan.");
    assert!(!base.native().is_empty());
    let [overlay] = items.overlays() else {
        panic!("one overlay");
    };
    assert_eq!(overlay.extends().as_str(), "architect");
    assert_eq!(overlay.mode(), OverlayMode::Extend);
    assert!(overlay.allow_widen());
    assert_eq!(overlay.body(), "More.");
    assert!(overlay.tools().is_none());
}

#[test]
fn a_base_must_have_a_name_and_a_description_and_may_have_no_unknown_field() {
    let (items, problems) = read(&[("agents/a.md", "---\nrole: x\nname: A\nextends: a\n---\n")]);
    assert_eq!(items.bases(), []);
    assert_eq!(
        problems,
        [
            "error [agent] personal:agents/a.md:1:1: description is required",
            "error [agent] personal:agents/a.md:3:7: name must be kebab-case: lower-case letters and digits in groups joined by '-'",
            "error [agent] personal:agents/a.md:4:1: unknown field 'extends'",
        ]
    );
}

#[test]
fn an_unreadable_frontmatter_is_a_frontmatter_error() {
    let (items, problems) = read(&[("agents/a.md", "---\nname: a\n")]);
    assert_eq!(items.bases(), []);
    assert_eq!(
        problems,
        ["error [frontmatter] personal:agents/a.md: frontmatter is not closed with '---'"]
    );
}

#[test]
fn a_segment_appends_removes_or_replaces_and_segments_apply_in_the_order_written() {
    assert_eq!(apply("skills: +[c]", &["a", "b"]), ["a", "b", "c"]);
    assert_eq!(apply("skills: -[a]", &["a", "b"]), ["b"]);
    assert_eq!(apply("skills: \"[x, y]\"", &["a", "b"]), ["x", "y"]);
    assert_eq!(apply("skills: -[a] +[a]", &["a", "b"]), ["b", "a"]);
    assert_eq!(apply("skills: +[a] -[a]", &["b"]), ["b"]);
    assert_eq!(apply("skills: \"[x] +[y] -[x]\"", &["a"]), ["y"]);
    assert_eq!(
        apply("skills: \"+[\\\"q\\\", 'r' , ,s]\"", &[]),
        ["q", "r", "s"]
    );
}

#[test]
fn a_yaml_list_replaces_and_nothing_or_an_empty_string_changes_nothing() {
    assert_eq!(apply("skills: [x, y]", &["a"]), ["x", "y"]);
    assert_eq!(apply("skills: \"\"", &["a"]), ["a"]);
    assert_eq!(edit_of("role: x"), Ok(None));
    assert_eq!(edit_of("skills: ~"), Ok(None));
}

#[test]
fn text_that_is_not_an_edit_is_refused_and_so_is_an_item_that_is_not_a_name() {
    for text in [
        "+a", "+[a", "[a] b", "*[a]", "+ [a]", "+[a]]", " ", "  ", "\\t",
    ] {
        assert_eq!(
            edit_of(&format!("skills: \"{text}\"")),
            Err(vec![format!("skills {}", AgentRefusal::ListEdit.rule())]),
            "{text}"
        );
    }
    assert_eq!(
        edit_of("skills: +[good, Bad, Worse]").map_err(|errors| errors.len()),
        Err(2)
    );
    assert!(edit_of("skills: 3").is_err());
}

fn texts() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        (
            proptest::sample::select(vec!["+", "-", ""]),
            proptest::collection::vec(proptest::sample::select(vec!["a", "b", "c", "d"]), 0..4),
        ),
        0..4,
    )
    .prop_map(|segments| {
        segments
            .into_iter()
            .map(|(sign, items)| format!("{sign}[{}]", items.join(", ")))
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn edit(text: &str) -> ListEdit<SkillName> {
    edit_of(&format!("skills: \"{text}\""))
        .expect("an edit")
        .expect("a field")
}

proptest! {
    #[test]
    fn writing_edits_one_after_the_other_applies_them_one_after_the_other(
        base in proptest::collection::vec(proptest::sample::select(vec!["a", "b", "c", "d"]), 0..5),
        first in texts(),
        second in texts(),
    ) {
        let base: OrderedSet<SkillName> = base.into_iter().map(skill).collect();
        // A text of white space alone is no edit (the reference refuses it), so an empty part
        // adds no separator.
        let joined = [first.as_str(), second.as_str()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let together = edit(&joined).apply(&base);
        let one_by_one = edit(&second).apply(&edit(&first).apply(&base));
        prop_assert_eq!(together, one_by_one);
    }

    #[test]
    fn composing_the_edits_of_a_stack_is_associative_and_applying_the_composition_is_applying_each(
        base in proptest::collection::vec(proptest::sample::select(vec!["a", "b", "c", "d"]), 0..5),
        a in texts(),
        b in texts(),
        c in texts(),
    ) {
        let base: OrderedSet<SkillName> = base.into_iter().map(skill).collect();
        let (a, b, c) = (edit(&a), edit(&b), edit(&c));
        let left = a.clone().merge(b.clone()).merge(c.clone());
        let right = a.clone().merge(b.clone().merge(c.clone()));
        prop_assert_eq!(left.apply(&base), right.apply(&base));
        prop_assert_eq!(&left, &right);
        prop_assert_eq!(
            left.apply(&base),
            c.apply(&b.apply(&a.apply(&base)))
        );
    }

    #[test]
    fn no_text_makes_the_segment_reader_panic(text in any::<String>()) {
        let _ = crate::item::list_edit::segments(&text);
    }
}

// Native fields and the variables no layer may set (§7.9, §7.2).

/// The agent file `helper.md` (or the overlay) whose frontmatter ends with `native`.
fn helper(native: &str) -> String {
    format!("---\nname: helper\ndescription: A helper.\n{native}---\nBody\n")
}

fn helper_overlay(native: &str) -> String {
    format!("---\nextends: helper\n{native}---\nBody\n")
}

/// A label, the key path a refusal names, and the native fields written for a variable.
type NativeForm = (&'static str, &'static str, fn(&str) -> String);

const NATIVE_FORMS: [NativeForm; 1] = [(
    "codex.shell_environment_policy",
    "codex.shell_environment_policy.set",
    |v| format!("codex:\n  shell_environment_policy:\n    set:\n      {v}: x\n"),
)];

fn protected(path: &str, file: &str, variable: &str) -> String {
    format!(
        "error [protected-variable] personal:{file}: {path} names {variable}; no layer may set the variables that choose an account folder or locate alz's files (\u{a7}7.9)"
    )
}

#[test]
fn a_native_field_that_sets_a_protected_variable_is_refused_in_an_agent_and_in_an_overlay() {
    for (label, path, native) in NATIVE_FORMS {
        for variable in crate::account::PROTECTED_VARIABLES {
            let (items, found) = read(&[("agents/helper.md", &helper(&native(variable)))]);
            assert_eq!(
                found,
                [protected(path, "agents/helper.md", variable)],
                "{label}, {variable}"
            );
            assert!(items.bases().is_empty(), "{label}, {variable}");
            let (items, found) = read(&[(
                "agents/helper.overlay.md",
                &helper_overlay(&native(variable)),
            )]);
            assert_eq!(
                found,
                [protected(path, "agents/helper.overlay.md", variable)],
                "{label}, {variable}"
            );
            assert!(items.overlays().is_empty(), "{label}, {variable}");
        }
    }
}

#[test]
fn a_native_variable_is_matched_in_any_ascii_case_and_never_printed_as_written() {
    let (_, found) = read(&[(
        "agents/helper.md",
        &helper("codex:\n  shell_environment_policy:\n    set:\n      Xdg_Cache_Home: x\n"),
    )]);
    assert_eq!(
        found,
        [protected(
            "codex.shell_environment_policy.set",
            "agents/helper.md",
            "XDG_CACHE_HOME"
        )]
    );
    assert!(found.iter().all(|line| !line.contains("Xdg_Cache_Home")));
}

#[test]
fn a_native_variable_that_is_not_a_plain_name_or_cannot_be_checked_is_refused() {
    let (_, found) = read(&[(
        "agents/helper.md",
        &helper("codex:\n  shell_environment_policy:\n    set:\n      \"HOME=/x\": y\n"),
    )]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("is not a plain variable name")
            || found[0].contains("not a plain variable name"),
        "{found:?}"
    );
}

#[test]
fn native_fields_that_set_other_variables_are_read() {
    let (items, found) = read(&[(
        "agents/helper.md",
        &helper(
            "claude:\n  effort: low\ncodex:\n  shell_environment_policy:\n    inherit: core\n    include_only: [HOME, PATH]\n",
        ),
    )]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(items.bases().len(), 1);
}

#[test]
fn servers_and_hooks_in_an_agents_native_fields_are_refused_in_an_agent_and_an_overlay() {
    for (native, key, owner) in [
        (
            "claude:\n  mcpServers:\n    s: {}\n",
            "claude.mcpServers",
            "mcp.yaml",
        ),
        (
            "claude:\n  hooks:\n    Stop: []\n",
            "claude.hooks",
            "hooks/hooks.yaml",
        ),
        (
            "codex:\n  mcp_servers:\n    s: {}\n",
            "codex.mcp_servers",
            "mcp.yaml",
        ),
        ("codex:\n  hooks: []\n", "codex.hooks", "hooks/hooks.yaml"),
    ] {
        let (items, found) = read(&[("agents/helper.md", &helper(native))]);
        assert_eq!(found.len(), 1, "{native}: {found:?}");
        assert!(
            found[0].starts_with("error [agent] personal:agents/helper.md:")
                && found[0].ends_with(&format!(
                    "'{key}' is set through {owner} so every agent gets the same servers and hooks"
                )),
            "{native}: {found:?}"
        );
        assert!(items.bases().is_empty(), "{native}");
        let (items, found) = read(&[("agents/helper.overlay.md", &helper_overlay(native))]);
        assert_eq!(found.len(), 1, "{native}: {found:?}");
        assert!(
            found[0].starts_with("error [agent] personal:agents/helper.overlay.md:")
                && found[0].ends_with(&format!(
                    "'{key}' is set through {owner} so every agent gets the same servers and hooks"
                )),
            "{native}: {found:?}"
        );
        assert!(items.overlays().is_empty(), "{native}");
    }
}

#[test]
fn other_native_keys_with_a_similar_name_are_read() {
    let (items, found) = read(&[(
        "agents/helper.md",
        &helper(
            "claude:\n  maxTurns: 3\ncodex:\n  mcpServersX: 1\n  nested:\n    hooks: 1\n  features:\n    hooks: true\n",
        ),
    )]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(items.bases().len(), 1);
}

#[test]
fn a_native_policy_of_another_type_is_an_agent_error_naming_the_key() {
    let native = "codex:\n  shell_environment_policy: HOME\n";
    let (items, found) = read(&[("agents/helper.md", &helper(native))]);
    assert_eq!(
        found,
        [
            "error [agent] personal:agents/helper.md: codex.shell_environment_policy must be a mapping"
        ]
    );
    assert_eq!(items.bases().len(), 0);
}

// The types of the Claude frontmatter fields (§7.2, TASK-79).

#[test]
fn a_claude_field_of_the_wrong_type_is_an_agent_error_naming_the_key_path_and_never_the_value() {
    for (native, expected) in [
        (
            "claude:\n  maxTurns: secret-value\n",
            "error [agent] personal:agents/helper.md:5:13: 'claude.maxTurns' must be a whole number",
        ),
        (
            "claude:\n  autoCompactWindow: 1.5\n",
            "error [agent] personal:agents/helper.md:5:22: 'claude.autoCompactWindow' must be a whole number",
        ),
        (
            "claude:\n  maxTurns: null\n",
            "error [agent] personal:agents/helper.md:5:13: 'claude.maxTurns' must be a whole number",
        ),
        (
            "claude:\n  background: secret-value\n",
            "error [agent] personal:agents/helper.md:5:15: 'claude.background' must be a boolean",
        ),
        (
            "claude:\n  omitClaudeMd: secret-value\n",
            "error [agent] personal:agents/helper.md:5:17: 'claude.omitClaudeMd' must be a boolean",
        ),
        (
            "claude:\n  effort: secret-value\n",
            "error [agent] personal:agents/helper.md:5:11: 'claude.effort' must be one of low, medium, high, xhigh, max",
        ),
        (
            "claude:\n  color: secret-value\n",
            "error [agent] personal:agents/helper.md:5:10: 'claude.color' must be one of red, blue, green, yellow, purple, orange, pink, cyan",
        ),
        (
            "claude:\n  initialPrompt: [secret-value]\n",
            "error [agent] personal:agents/helper.md:5:18: 'claude.initialPrompt' must be text",
        ),
        (
            "claude:\n  disallowedTools: 12\n",
            "error [agent] personal:agents/helper.md:5:20: 'claude.disallowedTools' must be text, or a list whose items are text",
        ),
        (
            "claude:\n  experimental:\n    cacheTtl: secret-value\n",
            "error [agent] personal:agents/helper.md:6:15: 'claude.experimental.cacheTtl' must be one of 5m, 1h",
        ),
    ] {
        let (_, found) = read(&[("agents/helper.md", &helper(native))]);
        assert_eq!(found.len(), 1, "{native}: {found:?}");
        assert!(!found[0].contains("secret-value"), "{found:?}");
        assert_eq!(found, [expected], "{native}");
        let (_, found) = read(&[("agents/helper.overlay.md", &helper_overlay(native))]);
        assert_eq!(found.len(), 1, "{native}: {found:?}");
        assert!(found[0].contains("must be"), "{found:?}");
        assert!(!found[0].contains("secret-value"), "{found:?}");
    }
}

#[test]
fn a_claude_field_that_claude_code_does_not_read_from_frontmatter_is_an_agent_error() {
    let (items, found) = read(&[(
        "agents/helper.md",
        &helper("claude:\n  maxTurn: 3\n  experimental:\n    other: 1\n"),
    )]);
    assert_eq!(
        found,
        [
            "error [agent] personal:agents/helper.md:5:3: 'claude.maxTurn' is not a field of a subagent's frontmatter that alz accepts (docs/target-facts.md section 14)",
            "error [agent] personal:agents/helper.md:7:5: 'claude.experimental.other' is not a field of a subagent's frontmatter that alz accepts (docs/target-facts.md section 14)",
        ]
    );
    assert_eq!(items.bases().len(), 0);
}

#[test]
fn a_neutral_key_and_a_first_class_key_are_reported_by_their_own_checks_only() {
    let (_, found) = read(&[(
        "agents/helper.md",
        &helper("claude:\n  model: x\n  hooks: 1\n  mcpServers: 1\n"),
    )]);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.iter().all(|line| !line.contains("alz accepts")));
}

#[test]
fn every_claude_frontmatter_field_accepts_a_value_of_its_type() {
    let (items, found) = read(&[(
        "agents/helper.md",
        &helper(
            "claude:\n  disallowedTools: [Bash, Write]\n  maxTurns: 5\n  autoCompactWindow: 100000\n  background: true\n  omitClaudeMd: false\n  effort: max\n  color: cyan\n  initialPrompt: Start\n  experimental:\n    cacheTtl: 1h\n",
        ),
    )]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(items.bases().len(), 1);
    let (_, found) = read(&[(
        "agents/helper.md",
        &helper("claude:\n  disallowedTools: Bash, Write\n"),
    )]);
    assert_eq!(found, Vec::<String>::new());
}

#[test]
fn codex_native_fields_are_not_typed_here() {
    let (_, found) = read(&[(
        "agents/helper.md",
        &helper("codex:\n  anything: [1, {a: b}]\n  maxTurns: x\n"),
    )]);
    assert_eq!(found, Vec::<String>::new());
}
