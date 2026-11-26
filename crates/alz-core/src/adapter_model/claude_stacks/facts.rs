//! Spec §8 "What an adapter may rely on": every output key the Claude adapter writes (a field of
//! frontmatter, a key of `settings.json` or `.mcp.json`, a file location) rests on a fact that
//! `docs/target-facts.md` records and does not mark UNVERIFIED.
//!
//! The test renders one stack that makes the adapter write every kind of key that §8.1 lists,
//! and gives its structured output to the facts harness of `alz-test-support`, with the facts file
//! that the adapter's descriptor names and [`ClaudeSubjects`], which says which rows of the target
//! facts that the spec names (the section in brackets of §8.1) each kind of key rests on. A key
//! that no such row records fails the test, unless it is one of [`NO_FACT_RECORDS`], which lists
//! every key that rests on no recorded fact today and so ratchets: adding an output key without a
//! fact fails, and recording a fact for a listed key, or no longer writing it, fails until the
//! list drops it.

use std::collections::BTreeSet;

use alz_adapter::{Adapter, Output};
use alz_target_claude::ClaudeAdapter;
use alz_test_support::facts::{
    Facts, KeyPath, Part, Subject, SubjectMap, assert_rests_on_facts, is_unverified, written_files,
};
use alz_test_support::workspace_root;

use crate::adapter_model::testing::{layer_executable, merged_with, output_of_whole};
use crate::lint::parse_document;
use crate::stack::Manifest;

/// The facts of the Claude adapter, as the repository records them, read from the file that its
/// descriptor names.
fn target_facts() -> Facts {
    Facts::of(ClaudeAdapter::new().descriptor(), &workspace_root())
        .expect("the descriptor names a facts file of the repository")
}

/// The kinds of file location the adapter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    Agents,
    Skills,
    Rules,
    Settings,
    McpJson,
    HookScripts,
}

/// What a written key is the key of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// A field of the frontmatter of `.claude/agents/<name>.md`.
    AgentField,
    /// A field of the frontmatter of `.claude/skills/<name>/SKILL.md`.
    SkillField,
    /// A field of the frontmatter of `.claude/rules/<name>.md`.
    RuleField,
    /// A top-level key of `.claude/settings.json`.
    SettingsKey,
    /// A key of `permissions` in `.claude/settings.json`.
    PermissionsKey,
    /// A hook event, a key of `hooks` in `.claude/settings.json`.
    HookEvent,
    /// A key of a hook group (`matcher`, `hooks`).
    HookGroupKey,
    /// A key of a hook handler.
    HookHandlerKey,
    /// A key of `.mcp.json`, or of one of its servers.
    McpKey,
    /// Where a file is written.
    Location(Place),
}

impl Kind {
    /// The facts of §8.1 for this kind of output, named by the spec's brackets: agents [§1],
    /// skills [§5], rules [§7c], `settings.json` [§4, §7b, §7c], `.mcp.json` [§7], and the
    /// native agent fields of §7.2 that `docs/target-facts.md` records in §14.4.
    const fn subject(self) -> Subject {
        match self {
            Self::AgentField => Subject {
                sections: &["## 1. ", "### 14.4 "],
                row_holds: "",
            },
            Self::SkillField => Subject {
                sections: &["## 5. "],
                row_holds: "skill",
            },
            Self::RuleField => Subject {
                sections: &["## 7c. "],
                row_holds: "Rules frontmatter",
            },
            Self::SettingsKey => Subject {
                sections: &["## 4. ", "## 7b. ", "## 7c. ", "### 13.4 ", "### 14.2 "],
                row_holds: "",
            },
            Self::PermissionsKey => Subject {
                sections: &["## 7b. ", "## 7c. ", "### 13.4 "],
                row_holds: "permissions",
            },
            Self::HookEvent => Subject {
                sections: &["## 4. "],
                row_holds: "Event names",
            },
            Self::HookGroupKey | Self::HookHandlerKey => Subject {
                sections: &["## 4. "],
                row_holds: "Config shape",
            },
            Self::McpKey => Subject {
                sections: &["## 7. ", "### 13.4 "],
                row_holds: ".mcp.json",
            },
            Self::Location(place) => place.subject(),
        }
    }
}

