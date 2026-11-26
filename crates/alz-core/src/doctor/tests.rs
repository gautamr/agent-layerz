use std::collections::BTreeSet;

use super::{
    AccountFolder, Ancestor, ClaudeFacts, ClientRoot, Facts, Installed, Report, WorkspaceFolder,
    WorkspaceState,
};
use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::doctor::Version;
use crate::live::Built;
use crate::untrusted_text::Stripped;

fn names(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|name| (*name).to_owned()).collect()
}

fn stripped(text: &str) -> Stripped {
    Stripped::from_untrusted(text)
}

fn empty_account(path: &str) -> AccountFolder {
    AccountFolder {
        path: stripped(path),
        agents: BTreeSet::new(),
        skills: BTreeSet::new(),
        has_claude_md: false,
    }
}

fn facts() -> Facts {
    Facts {
        root: stripped("/work/repo"),
        claude: Some(ClaudeFacts {
            accounts: vec![],
            installed: Installed::NotFound,
            built: Built::from_names(["architect".to_owned()], ["stack-conventions".to_owned()]),
        }),
        client_root: None,
        workspace: vec![],
        ancestors: vec![],
        blocked: vec![],
        build: Diagnostics::new(),
    }
}

fn with_accounts(accounts: Vec<AccountFolder>) -> Facts {
    let mut facts = facts();
    if let Some(claude) = facts.claude.as_mut() {
        claude.accounts = accounts;
    }
    facts
}

#[test]
fn a_clean_machine_is_ok() {
    let report = Report::of(&facts());
    assert_eq!(report.problems(), 0);
    assert_eq!(
        report.to_string(),
        "agent-layerz doctor: /work/repo\nclaude account dirs:\n  none found\nclaude: not found; version check skipped\nbuild: 0 errors, 0 warnings\ndoctor: ok\n"
    );
}

#[test]
fn an_account_with_a_built_name_has_a_collision_per_kind() {
    let account = AccountFolder {
        skills: names(&["stack-conventions", "mine"]),
        agents: names(&["architect"]),
        has_claude_md: true,
        ..empty_account("/home/me/.claude-work")
    };
    let report = Report::of(&with_accounts(vec![account]));
    let text = report.to_string();
    assert_eq!(report.problems(), 2);
    assert!(text.contains("  /home/me/.claude-work: 2 collisions\n"));
    assert!(text.contains("    collision: skill 'stack-conventions' also exists in /home/me/.claude-work/skills (Claude: a user skill beats the project one, silently)\n"));
    assert!(text.contains("    collision: agent 'architect' also exists in /home/me/.claude-work/agents (move it into a layer)\n"));
    assert!(text.contains(
        "    note: /home/me/.claude-work/CLAUDE.md is loaded into every session as well\n"
    ));
    assert!(!text.contains("'mine'"));
    assert!(text.ends_with("doctor: 2 problems\n"));
}

#[test]
fn a_single_collision_is_singular() {
    let account = AccountFolder {
        agents: names(&["architect"]),
        ..empty_account("/a")
    };
    let text = Report::of(&with_accounts(vec![account])).to_string();
    assert!(text.contains("  /a: 1 collision\n"));
    assert!(text.ends_with("doctor: 1 problem\n"));
}

#[test]
fn the_report_does_not_depend_on_the_names_of_two_empty_accounts() {
    let one = Report::of(&with_accounts(vec![empty_account("/x/one")])).to_string();
    let two = Report::of(&with_accounts(vec![empty_account("/x/two")])).to_string();
    assert_eq!(one.replace("/x/one", "<a>"), two.replace("/x/two", "<a>"));
    assert!(one.contains("  /x/one: no collisions\n"));
}

#[test]
fn the_version_line_says_found_in_range_or_outside() {
    let line = |installed| {
        let mut facts = facts();
        if let Some(claude) = facts.claude.as_mut() {
            claude.installed = installed;
        }
        Report::of(&facts).to_string()
    };
    assert!(
        line(Installed::Version(Version::new(2, 1, 400)))
            .contains("claude: 2.1.400 (verified range >=2.1.311 <2.2.0)\n")
    );
    assert!(line(Installed::Version(Version::new(2, 2, 0))).contains(
        "warning: claude 2.2.0 is outside the verified range >=2.1.311 <2.2.0; re-check docs/target-facts.md\n"
    ));
}

#[test]
fn a_repository_without_the_claude_target_has_no_claude_section() {
    let mut facts = facts();
    facts.claude = None;
    let text = Report::of(&facts).to_string();
    assert!(!text.contains("claude"));
}

