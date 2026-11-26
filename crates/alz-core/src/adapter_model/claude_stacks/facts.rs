//! Spec §8 "What an adapter may rely on": every output key the Claude adapter writes (a field of
//! frontmatter, a key of `settings.json` or `.mcp.json`, a file location) rests on a fact that
//! `docs/target-facts.md` records and does not mark UNVERIFIED.
//!
//! The test renders one stack that makes the adapter write every kind of key that §8.1 lists,
//! reads the keys back out of the files it wrote, and looks each one up in the rows of the
//! target facts that the spec names for that kind of output (the section in brackets of §8.1).
//! A key that no such row records fails the test, unless it is one of [`NO_FACT_RECORDS`], which
//! lists every key that rests on no recorded fact today and so ratchets: adding an output key
//! without a fact fails, and recording a fact for a listed key fails until the list drops it.

use std::collections::BTreeSet;

use crate::adapter_model::testing::{files_of, layer_executable, merged_with, settings_files_of};
use crate::json::{JsonValue, parse};
use crate::lint::parse_document;
use crate::markdown::MarkdownDoc;
use crate::render::tree::FileTree;
use crate::stack::Manifest;
use crate::yaml::Value;

/// The facts of the Claude adapter, as the repository records them.
const TARGET_FACTS: &str = include_str!("../../../../../docs/target-facts.md");

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

/// One key the adapter wrote, and the file that holds it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Written {
    kind: Kind,
    key: String,
    file: String,
}

/// Where the facts for a kind of key are: the headings of the sections of the target facts that
/// the spec names, and the words a row must hold to be about this kind of output.
struct Subject {
    sections: &'static [&'static str],
    row_holds: &'static str,
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

/// The text of the section whose heading starts with `heading`, up to the next heading.
fn section<'a>(facts: &'a str, heading: &str) -> &'a str {
    let start = facts
        .lines()
        .scan(0_usize, |offset, line| {
            let here = *offset;
            *offset += line.len() + 1;
            Some((here, line))
        })
        .find_map(|(here, line)| line.starts_with(heading).then_some(here));
    start.map_or("", |start| {
        let text = facts.get(start..).unwrap_or_default();
        let after_heading = text.find('\n').map_or(text.len(), |end| end + 1);
        let rest = text.get(after_heading..).unwrap_or_default();
        let end = rest
            .lines()
            .scan(0_usize, |offset, line| {
                let here = *offset;
                *offset += line.len() + 1;
                Some((here, line))
            })
            .find_map(|(here, line)| line.starts_with('#').then_some(here))
            .unwrap_or(rest.len());
        rest.get(..end).unwrap_or_default()
    })
}

/// Whether a row of the facts is marked UNVERIFIED in its status cell, the last one.
fn is_unverified(row: &str) -> bool {
    row.trim_end()
        .trim_end_matches('|')
        .trim_end()
        .ends_with("UNVERIFIED")
}

/// The words (identifiers with `_` and `-`) of the code spans of `row`.
fn span_words(row: &str) -> BTreeSet<String> {
    row.split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|span| {
            span.split(|character: char| {
                !(character.is_alphanumeric() || character == '_' || character == '-')
            })
        })
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The rows (table rows and bullets) of the sections of `subject` that are about it and are not
/// UNVERIFIED.
fn rows_of<'a>(facts: &'a str, subject: &Subject) -> Vec<&'a str> {
    subject
        .sections
        .iter()
        .flat_map(|heading| section(facts, heading).lines())
        .filter(|row| row.starts_with('|') || row.starts_with("- "))
        .filter(|row| row.contains(subject.row_holds))
        .filter(|row| !is_unverified(row))
        .collect()
}

/// Whether `facts` records `written`: a verified row of the subject's sections names the key in a
/// code span, or, for a location, holds its path.
fn is_recorded(facts: &str, written: &Written) -> bool {
    let subject = written.kind.subject();
    let rows = rows_of(facts, &subject);
    match written.kind {
        Kind::Location(_) => !rows.is_empty(),
        Kind::AgentField
        | Kind::SkillField
        | Kind::RuleField
        | Kind::SettingsKey
        | Kind::PermissionsKey
        | Kind::HookEvent
        | Kind::HookGroupKey
        | Kind::HookHandlerKey
        | Kind::McpKey => rows
            .iter()
            .any(|row| span_words(row).contains(&written.key)),
    }
}

