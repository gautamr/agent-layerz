//! The layers of the fixtures whose Claude files the goldens hold, read at compile time. Generated
//! once from the fixture folders; the layer files are those of the fixtures.

use super::{Golden, Layer};

pub(crate) const CASE_01: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/01-determinism/personal/layer.yaml"),
        files: &[
            (
                "agents/reviewer.md",
                include_str!("../../../../../fixtures/01-determinism/personal/agents/reviewer.md"),
            ),
            (
                "commands/learn.md",
                include_str!("../../../../../fixtures/01-determinism/personal/commands/learn.md"),
            ),
            (
                "hooks/hooks.yaml",
                include_str!("../../../../../fixtures/01-determinism/personal/hooks/hooks.yaml"),
            ),
            (
                "hooks/scripts/block-secrets.sh",
                include_str!(
                    "../../../../../fixtures/01-determinism/personal/hooks/scripts/block-secrets.sh"
                ),
            ),
            (
                "instructions/00-core.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/personal/instructions/00-core.md"
                ),
            ),
            (
                "mcp.yaml",
                include_str!("../../../../../fixtures/01-determinism/personal/mcp.yaml"),
            ),
            (
                "models.yaml",
                include_str!("../../../../../fixtures/01-determinism/personal/models.yaml"),
            ),
            (
                "permissions.yaml",
                include_str!("../../../../../fixtures/01-determinism/personal/permissions.yaml"),
            ),
            (
                "skills/review-checklist/SKILL.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/personal/skills/review-checklist/SKILL.md"
                ),
            ),
            (
                "skills/review-checklist/references/checklist.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/personal/skills/review-checklist/references/checklist.md"
                ),
            ),
            (
                "skills/stack-conventions/SKILL.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/personal/skills/stack-conventions/SKILL.md"
                ),
            ),
        ],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/01-determinism/client/layer.yaml"),
        files: &[
            (
                "agents/reviewer.overlay.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/client/agents/reviewer.overlay.md"
                ),
            ),
            (
                "instructions/10-domain.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/client/instructions/10-domain.md"
                ),
            ),
            (
                "settings.yaml",
                include_str!("../../../../../fixtures/01-determinism/client/settings.yaml"),
            ),
            (
                "skills/domain-glossary/SKILL.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/client/skills/domain-glossary/SKILL.md"
                ),
            ),
        ],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/01-determinism/repo/.layer/layer.yaml"),
        files: &[
            (
                "instructions/50-repo.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/repo/.layer/instructions/50-repo.md"
                ),
            ),
            (
                "permissions.yaml",
                include_str!("../../../../../fixtures/01-determinism/repo/.layer/permissions.yaml"),
            ),
            (
                "skills/stack-conventions/SKILL.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/repo/.layer/skills/stack-conventions/SKILL.md"
                ),
            ),
            (
                "targets/claude/CLAUDE.extra.md",
                include_str!(
                    "../../../../../fixtures/01-determinism/repo/.layer/targets/claude/CLAUDE.extra.md"
                ),
            ),
        ],
    },
];

pub(crate) const CASE_03: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/03-agent-overlay/personal/layer.yaml"),
        files: &[
            (
                "agents/architect.md",
                include_str!(
                    "../../../../../fixtures/03-agent-overlay/personal/agents/architect.md"
                ),
            ),
            (
                "models.yaml",
                include_str!("../../../../../fixtures/03-agent-overlay/personal/models.yaml"),
            ),
            (
                "skills/arch-standards/SKILL.md",
                include_str!(
                    "../../../../../fixtures/03-agent-overlay/personal/skills/arch-standards/SKILL.md"
                ),
            ),
        ],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/03-agent-overlay/client/layer.yaml"),
        files: &[
            (
                "agents/architect.overlay.md",
                include_str!(
                    "../../../../../fixtures/03-agent-overlay/client/agents/architect.overlay.md"
                ),
            ),
            (
                "skills/domain-glossary/SKILL.md",
                include_str!(
                    "../../../../../fixtures/03-agent-overlay/client/skills/domain-glossary/SKILL.md"
                ),
            ),
        ],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/03-agent-overlay/repo/.layer/layer.yaml"),
        files: &[(
            "agents/architect.overlay.md",
            include_str!(
                "../../../../../fixtures/03-agent-overlay/repo/.layer/agents/architect.overlay.md"
            ),
        )],
    },
];

pub(crate) const CASE_04: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/04-tool-narrowing/personal/layer.yaml"),
        files: &[
            (
                "agents/builder.md",
                include_str!(
                    "../../../../../fixtures/04-tool-narrowing/personal/agents/builder.md"
                ),
            ),
            (
                "agents/writer.md",
                include_str!("../../../../../fixtures/04-tool-narrowing/personal/agents/writer.md"),
            ),
        ],
    },
    Layer {
        yaml: include_str!(
            "../../../../../fixtures/04-tool-narrowing/repo-narrow/.layer/layer.yaml"
        ),
        files: &[(
            "agents/builder.overlay.md",
            include_str!(
                "../../../../../fixtures/04-tool-narrowing/repo-narrow/.layer/agents/builder.overlay.md"
            ),
        )],
    },
];

pub(crate) const CASE_05: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/05-skill-override/personal/layer.yaml"),
        files: &[(
            "skills/stack-conventions/SKILL.md",
            include_str!(
                "../../../../../fixtures/05-skill-override/personal/skills/stack-conventions/SKILL.md"
            ),
        )],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/05-skill-override/repo/.layer/layer.yaml"),
        files: &[(
            "skills/stack-conventions/SKILL.md",
            include_str!(
                "../../../../../fixtures/05-skill-override/repo/.layer/skills/stack-conventions/SKILL.md"
            ),
        )],
    },
];