impl Place {
    /// The facts for a location: the sections that name it, and the text a row has to hold.
    const fn subject(self) -> Subject {
        match self {
            Self::Agents => Subject {
                sections: &["## 2. "],
                row_holds: ".claude/agents/",
            },
            Self::Skills => Subject {
                sections: &["## 5. "],
                row_holds: ".claude/skills/",
            },
            Self::Rules => Subject {
                sections: &["## 7c. "],
                row_holds: ".claude/rules/",
            },
            Self::Settings => Subject {
                sections: &["## 4. ", "## 7c. "],
                row_holds: ".claude/settings.json",
            },
            Self::McpJson => Subject {
                sections: &["## 7. "],
                row_holds: ".mcp.json",
            },
            // Claude reads no folder of hook scripts: the adapter points a hook at its own copy
            // through the project-dir variable, which §4 records.
            Self::HookScripts => Subject {
                sections: &["## 4. "],
                row_holds: "CLAUDE_PROJECT_DIR",
            },
        }
    }
}

/// The subject map of the Claude adapter: what each file and key of its output rests on.
struct ClaudeSubjects;

impl SubjectMap for ClaudeSubjects {
    type Kind = Kind;

    fn location(&self, file: &str) -> Option<(Kind, Subject)> {
        let place = match file {
            ".claude/settings.json" => Place::Settings,
            ".mcp.json" => Place::McpJson,
            _ if file.starts_with(".claude/agents/") => Place::Agents,
            _ if file.starts_with(".claude/rules/") => Place::Rules,
            _ if file.starts_with(".claude/skills/") => Place::Skills,
            _ if file.starts_with(".claude/hooks/") => Place::HookScripts,
            _ => return None,
        };
        let kind = Kind::Location(place);
        Some((kind, kind.subject()))
    }

    /// The keys that alz chooses: the fields of frontmatter, the top-level keys of
    /// `.claude/settings.json` and those of `permissions`, the events of `hooks` and the keys of
    /// their groups and handlers, and the keys of `.mcp.json` and of its servers. The keys of a
    /// user's native settings are checked against section 14 by `item::native_types`, and the
    /// server ids and variable names are the user's.
    fn key(&self, file: &str, path: &KeyPath) -> Option<(Kind, Subject)> {
        let kind = match (file, path.parts().as_slice()) {
            (".claude/settings.json", [Part::Key(_)]) => Kind::SettingsKey,
            (".claude/settings.json", [Part::Key("permissions"), Part::Key(_)]) => {
                Kind::PermissionsKey
            }
            (".claude/settings.json", [Part::Key("hooks"), Part::Key(_)]) => Kind::HookEvent,
            (
                ".claude/settings.json",
                [Part::Key("hooks"), Part::Key(_), Part::Items, Part::Key(_)],
            ) => Kind::HookGroupKey,
            (
                ".claude/settings.json",
                [
                    Part::Key("hooks"),
                    Part::Key(_),
                    Part::Items,
                    Part::Key("hooks"),
                    Part::Items,
                    Part::Key(_),
                ],
            ) => Kind::HookHandlerKey,
            (
                ".mcp.json",
                [Part::Key(_)] | [Part::Key("mcpServers"), Part::Key(_), Part::Key(_)],
            ) => Kind::McpKey,
            (_, [Part::Key(_)]) if file.starts_with(".claude/agents/") => Kind::AgentField,
            (_, [Part::Key(_)]) if file.starts_with(".claude/rules/") => Kind::RuleField,
            (_, [Part::Key(_)])
                if file.starts_with(".claude/skills/") && file.ends_with("/SKILL.md") =>
            {
                Kind::SkillField
            }
            _ => return None,
        };
        Some((kind, kind.subject()))
    }
}

// The stack that makes the adapter write every kind of key.

const REPO: &str = "name: shop-api\nkind: repo\nvisibility: private\n";
const PERSONAL: &str = "name: personal\nkind: personal\nvisibility: private\n";

const AGENT: &str = "---\nname: reviewer\ndescription: Reviews a change\nrole: review\ntools: [read, grep]\nskills: [checklist]\npermission_mode: plan\nmemory: project\nisolation: worktree\nclaude:\n  maxTurns: 9\n  effort: high\n---\nReview.\n";
const SKILL: &str = "---\nname: checklist\ndescription: Checklist for a review\n---\nWalk it.\n";
const COMMAND: &str =
    "---\ndescription: Record a lesson\nargument_hint: <lesson>\n---\nWrite it.\n";