/// The keys of `written` that `facts` does not record.
fn unrecorded(facts: &str, written: &[Written]) -> BTreeSet<(Kind, String)> {
    written
        .iter()
        .filter(|one| !is_recorded(facts, one))
        .map(|one| (one.kind, one.key.clone()))
        .collect()
}

// What the files hold.

fn keys_of(value: &JsonValue) -> Vec<&str> {
    match value {
        JsonValue::Object(object) => object.iter().map(|(key, _)| key).collect(),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)
        | JsonValue::Array(_) => Vec::new(),
    }
}

fn members(value: &JsonValue) -> Vec<(&str, &JsonValue)> {
    match value {
        JsonValue::Object(object) => object.iter().collect(),
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)
        | JsonValue::Array(_) => Vec::new(),
    }
}

fn items(value: &JsonValue) -> &[JsonValue] {
    match value {
        JsonValue::Array(items) => items,
        JsonValue::Null
        | JsonValue::Bool(_)
        | JsonValue::Number(_)
        | JsonValue::Text(_)
        | JsonValue::Object(_) => &[],
    }
}

fn written(kind: Kind, key: &str, file: &str) -> Written {
    Written {
        kind,
        key: key.to_owned(),
        file: file.to_owned(),
    }
}

fn frontmatter_keys(kind: Kind, file: &str, bytes: &[u8]) -> Vec<Written> {
    let document = MarkdownDoc::parse(bytes).expect("a generated file has valid frontmatter");
    document
        .front()
        .mapping()
        .entries()
        .filter_map(|(key, _)| match key.value() {
            Value::Text(text) => Some(written(kind, text, file)),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Seq(_)
            | Value::Map(_) => None,
        })
        .collect()
}

/// The keys of `.claude/settings.json` that alz chooses: the top-level keys, those of
/// `permissions`, the events of `hooks` and the keys of their groups and handlers. The keys of a
/// user's native settings are checked against section 14 by `item::native_types`, and the
/// server ids and variable names are the user's.
fn settings_keys(file: &str, bytes: &[u8]) -> Vec<Written> {
    let settings = parse(bytes).expect("generated settings are JSON");
    let top = members(&settings);
    let child = |name: &str| {
        top.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| *value)
    };
    let groups: Vec<(&str, &JsonValue)> = child("hooks").map_or_default(members);
    let handler_keys = groups
        .iter()
        .flat_map(|(_, list)| items(list))
        .flat_map(|group| members(group))
        .filter(|(key, _)| *key == "hooks")
        .flat_map(|(_, handlers)| items(handlers))
        .flat_map(keys_of);
    let group_keys = groups
        .iter()
        .flat_map(|(_, list)| items(list))
        .flat_map(keys_of);
    top.iter()
        .map(|(key, _)| written(Kind::SettingsKey, key, file))
        .chain(
            child("permissions")
                .map_or_default(keys_of)
                .into_iter()
                .map(|key| written(Kind::PermissionsKey, key, file)),
        )
        .chain(
            groups
                .iter()
                .map(|(event, _)| written(Kind::HookEvent, event, file)),
        )
        .chain(group_keys.map(|key| written(Kind::HookGroupKey, key, file)))
        .chain(handler_keys.map(|key| written(Kind::HookHandlerKey, key, file)))
        .collect()
}

fn mcp_keys(file: &str, bytes: &[u8]) -> Vec<Written> {
    let document = parse(bytes).expect("a generated .mcp.json is JSON");
    keys_of(&document)
        .into_iter()
        .map(|key| written(Kind::McpKey, key, file))
        .chain(
            members(&document)
                .into_iter()
                .filter(|(key, _)| *key == "mcpServers")
                .flat_map(|(_, servers)| members(servers))
                .flat_map(|(_, server)| keys_of(server))
                .map(|key| written(Kind::McpKey, key, file)),
        )
        .collect()
}

