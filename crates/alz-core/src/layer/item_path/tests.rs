use proptest::prelude::{prop_assert_eq, prop_assume, proptest};

use super::ItemPath;
use crate::path::SafeRelPath;

fn path(text: &str) -> SafeRelPath {
    SafeRelPath::try_from(text).expect("a safe path")
}

#[test]
fn every_path_of_the_layout_in_section_4_1_is_one_variant() {
    let table = [
        ("layer.yaml", ItemPath::LayerYaml),
        (
            "instructions/00-core.md",
            ItemPath::Instruction { name: "00-core" },
        ),
        (
            "instructions/50-topic.md",
            ItemPath::Instruction { name: "50-topic" },
        ),
        ("agents/architect.md", ItemPath::Agent { name: "architect" }),
        (
            "agents/architect.overlay.md",
            ItemPath::AgentOverlay { name: "architect" },
        ),
        (
            "skills/review/SKILL.md",
            ItemPath::SkillFile {
                skill: "review",
                file: "SKILL.md",
            },
        ),
        (
            "skills/review/scripts/run.sh",
            ItemPath::SkillFile {
                skill: "review",
                file: "scripts/run.sh",
            },
        ),
        (
            "skills/review/references/x.md",
            ItemPath::SkillFile {
                skill: "review",
                file: "references/x.md",
            },
        ),
        ("commands/ship.md", ItemPath::Command { name: "ship" }),
        ("rules/rust.md", ItemPath::Rule { name: "rust" }),
        ("hooks/hooks.yaml", ItemPath::Hooks),
        (
            "hooks/scripts/guard.sh",
            ItemPath::HookScript { script: "guard.sh" },
        ),
        (
            "hooks/scripts/sub/guard.sh",
            ItemPath::HookScript {
                script: "sub/guard.sh",
            },
        ),
        ("permissions.yaml", ItemPath::Permissions),
        ("mcp.yaml", ItemPath::Mcp),
        ("models.yaml", ItemPath::Models),
        ("settings.yaml", ItemPath::Settings),
        (
            "targets/claude/.claude/output-styles/x.md",
            ItemPath::Passthrough {
                target: crate::target::claude().clone(),
                path: ".claude/output-styles/x.md",
            },
        ),
        (
            "targets/codex/config.toml",
            ItemPath::Passthrough {
                target: crate::target::codex().clone(),
                path: "config.toml",
            },
        ),
        ("targets/claude/CLAUDE.extra.md", ItemPath::ClaudeExtra),
        (
            "targets/claude/sub/CLAUDE.extra.md",
            ItemPath::Passthrough {
                target: crate::target::claude().clone(),
                path: "sub/CLAUDE.extra.md",
            },
        ),
        (
            "targets/codex/CLAUDE.extra.md",
            ItemPath::Passthrough {
                target: crate::target::codex().clone(),
                path: "CLAUDE.extra.md",
            },
        ),
    ];
    for (text, expected) in table {
        assert_eq!(ItemPath::of(&path(text)), expected, "{text}");
    }
}

#[test]
fn any_other_path_is_other() {
    for text in [
        "README.md",
        "notes/todo.md",
        "instructions/sub/a.md",
        "instructions/a.txt",
        "instructions/.md",
        "instructions",
        "agents/a.txt",
        "agents/.md",
        "agents/.overlay.md",
        "agents/sub/a.md",
        "skills/SKILL.md",
        "skills/review",
        "commands/sub/a.md",
        "commands/a.yaml",
        "rules/sub/a.md",
        "rules/a.mdx",
        "hooks/other.yaml",
        "hooks/scripts",
        "hooks/hooks.yaml.bak",
        "hooks/sub/hooks.yaml",
        "sub/layer.yaml",
        "sub/mcp.yaml",
        "Permissions.yaml",
        "targets/claude",
        "targets/Other/x.md",
    ] {
        assert_eq!(ItemPath::of(&path(text)), ItemPath::Other, "{text}");
    }
}

proptest! {
    #[test]
    fn a_path_outside_every_folder_and_file_of_the_layout_is_other(
        text in "[a-z]{1,8}(/[a-zA-Z][a-zA-Z.]{0,7}){0,3}"
    ) {
        let roots = ["instructions", "agents", "skills", "commands", "rules", "hooks", "targets"];
        let top = text.split('/').next().unwrap_or_default();
        prop_assume!(!roots.contains(&top));
        prop_assume!(!["layer", "permissions", "mcp", "models", "settings"].contains(&text.as_str()));
        let safe = path(&text);
        prop_assert_eq!(ItemPath::of(&safe), ItemPath::Other);
    }
}