const RULE: &str = "---\npaths: [\"src/**\"]\n---\nKeep handlers thin.\n";
const HOOKS: &str = "- id: guard\n  event: pre_tool\n  match: { tools: [read], paths: [\".env*\"] }\n  run: scripts/guard.sh\n  timeout: 5\n  on_fail: deny\n- id: fmt\n  event: post_tool\n  match: { tools: [edit] }\n  command: '\"${PROJECT_DIR}\"/bin/fmt'\n- id: start\n  event: session_start\n  command: echo hi\n- id: stop\n  event: stop\n  command: ./done.sh\n- id: sub\n  event: subagent_stop\n  command: ./sub.sh\n- id: prompt\n  event: prompt_submit\n  command: echo prompt\n";
const MCP: &str = "local:\n  transport: stdio\n  command: npx\n  args: [\"-y\", \"server\"]\n  env_from: [TOKEN]\nremote:\n  transport: http\n  url: https://remote.invalid/mcp\nstream:\n  transport: sse\n  url: https://stream.invalid/sse\n";
const MODELS: &str =
    "aliases:\n  smart:\n    claude: claude-opus-4-1\ndefault: smart\nroles:\n  review: smart\n";
const PERMISSIONS: &str = "allow: [\"read\"]\nask: [\"bash:git push *\"]\ndeny: [\"read:.env*\"]\ndefault_mode: plan\ndisable_bypass_mode: true\nblock_reads_outside: true\nblock_writes_outside: true\n";
const OPTIONS: &str =
    "mcp:\n  enable: [local, remote, stream]\nworkspace:\n  hub: ../hub\n  add_dirs: [../shared]\n";

/// The structured output of the adapter for the stack: the files that §8.1 lists, which are the
/// agents, skills, commands, rules, settings, hook scripts and `.mcp.json`. `CLAUDE.md` is
/// Markdown without frontmatter that the core writes from the instruction files, so it holds no
/// key.
fn rendered() -> Output {
    let layers = vec![
        layer_executable(
            PERSONAL,
            &[
                ("agents/reviewer.md", AGENT),
                ("skills/checklist/SKILL.md", SKILL),
                ("skills/checklist/references/notes.md", "Notes.\n"),
                ("commands/learn.md", COMMAND),
                ("rules/backend.md", RULE),
                ("hooks/hooks.yaml", HOOKS),
                ("hooks/scripts/guard.sh", "#!/bin/sh\nexit 0\n"),
                ("mcp.yaml", MCP),
                ("models.yaml", MODELS),
                ("permissions.yaml", PERMISSIONS),
            ],
            None,
            &["hooks/scripts/guard.sh"],
        ),
        layer_executable(REPO, &[], None, &[]),
    ];
    let found = merged_with(layers, OPTIONS);
    assert!(!found.diagnostics().has_errors(), "{}", found.diagnostics());
    let merged = found.merged().expect("no error was found");
    let manifest = format!("stack:\n  - source: path:./.layer\n{OPTIONS}");
    let node = parse_document(manifest.as_bytes()).expect("valid YAML");
    let workspace = Manifest::from_yaml(&node)
        .expect("a manifest")
        .workspace()
        .clone();
    output_of_whole(merged, &workspace)
}

/// What the adapter writes that no fact of `docs/target-facts.md` records: a key of a file kind
/// whose rows name it nowhere, or no verified row at all for a location. Each is a difference from
/// §8 "What an adapter may rely on". The facts have to be verified and recorded, by a docs task,
/// and the key leaves this list when they are. Empty since TASK-97 recorded the last eleven.
const NO_FACT_RECORDS: &[(Kind, &str)] = &[];

#[test]
fn every_output_key_the_claude_adapter_writes_rests_on_a_recorded_fact() {
    assert_rests_on_facts(
        &target_facts(),
        &ClaudeSubjects,
        &rendered(),
        NO_FACT_RECORDS,
    );
}