#[test]
fn the_client_root_warns_for_each_file_and_for_a_git_repo() {
    let mut facts = facts();
    facts.client_root = Some(ClientRoot {
        path: stripped("/work"),
        has_claude_md: true,
        has_agents_md: true,
        is_git_repository: true,
    });
    let text = Report::of(&facts).to_string();
    assert!(text.contains("warning: client root /work/CLAUDE.md exists; remove it (put client content in the hub layer)\n"));
    assert!(text.contains("warning: client root /work/AGENTS.md exists;"));
    assert!(text.contains("warning: client root /work is a git repo; it should not be (its .claude/ and AGENTS.md would load in every repo)\n"));
    assert!(text.ends_with("doctor: ok\n"));
}

#[test]
fn workspace_folders_warn_when_missing_and_note_what_they_would_load() {
    let mut facts = facts();
    facts.workspace = vec![
        WorkspaceFolder {
            declared: stripped("../gone"),
            resolved: stripped("/work/gone"),
            state: WorkspaceState::Missing,
        },
        WorkspaceFolder {
            declared: stripped("../lib"),
            resolved: stripped("/work/lib"),
            state: WorkspaceState::Present {
                loaded: vec!["skills", "agents"],
            },
        },
        WorkspaceFolder {
            declared: stripped("../plain"),
            resolved: stripped("/work/plain"),
            state: WorkspaceState::Present { loaded: vec![] },
        },
    ];
    let text = Report::of(&facts).to_string();
    assert!(text.contains("warning: workspace dir ../gone does not exist (/work/gone)\n"));
    assert!(text.contains("note: ../lib/.claude has skills, agents; they load only if you pass --add-dir ../lib yourself\n"));
    assert!(!text.contains("plain"));
}

#[test]
fn ancestors_warn_for_claude_md_and_for_a_git_repo_with_claude() {
    let mut facts = facts();
    facts.ancestors = vec![
        Ancestor {
            path: stripped("/work"),
            has_claude_md: true,
            is_git_repository_with_claude: true,
        },
        Ancestor {
            path: stripped("/"),
            has_claude_md: false,
            is_git_repository_with_claude: false,
        },
    ];
    let text = Report::of(&facts).to_string();
    assert!(text.contains("warning: /work/CLAUDE.md is loaded too (ancestor directory)\n"));
    assert!(text.contains(
        "warning: /work is a git repo with .claude/; its skills or agents may load in this repo\n"
    ));
}

#[test]
fn build_errors_are_problems_and_warnings_are_listed() {
    let mut facts = facts();
    facts.build = Diagnostics::new()
        .with(Diagnostic::error(Code::Skill, None, Message::new("broken")))
        .with(Diagnostic::warning(Code::Skill, None, Message::new("odd")));
    let report = Report::of(&facts);
    assert_eq!(report.problems(), 1);
    let text = report.to_string();
    assert!(
        text.contains("build: 1 error, 1 warning\n  error [skill] broken\n  warning [skill] odd\n")
    );
    assert!(text.ends_with("doctor: 1 problem\n"));
}

#[test]
fn a_blocked_folder_is_warned_about_once_and_is_not_a_problem() {
    let mut facts = facts();
    facts.blocked = vec![stripped("/home/me/.claude-alz-blocked")];
    let report = Report::of(&facts);
    assert_eq!(report.problems(), 0);
    let text = report.to_string();
    assert_eq!(text.matches("blocked folder").count(), 1, "{text}");
    assert!(text.contains(
        "claude account dirs:\n  none found\nclaude: not found; version check skipped\nwarning: /home/me/.claude-alz-blocked is a blocked folder (§7.13), not an account; delete it\nbuild: 0 errors, 0 warnings\n"
    ));
    assert!(text.ends_with("doctor: ok\n"));
}

#[test]
fn both_blocked_folders_get_a_warning_whatever_the_targets() {
    let mut facts = facts();
    facts.claude = None;
    facts.blocked = vec![
        stripped("/home/me/.claude-alz-blocked"),
        stripped("/home/me/.codex-alz-blocked"),
    ];
    let text = Report::of(&facts).to_string();
    assert_eq!(
        text,
        "agent-layerz doctor: /work/repo\nwarning: /home/me/.claude-alz-blocked is a blocked folder (§7.13), not an account; delete it\nwarning: /home/me/.codex-alz-blocked is a blocked folder (§7.13), not an account; delete it\nbuild: 0 errors, 0 warnings\ndoctor: ok\n"
    );
}