pub(crate) const CASE_06: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/06-slots/personal/layer.yaml"),
        files: &[],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/06-slots/repo-loose/.layer/layer.yaml"),
        files: &[],
    },
];

pub(crate) const CASE_17: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/17-rules/personal/layer.yaml"),
        files: &[
            (
                "rules/backend.md",
                include_str!("../../../../../fixtures/17-rules/personal/rules/backend.md"),
            ),
            (
                "rules/testing.md",
                include_str!("../../../../../fixtures/17-rules/personal/rules/testing.md"),
            ),
        ],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/17-rules/repo/.layer/layer.yaml"),
        files: &[
            (
                "rules/backend.md",
                include_str!("../../../../../fixtures/17-rules/repo/.layer/rules/backend.md"),
            ),
            (
                "rules/testing.md",
                include_str!("../../../../../fixtures/17-rules/repo/.layer/rules/testing.md"),
            ),
        ],
    },
];

pub(crate) const CASE_18: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/18-native-fields/personal/layer.yaml"),
        files: &[(
            "agents/architect.md",
            include_str!("../../../../../fixtures/18-native-fields/personal/agents/architect.md"),
        )],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/18-native-fields/repo/.layer/layer.yaml"),
        files: &[(
            "agents/architect.overlay.md",
            include_str!(
                "../../../../../fixtures/18-native-fields/repo/.layer/agents/architect.overlay.md"
            ),
        )],
    },
];

pub(crate) const CASE_22: &[Layer] = &[
    Layer {
        yaml: include_str!("../../../../../fixtures/22-ls/personal/layer.yaml"),
        files: &[(
            "agents/architect.md",
            include_str!("../../../../../fixtures/22-ls/personal/agents/architect.md"),
        )],
    },
    Layer {
        yaml: include_str!("../../../../../fixtures/22-ls/repo/.layer/layer.yaml"),
        files: &[],
    },
];
pub(crate) const GOLDEN_01: &[Golden] = &[
    (
        ".claude/agents/reviewer.md",
        include_str!(
            "../../../tests/fixtures/claude-files/01-determinism/.claude/agents/reviewer.md"
        ),
    ),
    (
        ".claude/skills/domain-glossary/SKILL.md",
        include_str!(
            "../../../tests/fixtures/claude-files/01-determinism/.claude/skills/domain-glossary/SKILL.md"
        ),
    ),
    (
        ".claude/skills/learn/SKILL.md",
        include_str!(
            "../../../tests/fixtures/claude-files/01-determinism/.claude/skills/learn/SKILL.md"
        ),
    ),
    (
        ".claude/skills/review-checklist/SKILL.md",
        include_str!(
            "../../../tests/fixtures/claude-files/01-determinism/.claude/skills/review-checklist/SKILL.md"
        ),
    ),
    (
        ".claude/skills/review-checklist/references/checklist.md",
        include_str!(
            "../../../tests/fixtures/claude-files/01-determinism/.claude/skills/review-checklist/references/checklist.md"
        ),
    ),
    (
        ".claude/skills/stack-conventions/SKILL.md",
        include_str!(
            "../../../tests/fixtures/claude-files/01-determinism/.claude/skills/stack-conventions/SKILL.md"
        ),
    ),
];

pub(crate) const GOLDEN_03: &[Golden] = &[
    (
        ".claude/agents/architect.md",
        include_str!(
            "../../../tests/fixtures/claude-files/03-agent-overlay/.claude/agents/architect.md"
        ),
    ),
    (
        ".claude/skills/arch-standards/SKILL.md",
        include_str!(
            "../../../tests/fixtures/claude-files/03-agent-overlay/.claude/skills/arch-standards/SKILL.md"
        ),
    ),
    (
        ".claude/skills/domain-glossary/SKILL.md",
        include_str!(
            "../../../tests/fixtures/claude-files/03-agent-overlay/.claude/skills/domain-glossary/SKILL.md"
        ),
    ),
];

pub(crate) const GOLDEN_04: &[Golden] = &[
    (
        ".claude/agents/builder.md",
        include_str!(
            "../../../tests/fixtures/claude-files/04-tool-narrowing/.claude/agents/builder.md"
        ),
    ),
    (
        ".claude/agents/writer.md",
        include_str!(
            "../../../tests/fixtures/claude-files/04-tool-narrowing/.claude/agents/writer.md"
        ),
    ),
];

pub(crate) const GOLDEN_05: &[Golden] = &[(
    ".claude/skills/stack-conventions/SKILL.md",
    include_str!(
        "../../../tests/fixtures/claude-files/05-skill-override/.claude/skills/stack-conventions/SKILL.md"
    ),
)];

pub(crate) const GOLDEN_06: &[Golden] = &[(
    ".claude/skills/domain-glossary/SKILL.md",
    include_str!(
        "../../../tests/fixtures/claude-files/06-slots/.claude/skills/domain-glossary/SKILL.md"
    ),
)];

pub(crate) const GOLDEN_17: &[Golden] = &[
    (
        ".claude/rules/backend.md",
        include_str!("../../../tests/fixtures/claude-files/17-rules/.claude/rules/backend.md"),
    ),
    (
        ".claude/rules/testing.md",
        include_str!("../../../tests/fixtures/claude-files/17-rules/.claude/rules/testing.md"),
    ),
];

pub(crate) const GOLDEN_18: &[Golden] = &[(
    ".claude/agents/architect.md",
    include_str!(
        "../../../tests/fixtures/claude-files/18-native-fields/.claude/agents/architect.md"
    ),
)];

pub(crate) const GOLDEN_22: &[Golden] = &[(
    ".claude/agents/architect.md",
    include_str!("../../../tests/fixtures/claude-files/22-ls/.claude/agents/architect.md"),
)];
