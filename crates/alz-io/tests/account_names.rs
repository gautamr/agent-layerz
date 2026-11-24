//! The account folders `alz doctor` reads (spec §9.1): names only. A file inside an account folder
//! is never opened, which the tests show by taking every permission from the files.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;

    use alz_core::env::EnvVars;
    use alz_io::{EnvSnapshot, blocked_folders, claude_account_folders};
    use alz_test_support::{ScratchDir, running_as_root};

    fn snapshot(home: &Path, configured: Option<&Path>) -> EnvSnapshot {
        EnvSnapshot::from_vars(
            [("HOME", Some(home)), ("CLAUDE_CONFIG_DIR", configured)]
                .into_iter()
                .filter_map(|(name, path)| {
                    path.map(|path| (OsString::from(name), path.as_os_str().to_owned()))
                })
                .collect::<EnvVars>(),
        )
    }

    #[test]
    fn the_folders_are_the_configured_one_then_the_default_then_each_named_one_in_order() {
        let scratch = ScratchDir::new().expect("scratch");
        for folder in [
            "home/.claude",
            "home/.claude-b",
            "home/.claude-a",
            "home/.codex-x",
            "config",
        ] {
            scratch.make_dirs(folder).expect("made");
        }
        scratch
            .write("home/.claude-file", "not a folder")
            .expect("written");
        let env = snapshot(&scratch.join("home"), Some(&scratch.join("config")));
        let prefix = scratch.path().to_string_lossy().into_owned();
        let found: Vec<String> = claude_account_folders(&env)
            .iter()
            .map(|folder| folder.path.as_str().replace(&prefix, "<s>"))
            .collect();
        assert_eq!(
            found,
            [
                "<s>/config",
                "<s>/home/.claude",
                "<s>/home/.claude-a",
                "<s>/home/.claude-b"
            ]
        );
    }

    #[test]
    fn the_blocked_folders_are_not_accounts_even_when_claude_config_dir_names_one() {
        let scratch = ScratchDir::new().expect("scratch");
        for folder in [
            "home/.claude",
            "home/.claude-work",
            "home/.claude-alz-blocked",
            "home/.codex-alz-blocked",
        ] {
            scratch.make_dirs(folder).expect("made");
        }
        let blocked = scratch.join("home/.claude-alz-blocked");
        let env = snapshot(&scratch.join("home"), Some(&blocked));
        let prefix = scratch.path().to_string_lossy().into_owned();
        let found: Vec<String> = claude_account_folders(&env)
            .iter()
            .map(|folder| folder.path.as_str().replace(&prefix, "<s>"))
            .collect();
        assert_eq!(found, ["<s>/home/.claude", "<s>/home/.claude-work"]);
    }

    #[test]
    fn each_blocked_folder_that_exists_is_found_by_name_claude_first() {
        let scratch = ScratchDir::new().expect("scratch");
        let home = scratch.join("home");
        let env = snapshot(&home, None);
        scratch.make_dirs("home").expect("made");
        assert_eq!(blocked_folders(&env).len(), 0);
        scratch.make_dirs("home/.codex-alz-blocked").expect("made");
        scratch.make_dirs("home/.claude-alz-blocked").expect("made");
        scratch.make_dirs("home/.claude-other").expect("made");
        let prefix = scratch.path().to_string_lossy().into_owned();
        let found: Vec<String> = blocked_folders(&env)
            .iter()
            .map(|folder| folder.as_str().replace(&prefix, "<s>"))
            .collect();
        assert_eq!(
            found,
            [
                "<s>/home/.claude-alz-blocked",
                "<s>/home/.codex-alz-blocked"
            ]
        );
    }

    #[test]
    fn a_blocked_folder_is_found_without_opening_anything_in_it() {
        if running_as_root() {
            return;
        }
        let scratch = ScratchDir::new().expect("scratch");
        scratch.make_dirs("home/.claude-alz-blocked").expect("made");
        scratch
            .write("home/.claude-alz-blocked/.credentials.json", "secret")
            .expect("written");
        scratch
            .set_mode("home/.claude-alz-blocked/.credentials.json", 0o000)
            .expect("mode");
        scratch
            .set_mode("home/.claude-alz-blocked", 0o000)
            .expect("mode");
        let found = blocked_folders(&snapshot(&scratch.join("home"), None));
        scratch
            .set_mode("home/.claude-alz-blocked", 0o700)
            .expect("restored");
        scratch
            .set_mode("home/.claude-alz-blocked/.credentials.json", 0o600)
            .expect("restored");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn without_a_home_there_are_no_blocked_folders() {
        let env = EnvSnapshot::from_vars(EnvVars::default());
        assert_eq!(blocked_folders(&env).len(), 0);
    }

    #[test]
    fn the_configured_folder_that_is_also_the_default_is_listed_once() {
        let scratch = ScratchDir::new().expect("scratch");
        scratch.make_dirs("home/.claude").expect("made");
        let env = snapshot(&scratch.join("home"), Some(&scratch.join("home/.claude")));
        assert_eq!(claude_account_folders(&env).len(), 1);
    }

    #[test]
    fn agent_files_skill_folders_and_claude_md_are_listed_by_name() {
        let scratch = ScratchDir::new().expect("scratch");
        scratch.make_dirs("home/.claude/agents").expect("made");
        scratch
            .make_dirs("home/.claude/skills/shared")
            .expect("made");
        scratch
            .write("home/.claude/agents/architect.md", "x")
            .expect("written");
        scratch
            .write("home/.claude/agents/notes.txt", "x")
            .expect("written");
        scratch
            .write("home/.claude/CLAUDE.md", "x")
            .expect("written");
        let folders = claude_account_folders(&snapshot(&scratch.join("home"), None));
        let folder = folders.first().expect("one folder");
        assert_eq!(folder.agents.iter().collect::<Vec<_>>(), ["architect"]);
        assert_eq!(folder.skills.iter().collect::<Vec<_>>(), ["shared"]);
        assert!(folder.has_claude_md);
    }

    #[test]
    fn no_file_inside_an_account_folder_is_opened() {
        if running_as_root() {
            return;
        }
        let scratch = ScratchDir::new().expect("scratch");
        scratch.make_dirs("home/.claude/agents").expect("made");
        scratch
            .make_dirs("home/.claude/skills/shared")
            .expect("made");
        let files = [
            "home/.claude/agents/architect.md",
            "home/.claude/skills/shared/SKILL.md",
            "home/.claude/CLAUDE.md",
            "home/.claude/.credentials.json",
            "home/.claude.json",
        ];
        for file in files {
            scratch.write(file, "secret").expect("written");
            scratch.set_mode(file, 0o000).expect("mode");
        }
        let folders = claude_account_folders(&snapshot(&scratch.join("home"), None));
        let folder = folders.first().expect("one folder");
        assert_eq!(folder.agents.iter().collect::<Vec<_>>(), ["architect"]);
        assert_eq!(folder.skills.iter().collect::<Vec<_>>(), ["shared"]);
        assert!(folder.has_claude_md);
        // Every file has mode 000, so an open of any of them would have failed; the names above
        // came from listings and `lstat` alone. The modes are restored so the folder is removed.
        for file in files {
            scratch.set_mode(file, 0o600).expect("restored");
        }
    }
}