/// Every key and location of the files of `tree`; a file of a place no kind of §8.1 names is
/// reported as a location with the key `unclassified`, which no fact records.
fn written_by(tree: &FileTree) -> Vec<Written> {
    tree.iter()
        .flat_map(|(path, file)| {
            let name = path.as_str();
            let at = |place: Place| written(Kind::Location(place), "", name);
            if name == ".claude/settings.json" {
                std::iter::once(at(Place::Settings))
                    .chain(settings_keys(name, &file.bytes))
                    .collect::<Vec<_>>()
            } else if name == ".mcp.json" {
                std::iter::once(at(Place::McpJson))
                    .chain(mcp_keys(name, &file.bytes))
                    .collect()
            } else if name.starts_with(".claude/agents/") {
                std::iter::once(at(Place::Agents))
                    .chain(frontmatter_keys(Kind::AgentField, name, &file.bytes))
                    .collect()
            } else if name.starts_with(".claude/rules/") {
                std::iter::once(at(Place::Rules))
                    .chain(frontmatter_keys(Kind::RuleField, name, &file.bytes))
                    .collect()
            } else if name.starts_with(".claude/skills/") && name.ends_with("/SKILL.md") {
                std::iter::once(at(Place::Skills))
                    .chain(frontmatter_keys(Kind::SkillField, name, &file.bytes))
                    .collect()
            } else if name.starts_with(".claude/skills/") {
                vec![at(Place::Skills)]
            } else if name.starts_with(".claude/hooks/") {
                vec![at(Place::HookScripts)]
            } else {
                vec![written(Kind::SettingsKey, "unclassified", name)]
            }
        })
        .collect()
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

fn rendered() -> FileTree {
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
    let (files, files_found) = files_of(merged).into_parts();
    let (settings, settings_found) = settings_files_of(merged, &workspace).into_parts();
    assert!(!files_found.has_errors(), "{files_found}");
    assert!(!settings_found.has_errors(), "{settings_found}");
    files
        .iter()
        .chain(settings.iter())
        .fold(FileTree::new(), |mut tree, (path, file)| {
            tree.insert(path.clone(), file.clone())
                .expect("the two sets of files share no path");
            tree
        })
}

/// What the adapter writes that no fact of `docs/target-facts.md` records: a key of a file kind
/// whose rows name it nowhere, or no verified row at all for a location. Each is a difference from
/// §8 "What an adapter may rely on". The facts have to be verified and recorded, by a docs task,
/// and the key leaves this list when they are. Empty since TASK-97 recorded the last eleven.
const NO_FACT_RECORDS: &[(Kind, &str)] = &[];

#[test]
fn every_output_key_the_claude_adapter_writes_rests_on_a_recorded_fact() {
    let found = unrecorded(TARGET_FACTS, &written_by(&rendered()));
    let known: BTreeSet<(Kind, String)> = NO_FACT_RECORDS
        .iter()
        .map(|(kind, key)| (*kind, (*key).to_owned()))
        .collect();
    assert_eq!(
        found, known,
        "a key the adapter writes rests on no verified row of docs/target-facts.md (left), or a \
         key of NO_FACT_RECORDS is recorded now (right)"
    );
}

#[test]
fn the_stack_makes_the_adapter_write_every_kind_of_key_section_8_1_lists() {
    let all = written_by(&rendered());
    let keys = |kind: Kind| -> BTreeSet<&str> {
        all.iter()
            .filter(|one| one.kind == kind)
            .map(|one| one.key.as_str())
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
        .filter(|one| matches!(one.kind, Kind::Location(_)))
        .map(|one| one.kind)
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
    assert!(
        all.iter().all(|one| one.key != "unclassified"),
        "the adapter wrote a file in a place that §8.1 does not list"
    );
}

#[test]
fn a_key_that_no_verified_row_records_is_reported() {
    let facts = "## 1. Agents\n\n| Item | Verified |\n|---|---|\n| Fields | `name`, `tools` | DOCS |\n| Isolation | `isolation: worktree` | UNVERIFIED |\n\n## 2. Next\n\n| Other | `memory` | DOCS |\n";
    let one = |key: &str| written(Kind::AgentField, key, ".claude/agents/a.md");
    let all = [
        one("name"),
        one("tools"),
        one("isolation"),
        one("memory"),
        one("bogus"),
    ];
    let expected: BTreeSet<(Kind, String)> = ["isolation", "memory", "bogus"]
        .into_iter()
        .map(|key| (Kind::AgentField, key.to_owned()))
        .collect();
    assert_eq!(
        unrecorded(facts, &all),
        expected,
        "an UNVERIFIED row and a row of another section record nothing"
    );
}

#[test]
fn the_rows_the_spec_lists_as_unverified_are_the_only_unverified_rows_the_adapter_relies_on() {
    // Spec §8: relative paths in `permissions.additionalDirectories` (§7c) and how Claude treats
    // a settings key it does not know (§7). When one is verified, the spec's list shrinks.
    let unverified = |heading: &str| -> Vec<String> {
        section(TARGET_FACTS, heading)
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