/// The kind and name of every location and key that `output` holds, as the subject map sees them.
fn written_by(output: &Output) -> BTreeSet<(Kind, String)> {
    written_files(output)
        .iter()
        .flat_map(|file| {
            let location = ClaudeSubjects
                .location(&file.file)
                .map(|(kind, _)| (kind, String::new()));
            let keys = file.keys.iter().filter_map(|path| {
                let (kind, _) = ClaudeSubjects.key(&file.file, path)?;
                Some((kind, path.name()?.to_owned()))
            });
            location.into_iter().chain(keys).collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn the_stack_makes_the_adapter_write_every_kind_of_key_section_8_1_lists() {
    let output = rendered();
    let all = written_by(&output);
    let keys = |kind: Kind| -> BTreeSet<&str> {
        all.iter()
            .filter(|(one, _)| *one == kind)
            .map(|(_, key)| key.as_str())
            .collect()
    };
    let wanted = |kind: Kind, names: &[&str]| {
        let missing: Vec<&&str> = names
            .iter()
            .filter(|name| !keys(kind).contains(**name))
            .collect();
        assert!(missing.is_empty(), "{kind:?} lacks {missing:?}");
    };
    wanted(
        Kind::AgentField,
        &[
            "name",
            "description",
            "tools",
            "model",
            "permissionMode",
            "skills",
            "isolation",
            "memory",
            "maxTurns",
            "effort",
        ],
    );
    wanted(Kind::SkillField, &["name", "description"]);
    wanted(
        Kind::SkillField,
        &["argument-hint", "disable-model-invocation"],
    );
    wanted(Kind::RuleField, &["paths"]);
    wanted(Kind::SettingsKey, &["permissions", "hooks", "model"]);
    wanted(
        Kind::PermissionsKey,
        &[
            "allow",
            "ask",
            "deny",
            "defaultMode",
            "additionalDirectories",
            "disableBypassPermissionsMode",
            "blockReadsOutsideWorkingDirectories",
            "blockWritesOutsideWorkingDirectories",
        ],
    );
    wanted(
        Kind::HookEvent,
        &[
            "SessionStart",
            "PreToolUse",
            "PostToolUse",
            "Stop",
            "SubagentStop",
            "UserPromptSubmit",
        ],
    );
    wanted(Kind::HookGroupKey, &["hooks", "matcher"]);
    wanted(
        Kind::HookHandlerKey,
        &["type", "command", "if", "timeout", "onFailure"],
    );
    wanted(
        Kind::McpKey,
        &["mcpServers", "command", "args", "env", "type", "url"],
    );
    let places: BTreeSet<Kind> = all
        .iter()
        .filter(|(kind, _)| matches!(kind, Kind::Location(_)))
        .map(|(kind, _)| *kind)
        .collect();
    assert_eq!(
        places,
        [
            Place::Agents,
            Place::Skills,
            Place::Rules,
            Place::Settings,
            Place::McpJson,
            Place::HookScripts,
        ]
        .map(Kind::Location)
        .into_iter()
        .collect()
    );
    let unplaced: Vec<String> = written_files(&output)
        .into_iter()
        .map(|file| file.file)
        .filter(|file| ClaudeSubjects.location(file).is_none())
        .collect();
    assert!(
        unplaced.is_empty(),
        "the adapter wrote a file in a place that §8.1 does not list: {unplaced:?}"
    );
}

#[test]
fn the_rows_the_spec_lists_as_unverified_are_the_only_unverified_rows_the_adapter_relies_on() {
    // Spec §8: relative paths in `permissions.additionalDirectories` (§7c) and how Claude treats
    // a settings key it does not know (§7). When one is verified, the spec's list shrinks.
    let facts = target_facts();
    let unverified = |heading: &str| -> Vec<String> {
        facts
            .section(heading)
            .lines()
            .filter(|row| row.starts_with('|') && is_unverified(row))
            .filter(|row| row.contains("Claude") || row.contains("Relative paths"))
            .map(|row| row.split('|').nth(1).unwrap_or_default().trim().to_owned())
            .collect()
    };
    assert_eq!(unverified("## 7c. "), ["Relative paths"]);
    assert_eq!(
        unverified("## 7. "),
        ["Claude, unknown *settings* key, malformed `settings.json`"]
    );
    for heading in ["## 1. ", "## 3. ", "## 4. ", "## 5. "] {
        assert_eq!(
            unverified(heading),
            Vec::<String>::new(),
            "{heading} has an UNVERIFIED Claude row the spec does not list"
        );
    }
}
