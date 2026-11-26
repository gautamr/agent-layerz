//! The Claude adapter on a model built by hand (spec §8.0, §8.1): what needs no merge. The adapter
//! on whole stacks, with the golden files of the fixtures, is tested in `alz-core`, which owns the
//! merge that makes a model from layers.

use alz_adapter::{Adapter, Content, Context, Mode, Model, TargetId};
use proptest::prelude::{prop_assert_eq, proptest};

use super::{ClaudeAdapter, is_model_value, neutral_permission_mode, neutral_tool};

fn claude() -> TargetId {
    TargetId::try_from("claude").expect("a target id")
}

#[test]
fn the_descriptor_names_the_claude_target_and_the_files_it_owns() {
    let adapter = ClaudeAdapter::new();
    let descriptor = adapter.descriptor();
    assert_eq!(descriptor.target, claude());
    assert_eq!(
        descriptor.adopted_files,
        [".claude/settings.json", ".mcp.json"]
    );
    assert!(
        descriptor
            .managed_folders
            .contains(&".claude/agents".to_owned())
    );
}

#[test]
fn an_empty_model_still_writes_claude_md_with_the_import_of_agents_md() {
    let output = ClaudeAdapter::new().render(&Model::new(claude()), &Context::default());
    let file = output
        .files
        .iter()
        .find(|file| file.path == "CLAUDE.md")
        .expect("CLAUDE.md is always written");
    let Content::Markdown(markdown) = &file.content else {
        panic!("CLAUDE.md is Markdown");
    };
    assert_eq!(markdown.body, "@AGENTS.md");
}

#[test]
fn a_neutral_tool_has_one_native_name_and_the_reverse_finds_it() {
    assert_eq!(neutral_tool("WebFetch"), Some("web_fetch"));
    assert_eq!(neutral_tool("mcp__server__tool"), None);
    assert_eq!(neutral_tool("read"), None);
}

#[test]
fn a_native_permission_mode_is_the_neutral_one_and_manual_is_default() {
    assert_eq!(
        neutral_permission_mode("acceptEdits"),
        Some(Mode::AcceptEdits)
    );
    assert_eq!(neutral_permission_mode("manual"), Some(Mode::Default));
    assert_eq!(neutral_permission_mode("yolo"), None);
}

#[test]
fn a_model_value_is_a_name_or_a_claude_id() {
    assert!(is_model_value("opus") && is_model_value("claude-opus-4.1"));
    assert!(!is_model_value("claude-") && !is_model_value("gpt-5") && !is_model_value("Claude-x"));
}

proptest! {
    #[test]
    fn rendering_is_deterministic(name in "[a-z]{1,8}") {
        let target = TargetId::try_from(name.as_str()).expect("a target id");
        let model = Model::new(target);
        let adapter = ClaudeAdapter::new();
        let first = adapter.render(&model, &Context::default());
        let second = adapter.render(&model, &Context::default());
        prop_assert_eq!(first, second);
    }
}
