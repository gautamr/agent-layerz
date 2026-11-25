//! The git calls on the repository being built (spec §9.4): the place of `.git/config`, the
//! write of the declared identity, the checks right before and right after it, and the
//! environment of every call. Git is real, in temporary repositories with an isolated `HOME`,
//! `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_NOSYSTEM`, so no test reads or writes the user's own git
//! configuration. A stand-in `git` first on `PATH` swaps a path or records the environment, then
//! runs the real one.
//!
//! These tests live in the `alz` crate because the runner starts git through the `alz` binary.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one; an integration test crate is compiled with `cfg(test)` already.
#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};

    use alz_core::env::EnvVars;
    use alz_core::identity::{
        Blocker, CommitIdentity, ConfigAnswer, ConfigKey, DeclaredBy, DeclaredIdentity,
        EmailAddress, Field, GitConfigPlace, GitScope, IdentityName, KeyState, RepoState, Report,
    };
    use alz_core::layer::LayerName;
    use alz_core::path::AbsolutePath;
    use alz_core::stack::Source;
    use alz_io::{
        Changed, Deadlines, EnvSnapshot, GitConfigSeal, GitRunner, KeyWrite, Launcher, PlaceChecks,
        RepoRoot, RepositoryGit, SetupResult, Stopped,
    };
    use alz_test_support::ScratchDir;
    use assert_cmd::Command;

    const NAME: &str = "Declared Owner";
    const EMAIL: &str = "owner@example.com";

    fn declared() -> CommitIdentity {
        CommitIdentity::new(
            IdentityName::try_from(NAME).expect("a name"),
            EmailAddress::try_from(EMAIL).expect("an email"),
        )
    }

    /// `base/repo` (a repository made by git), `base/outside` (what nothing may change),
    /// `base/home` and `base/bin`.
    struct Place {
        scratch: ScratchDir,
        base: PathBuf,
    }

    fn text(path: &Path) -> &str {
        path.to_str().expect("a path of text")
    }

    impl Place {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let base = scratch.real_path().expect("a real path");
            for name in ["repo", "outside", "home", "bin"] {
                scratch.make_dir(name).expect("made");
            }
            let place = Self { scratch, base };
            place.git_in(&place.repo(), &["init", "--quiet"]);
            place
        }

        // Files go through the scratch directory, which takes an absolute path as it is.
        fn mkdir(&self, path: impl AsRef<Path>) {
            self.scratch.make_dir(path).expect("made");
        }

        fn read(&self, path: impl AsRef<Path>) -> Vec<u8> {
            self.scratch.read(path).expect("read")
        }

        fn read_text(&self, path: impl AsRef<Path>) -> String {
            self.scratch.read_text(path).expect("read")
        }

        fn write(&self, path: impl AsRef<Path>, contents: impl AsRef<[u8]>) {
            self.scratch.write(path, contents).expect("written");
        }

        fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
            self.scratch.rename(from, to).expect("moved");
        }

        fn remove(&self, path: impl AsRef<Path>) {
            self.scratch.remove_file(path).expect("removed");
        }

        fn symlink(&self, target: impl AsRef<Path>, link: impl AsRef<Path>) {
            self.scratch.make_link(link, target).expect("linked");
        }

        fn fifo(&self, path: impl AsRef<Path>) {
            self.scratch.make_fifo(path).expect("a pipe");
        }

        fn exists(&self, path: impl AsRef<Path>) -> bool {
            self.scratch.exists(path)
        }

        fn is_link(path: &Path) -> bool {
            Command::new("test")
                .args(["-L", text(path)])
                .output()
                .expect("test runs")
                .status
                .success()
        }

        fn repo(&self) -> PathBuf {
            self.base.join("repo")
        }

        fn outside(&self) -> PathBuf {
            self.base.join("outside")
        }

        fn global_config(&self) -> PathBuf {
            self.base.join("home").join("gitconfig")
        }

        /// The variables of every git call of a test, apart from `PATH`.
        fn pairs(&self) -> Vec<(OsString, OsString)> {
            [
                ("HOME", self.base.join("home").into_os_string()),
                ("GIT_CONFIG_GLOBAL", self.global_config().into_os_string()),
                ("GIT_CONFIG_NOSYSTEM", OsString::from("1")),
                ("GIT_AUTHOR_DATE", OsString::from("2020-01-01T00:00:00Z")),
                ("GIT_COMMITTER_DATE", OsString::from("2020-01-01T00:00:00Z")),
            ]
            .into_iter()
            .map(|(name, value)| (OsString::from(name), value))
            .collect()
        }

        fn real_path() -> OsString {
            EnvSnapshot::capture()
                .vars()
                .get("PATH")
                .expect("PATH is set")
                .to_owned()
        }

        /// Runs the real git in `directory` with the isolated environment and nothing else.
        fn git_in(&self, directory: &Path, args: &[&str]) -> String {
            let output = Command::new("git")
                .args(args)
                .current_dir(directory)
                .env_clear()
                .env("PATH", Self::real_path())
                .envs(self.pairs())
                .output()
                .expect("git runs");
            assert!(
                output.status.success(),
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).into_owned()
        }

        /// A stand-in `git` that runs `before` and then the real git with the same arguments.
        fn stand_in(&self, before: &str) {
            let real = Command::new("sh")
                .args(["-c", "command -v git"])
                .env("PATH", Self::real_path())
                .output()
                .expect("sh runs");
            let real = String::from_utf8_lossy(&real.stdout).trim().to_owned();
            let script = format!("#!/bin/sh\nREAL='{real}'\n{before}\nexec \"$REAL\" \"$@\"\n");
            self.scratch
                .write_executable(self.base.join("bin").join("git"), script)
                .expect("the stand-in is written");
        }

        /// Runs `check` with the calls on `directory`, with `extra` added to the isolated
        /// environment. The stand-in, when there is one, comes first on `PATH`.
        fn on<T>(
            &self,
            directory: &Path,
            extra: &[(&str, &OsStr)],
            check: impl FnOnce(&RepositoryGit<'_>, &RepoRoot) -> T,
        ) -> T {
            let mut path = self.base.join("bin").into_os_string();
            path.push(":");
            path.push(Self::real_path());
            let vars: EnvVars = self
                .pairs()
                .into_iter()
                .chain([(OsString::from("PATH"), path)])
                .chain(
                    extra
                        .iter()
                        .map(|(name, value)| (OsString::from(name), (*value).to_owned())),
                )
                .collect();
            let env = EnvSnapshot::from_vars(vars);
            let runner = GitRunner::with_launcher(
                Launcher::at(env!("CARGO_BIN_EXE_alz")),
                &env,
                Deadlines::STANDARD,
            )
            .expect("a runner");
            let absolute = AbsolutePath::try_from(text(directory)).expect("absolute");
            let root = RepoRoot::at(&env, &absolute).expect("the folder opens");
            check(&RepositoryGit::new(&runner, &root, directory), &root)
        }

        fn config_value(&self, key: &str) -> Option<String> {
            let file = self.repo().join(".git").join("config");
            Command::new("git")
                .args(["config", "--file", text(&file), "--get", key])
                .env_clear()
                .env("PATH", Self::real_path())
                .envs(self.pairs())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        }
    }

    // --- The place --------------------------------------------------------------------------

    #[test]
    fn an_ordinary_repository_is_a_writable_place() {
        let place = Place::new();
        let found = place.on(&place.repo(), &[], |git, root| {
            (git.place(), root.git_config_place())
        });
        assert_eq!(found, (GitConfigPlace::Writable, GitConfigPlace::Writable));
    }

    #[test]
    fn a_root_below_the_top_level_is_not_writable_and_nothing_is_written() {
        let place = Place::new();
        let below = place.repo().join("below");
        place.mkdir(&below);
        let before = place.read(place.repo().join(".git/config"));
        let result = place.on(&below, &[], |git, _| (git.place(), git.set_up(&declared())));
        assert_eq!(
            result,
            (
                GitConfigPlace::Blocked(Blocker::NotTopLevel),
                SetupResult::Blocked(Blocker::NotTopLevel)
            )
        );
        assert_eq!(place.read(place.repo().join(".git/config")), before);
    }

    #[test]
    fn the_gitfile_of_a_linked_worktree_is_not_a_plain_git_directory() {
        let place = Place::new();
        place.git_in(
            &place.repo(),
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@example.com",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                "first",
            ],
        );
        let worktree = place.base.join("wt");
        place.git_in(
            &place.repo(),
            &["worktree", "add", "--quiet", text(&worktree)],
        );
        let before = place.read(place.repo().join(".git/config"));
        let result = place.on(&worktree, &[], |git, _| git.set_up(&declared()));
        assert_eq!(result, SetupResult::Blocked(Blocker::GitNotADirectory));
        assert_eq!(place.read(place.repo().join(".git/config")), before);
        assert_eq!(place.config_value("user.name"), None);
    }

    #[test]
    fn a_linked_git_folder_is_refused_and_the_folder_it_names_is_unchanged() {
        let place = Place::new();
        let real = place.outside().join("real.git");
        place.rename(place.repo().join(".git"), &real);
        place.symlink(&real, place.repo().join(".git"));
        let before = place.read(real.join("config"));
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        assert_eq!(result, SetupResult::Blocked(Blocker::GitNotADirectory));
        assert_eq!(place.read(real.join("config")), before);
    }

    #[test]
    fn a_link_at_the_config_file_is_refused_and_the_file_it_names_is_unchanged() {
        let place = Place::new();
        let target = place.outside().join("target.conf");
        place.write(&target, "[core]\n\tbare = false\n");
        let config = place.repo().join(".git/config");
        place.remove(&config);
        place.symlink(&target, &config);
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        assert_eq!(result, SetupResult::Blocked(Blocker::ConfigNotRegular));
        assert_eq!(place.read_text(&target), "[core]\n\tbare = false\n");
        assert!(Place::is_link(&config), "the link is still in place");
    }

    #[test]
    fn a_relative_link_that_stays_inside_git_is_still_not_a_regular_file() {
        let place = Place::new();
        let git = place.repo().join(".git");
        place.rename(git.join("config"), git.join("config.real"));
        place.symlink("config.real", git.join("config"));
        let before = place.read(git.join("config.real"));
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        assert_eq!(result, SetupResult::Blocked(Blocker::ConfigNotRegular));
        assert_eq!(place.read(git.join("config.real")), before);
    }

    #[test]
    fn a_pipe_at_the_config_file_is_refused_without_being_opened_for_a_read() {
        let place = Place::new();
        let config = place.repo().join(".git/config");
        place.remove(&config);
        place.fifo(&config);
        let found = place.on(&place.repo(), &[], |git, root| {
            (root.git_config_place(), git.set_up(&declared()))
        });
        assert_eq!(
            found,
            (
                GitConfigPlace::Blocked(Blocker::ConfigNotRegular),
                SetupResult::Blocked(Blocker::ConfigNotRegular)
            )
        );
    }

    #[test]
    fn a_missing_config_file_is_writable_and_git_creates_it() {
        let place = Place::new();
        place.remove(place.repo().join(".git/config"));
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        assert!(matches!(result, SetupResult::Planned { stopped: None, .. }));
        assert_eq!(place.config_value("user.name").as_deref(), Some(NAME));
        assert_eq!(place.config_value("user.email").as_deref(), Some(EMAIL));
    }

    // --- The write --------------------------------------------------------------------------

    #[test]
    fn only_the_missing_keys_are_added_and_a_different_value_is_kept() {
        let place = Place::new();
        place.git_in(
            &place.repo(),
            &["config", "--local", "user.name", "Someone Else"],
        );
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        let SetupResult::Planned {
            plan,
            written,
            stopped,
        } = result
        else {
            panic!("the place is writable");
        };
        assert_eq!(written, vec![(Field::Email, KeyWrite::Written)]);
        assert_eq!(stopped, None);
        assert_eq!(plan.kept().collect::<Vec<_>>(), vec![Field::Name]);
        assert_eq!(
            place.config_value("user.name").as_deref(),
            Some("Someone Else")
        );
        assert_eq!(place.config_value("user.email").as_deref(), Some(EMAIL));
        // A second run has nothing left to add: the equal value is left alone.
        let again = place.on(&place.repo(), &[], |git, _| git.key_states(&declared()));
        assert_eq!(again, (KeyState::Different, KeyState::Equal));
        let SetupResult::Planned { written, .. } =
            place.on(&place.repo(), &[], |git, _| git.set_up(&declared()))
        else {
            panic!("the place is writable");
        };
        assert_eq!(written, vec![]);
    }

    #[test]
    fn a_write_never_touches_the_global_or_the_system_configuration() {
        let place = Place::new();
        place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        assert!(
            !place.exists(place.global_config()),
            "no global file was made"
        );
        assert_eq!(place.config_value("user.name").as_deref(), Some(NAME));
        // The written identity is what git takes for a commit here.
        let verdicts = place.on(&place.repo(), &[], |git, _| git.verdicts(&declared()));
        assert!(verdicts.author.matches() && verdicts.committer.matches());
    }

    #[test]
    fn an_include_that_overrides_the_written_key_is_reported_after_the_re_read() {
        let place = Place::new();
        place.write(
            place.repo().join(".git/override.conf"),
            "[user]\n\temail = elsewhere@example.com\n",
        );
        place.git_in(&place.repo(), &["config", "--local", "user.name", NAME]);
        let mut config = place.read_text(place.repo().join(".git/config"));
        config.push_str("[include]\n\tpath = override.conf\n");
        place.write(place.repo().join(".git/config"), config);
        let (written, verdicts, answer) = place.on(&place.repo(), &[], |git, _| {
            let SetupResult::Planned { written, .. } = git.set_up(&declared()) else {
                panic!("the place is writable");
            };
            (
                written,
                git.verdicts(&declared()),
                git.config_answer(ConfigKey::User(Field::Email)),
            )
        });
        assert_eq!(written, vec![(Field::Email, KeyWrite::Written)]);
        assert!(!verdicts.author.matches(), "the include wins over the key");
        let ConfigAnswer::Set(origin) = answer else {
            panic!("git reports an origin");
        };
        assert_eq!(origin.scope(), GitScope::Local);
        assert!(origin.origin().as_str().ends_with("override.conf"));
    }

    // --- The checks around the write --------------------------------------------------------

    #[test]
    fn a_link_put_at_the_config_file_between_the_check_and_the_write_is_found_by_the_check_after() {
        let place = Place::new();
        let target = place.outside().join("target.conf");
        place.write(&target, "[core]\n\tbare = false\n");
        place.stand_in(
            r#"if [ "$1" = config ] && [ "$2" = --file ] && [ "$4" = --end-of-options ]; then
  rm -f "$3"; ln -s "$ALZ_TEST_TARGET" "$3"
fi"#,
        );
        let result = place.on(
            &place.repo(),
            &[("ALZ_TEST_TARGET", target.as_os_str())],
            |git, _| git.set_up(&declared()),
        );
        let SetupResult::Planned {
            written, stopped, ..
        } = result
        else {
            panic!("the place was writable when checked");
        };
        assert_eq!(
            stopped,
            Some(Stopped::Tampered {
                field: Field::Name,
                changed: Changed::Config
            })
        );
        assert!(written.is_empty(), "nothing is reported as written");
        let outside = place.read_text(&target);
        assert!(
            !outside.contains(EMAIL),
            "nothing more was written after the failed check"
        );
    }

    #[test]
    fn a_git_folder_swapped_for_a_link_between_the_check_and_the_write_is_found_by_the_check_after()
    {
        let place = Place::new();
        place.stand_in(
            r#"if [ "$1" = config ] && [ "$2" = --file ] && [ "$4" = --end-of-options ]; then
  mv .git .git.moved; ln -s .git.moved .git
fi"#,
        );
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        let SetupResult::Planned { stopped, .. } = result else {
            panic!("the place was writable when checked");
        };
        assert_eq!(
            stopped,
            Some(Stopped::Tampered {
                field: Field::Name,
                changed: Changed::Git
            })
        );
        assert!(
            !place
                .read_text(place.repo().join(".git.moved/config"))
                .contains(EMAIL)
        );
    }

    #[test]
    fn a_config_file_that_the_write_turned_into_a_pipe_is_found_by_the_check_after_it() {
        let place = Place::new();
        // The stand-in turns the config file into a pipe after the first write.
        place.stand_in(
            r#"if [ "$1" = config ] && [ "$2" = --file ] && [ "$4" = --end-of-options ]; then
  "$REAL" "$@"; status=$?; rm -f "$3"; mkfifo "$3"; exit $status
fi"#,
        );
        let result = place.on(&place.repo(), &[], |git, _| git.set_up(&declared()));
        let SetupResult::Planned { stopped, .. } = result else {
            panic!("the place was writable when checked");
        };
        assert_eq!(
            stopped,
            Some(Stopped::Tampered {
                field: Field::Name,
                changed: Changed::Config
            })
        );
    }

    /// The checks of the root, except that the check before the second write fails as it does
    /// when `.git/config` has become a link between the two writes, which only a race does on a
    /// real disk (spec §9.4 "Writing", §10.3 the commit identity step).
    struct FailsBeforeTheSecondWrite<'a> {
        root: &'a RepoRoot,
        seals: Cell<u32>,
    }

    impl PlaceChecks for FailsBeforeTheSecondWrite<'_> {
        fn seal(&self) -> Result<GitConfigSeal, Blocker> {
            self.seals.set(self.seals.get() + 1);
            if self.seals.get() >= 2 {
                Err(Blocker::ConfigNotRegular)
            } else {
                self.root.seal_git_config()
            }
        }

        fn verify(&self, seal: GitConfigSeal) -> Result<(), Changed> {
            self.root.verify_git_config(seal)
        }
    }

    /// Spec §9.4 "Writing" and §10.3 the commit identity step: a failed check before the second
    /// write stops it. The first key stays written and is printed as `set user.name in
    /// .git/config to '<value>' (declared by <by>)`, the second key is not written, the stop is
    /// the `Refused` blocker, and the line that follows is the "Setup, when alz may not write"
    /// warning with its reason and its advice (`Run by hand: <step>; <step>`).
    #[test]
    fn a_failed_check_before_the_second_write_leaves_the_second_key_unwritten_and_is_reported_as_a_blocked_place()
     {
        let place = Place::new();
        let (result, seals) = place.on(&place.repo(), &[], |git, root| {
            let checks = FailsBeforeTheSecondWrite {
                root,
                seals: Cell::new(0),
            };
            (
                git.set_up_checked_by(&declared(), &checks),
                checks.seals.get(),
            )
        });
        let SetupResult::Planned {
            written, stopped, ..
        } = result
        else {
            panic!("the place was writable when the plan was made");
        };
        assert_eq!(seals, 2, "the place is checked before each write");
        assert_eq!(written, vec![(Field::Name, KeyWrite::Written)]);
        assert_eq!(stopped, Some(Stopped::Refused(Blocker::ConfigNotRegular)));
        assert_eq!(place.config_value("user.name").as_deref(), Some(NAME));
        assert_eq!(
            place.config_value("user.email"),
            None,
            "the second key is not written"
        );

        // The lines `alz` prints for that result: the first key's line, then the warning.
        let source = Source::try_from("path:../client").expect("a source");
        let layer = LayerName::try_from("client").expect("a layer name");
        let declared_identity = DeclaredIdentity::new(DeclaredBy::new(&layer, &source), declared());
        let report = Report::new(&declared_identity, None);
        let by = "layer 'client' (path:../client)";
        assert_eq!(
            report.written_line(Field::Name).to_string(),
            format!("set user.name in .git/config to '{NAME}' (declared by {by})")
        );
        assert_eq!(
            report.blocked_line(Blocker::ConfigNotRegular).to_string(),
            format!(
                "warning [identity] .git/config is not a regular file, so alz did not write the git identity declared by {by}. Run by hand: git config --local user.name '{NAME}'; git config --local user.email '{EMAIL}'"
            )
        );
    }

    // --- The environment --------------------------------------------------------------------

    const RECORD: &str = r#"{ echo "-- $1"; env | sort; } >> "$ALZ_TEST_RECORD""#;

    #[test]
    fn variables_that_point_at_another_repository_change_nothing_and_the_others_are_honoured() {
        let place = Place::new();
        let other = place.outside().join("other");
        place.mkdir(&other);
        place.git_in(&other, &["init", "--quiet"]);
        let other_config = place.read(other.join(".git/config"));
        let record = place.base.join("record");
        place.stand_in(RECORD);
        let other_git = other.join(".git");
        let stray_config = place.outside().join("stray.conf");
        let stray_index = place.outside().join("stray.index");
        let global = place.global_config();
        place.write(&global, "[user]\n\tname = Global Person\n");
        let extra: Vec<(&str, &OsStr)> = vec![
            ("ALZ_TEST_RECORD", record.as_os_str()),
            ("GIT_DIR", other_git.as_os_str()),
            ("GIT_WORK_TREE", other.as_os_str()),
            ("GIT_COMMON_DIR", other_git.as_os_str()),
            ("GIT_CONFIG", stray_config.as_os_str()),
            ("GIT_INDEX_FILE", stray_index.as_os_str()),
            ("GIT_AUTHOR_NAME", OsStr::new("Env Author")),
            ("GIT_CONFIG_COUNT", OsStr::new("1")),
            ("GIT_CONFIG_KEY_0", OsStr::new("user.email")),
            ("GIT_CONFIG_VALUE_0", OsStr::new("count@example.com")),
        ];
        let (result, verdicts, name, email) = place.on(&place.repo(), &extra, |git, _| {
            (
                git.set_up(&declared()),
                git.verdicts(&declared()),
                git.config_answer(ConfigKey::User(Field::Name)),
                git.config_answer(ConfigKey::User(Field::Email)),
            )
        });
        assert!(matches!(result, SetupResult::Planned { stopped: None, .. }));
        // The identity went into this repository, and nothing the variables name changed.
        assert_eq!(place.config_value("user.name").as_deref(), Some(NAME));
        assert_eq!(place.read(other.join(".git/config")), other_config);
        assert!(!place.exists(&stray_config) && !place.exists(&stray_index));
        // GIT_AUTHOR_NAME, GIT_CONFIG_COUNT and GIT_CONFIG_GLOBAL are honoured.
        assert!(!verdicts.author.matches());
        let ConfigAnswer::Set(origin) = email else {
            panic!("the command scope has the email");
        };
        assert_eq!(origin.scope(), GitScope::Command);
        assert!(matches!(name, ConfigAnswer::Set(_)));
        // What every call was given.
        let recorded = place.read_text(&record);
        // `git version`, which the runner makes when it is built, is a source call with its own
        // environment (spec §6) and is not among the calls on this repository.
        let calls: Vec<&str> = recorded
            .split("-- ")
            .filter(|call| !call.is_empty() && !call.starts_with("version\n"))
            .collect();
        assert!(calls.len() >= 9, "every call was recorded");
        for call in calls {
            let names: Vec<&str> = call
                .lines()
                .filter_map(|line| line.split_once('=').map(|(name, _)| name))
                .collect();
            for dropped in [
                "GIT_DIR",
                "GIT_WORK_TREE",
                "GIT_COMMON_DIR",
                "GIT_CONFIG",
                "GIT_INDEX_FILE",
            ] {
                assert!(!names.contains(&dropped), "{dropped} reached git");
            }
            for kept in [
                "GIT_AUTHOR_NAME",
                "GIT_CONFIG_COUNT",
                "GIT_CONFIG_GLOBAL",
                "GIT_CONFIG_NOSYSTEM",
            ] {
                assert!(names.contains(&kept), "{kept} did not reach git");
            }
            assert!(call.contains("GIT_TERMINAL_PROMPT=0\n"));
        }
    }

    #[test]
    fn the_message_of_a_directory_that_is_not_a_repository_is_read_in_the_c_locale() {
        let place = Place::new();
        let plain = place.base.join("plain");
        place.mkdir(&plain);
        let state = place.on(
            &plain,
            &[
                ("LANG", OsStr::new("de_DE.UTF-8")),
                ("LC_ALL", OsStr::new("de_DE.UTF-8")),
            ],
            |git, _| git.state(),
        );
        assert_eq!(state, RepoState::NotARepository);
        let repository = place.on(&place.repo(), &[], |git, _| git.state());
        assert_eq!(repository, RepoState::Repository);
    }

    #[test]
    fn a_reported_value_never_reaches_a_result_that_can_be_printed() {
        let place = Place::new();
        place.write(
            place.global_config(),
            "[user]\n\tname = Wrong Secret Name\n\temail = wrong.secret@example.com\n",
        );
        let shown = place.on(&place.repo(), &[], |git, _| {
            format!(
                "{:?} {:?} {:?} {:?}",
                git.verdicts(&declared()),
                git.config_answer(ConfigKey::User(Field::Name)),
                git.config_answer(ConfigKey::User(Field::Email)),
                git.key_states(&declared()),
            )
        });
        assert!(!shown.contains("Wrong Secret Name"), "{shown}");
        assert!(!shown.contains("wrong.secret"), "{shown}");
        assert!(shown.contains("Global"), "the scope is reported: {shown}");
    }
}
