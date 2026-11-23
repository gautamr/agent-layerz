//! The git runner (spec §6 "Git subprocesses"), through the real `alz` binary as its launcher:
//! typed calls, the cleared environment, the version floor, the deadline that ends the process
//! group, and the absence of a controlling terminal. Git is the real one, or a stand-in script
//! that records what it was given.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one; an integration test crate is compiled with `cfg(test)` already.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::{OsStr, OsString};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use alz_core::env::EnvVars;
    use alz_core::git::{CloneTransport, EntryMode, ObjectCheck, ObjectContent, ObjectKind};
    use alz_core::lock::ObjectId;
    use alz_core::path::SafeRelPath;
    use alz_core::stack::GitRef;
    use alz_io::{
        Deadlines, EnvSnapshot, GitFailure, GitRunner, GitRunnerError, Launcher, ReadScope,
    };
    use alz_test_support::{ScratchDir, start_silently, test_executable};
    use assert_cmd::Command;

    const SHA: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";

    fn scratch() -> ScratchDir {
        ScratchDir::new().expect("a scratch directory")
    }

    fn launcher() -> Launcher {
        Launcher::at(env!("CARGO_BIN_EXE_alz"))
    }

    fn id(text: &str) -> ObjectId {
        ObjectId::try_from(text).expect("an object id")
    }

    /// A snapshot of exactly `pairs`.
    fn snapshot(pairs: &[(&str, &OsStr)]) -> EnvSnapshot {
        EnvSnapshot::from_vars(
            pairs
                .iter()
                .map(|(name, value)| (OsString::from(name), (*value).to_owned()))
                .collect::<EnvVars>(),
        )
    }

    /// The `PATH` of this test run, where the real git lives.
    fn real_path() -> OsString {
        EnvSnapshot::capture()
            .vars()
            .get("PATH")
            .expect("PATH is set")
            .to_owned()
    }

    /// A `PATH` with `<dir>/bin` first, then only the system's own directories.
    fn path_with_bin(dir: &ScratchDir) -> OsString {
        let mut path = dir.join("bin").into_os_string();
        path.push(":/usr/bin:/bin");
        path
    }

    /// Plants `script` as `<dir>/bin/git`.
    fn install_git(dir: &ScratchDir, script: &str) {
        dir.make_dir("bin").unwrap();
        dir.write_executable("bin/git", script).unwrap();
    }

    // --- A stand-in git that records its call ------------------------------------------------

    /// A `git` that answers `git version`, and for any other call records its arguments,
    /// environment, directory and standard input in `$ALZ_TEST_RECORD`, then prints
    /// `$ALZ_TEST_REPLY`.
    const RECORDING_GIT: &str = r#"#!/bin/sh
if [ "$1" = version ]; then echo "git version 2.40.0"; exit 0; fi
for a in "$@"; do printf '%s\n' "$a"; done > "$ALZ_TEST_RECORD/argv"
env | sort > "$ALZ_TEST_RECORD/env"
pwd -P > "$ALZ_TEST_RECORD/pwd"
cat > "$ALZ_TEST_RECORD/stdin"
printf '%b' "$ALZ_TEST_REPLY"
"#;

    struct Recorded {
        argv: Vec<String>,
        env: BTreeMap<String, String>,
        stdin: String,
        pwd: String,
    }

    fn lines(text: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(text)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Runs `call` with the recording git on `PATH`; `extra` joins the snapshot, which holds
    /// nothing else but `PATH`, `HOME` and the two variables the stand-in reads.
    fn recorded<T>(
        reply: &str,
        extra: &[(&str, &OsStr)],
        call: impl FnOnce(&GitRunner) -> T,
    ) -> (T, Recorded) {
        let dir = scratch();
        install_git(&dir, RECORDING_GIT);
        dir.make_dir("record").unwrap();
        let path = path_with_bin(&dir);
        let home = dir.path().as_os_str().to_owned();
        let record = dir.join("record").into_os_string();
        let reply = OsString::from(reply);
        let mut pairs: Vec<(&str, &OsStr)> = vec![
            ("PATH", &path),
            ("HOME", &home),
            ("ALZ_TEST_RECORD", &record),
            ("ALZ_TEST_REPLY", &reply),
        ];
        pairs.extend_from_slice(extra);
        let runner = GitRunner::with_launcher(launcher(), &snapshot(&pairs), Deadlines::STANDARD)
            .expect("the stand-in's version is accepted");
        let result = call(&runner);
        let recorded = Recorded {
            argv: lines(&dir.read("record/argv").expect("git was called")),
            env: lines(&dir.read("record/env").unwrap())
                .iter()
                .filter_map(|line| line.split_once('='))
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
            stdin: String::from_utf8_lossy(&dir.read("record/stdin").unwrap()).into_owned(),
            pwd: String::from_utf8_lossy(&dir.read("record/pwd").unwrap())
                .trim()
                .to_owned(),
        };
        (result, recorded)
    }

    #[test]
    fn every_call_runs_git_with_the_argument_array_of_its_type_and_standard_input_only_for_batches()
    {
        let repository = scratch();
        let repo = repository.path();
        let commit = id(SHA);
        let rev = GitRef::try_from("v1.0").unwrap();
        let no_vars: &[(&str, &OsStr)] = &[];

        let (result, call) = recorded("", no_vars, |git| {
            git.clone_source(
                CloneTransport::Remote,
                OsStr::new("-oProxyCommand=evil"),
                Path::new("/tmp/destination"),
            )
        });
        result.unwrap();
        assert_eq!(
            call.argv,
            [
                "-c",
                "protocol.allow=never",
                "-c",
                "protocol.https.allow=always",
                "-c",
                "protocol.ssh.allow=always",
                "clone",
                "--quiet",
                "--",
                "-oProxyCommand=evil",
                "/tmp/destination"
            ]
        );
        assert_eq!(call.stdin, "");

        let (result, call) = recorded("", no_vars, |git| {
            git.clone_source(CloneTransport::Local, OsStr::new("/src"), Path::new("/dst"))
        });
        result.unwrap();
        assert_eq!(
            call.argv,
            ["clone", "--quiet", "--no-local", "--", "/src", "/dst"]
        );

        let (result, call) = recorded(&format!("{SHA}\\n"), no_vars, |git| {
            git.rev_parse_commit(repo, ReadScope::CacheEntry, &rev)
        });
        assert_eq!(result.unwrap(), Some(commit));
        assert_eq!(
            call.argv,
            [
                "rev-parse",
                "--verify",
                "--quiet",
                "--end-of-options",
                "v1.0^{commit}"
            ]
        );
        assert_eq!(call.stdin, "");
        assert!(
            call.pwd
                .ends_with(repo.file_name().unwrap().to_str().unwrap())
        );

        let (result, call) = recorded("", no_vars, |git| git.checkout(repo, &commit));
        result.unwrap();
        assert_eq!(
            call.argv,
            [
                "-c",
                "advice.detachedHead=false",
                "checkout",
                "--quiet",
                "--end-of-options",
                SHA
            ]
        );
    }

    #[test]
    fn the_tree_and_object_calls_run_git_with_the_argument_array_of_their_type_and_batches_feed_standard_input()
     {
        let repository = scratch();
        let repo = repository.path();
        let commit = id(SHA);
        let path = SafeRelPath::try_from("layers/-x").unwrap();
        let objects = [id(SHA)];
        let no_vars: &[(&str, &OsStr)] = &[];

        let (result, call) = recorded("/a/b\\n", no_vars, |git| git.show_toplevel(repo));
        assert_eq!(result.unwrap(), Path::new("/a/b"));
        assert_eq!(call.argv, ["rev-parse", "--show-toplevel"]);

        let (result, call) = recorded("layers/x/\\n", no_vars, |git| git.show_prefix(repo));
        assert_eq!(result.unwrap(), OsString::from("layers/x/"));
        assert_eq!(call.argv, ["rev-parse", "--show-prefix"]);

        let (result, call) = recorded("", no_vars, |git| git.ls_tree_at(repo, &commit, &path));
        assert_eq!(result.unwrap(), vec![]);
        assert_eq!(
            call.argv,
            ["ls-tree", "-z", "--end-of-options", SHA, "layers/-x"]
        );

        let (result, call) = recorded("", no_vars, |git| git.ls_tree_recursive(repo, &commit));
        assert_eq!(result.unwrap(), vec![]);
        assert_eq!(call.argv, ["ls-tree", "-r", "-z", "--end-of-options", SHA]);

        let (result, call) = recorded(&format!("{SHA} blob 3\\n"), no_vars, |git| {
            git.cat_file_batch_check(repo, &objects)
        });
        assert_eq!(
            result.unwrap(),
            [ObjectCheck::Present {
                id: commit,
                kind: ObjectKind::Blob,
                size: 3
            }]
        );
        assert_eq!(call.argv, ["cat-file", "--batch-check"]);
        assert_eq!(call.stdin, format!("{SHA}\n"));

        let (result, call) = recorded(&format!("{SHA} blob 2\\nhi\\n"), no_vars, |git| {
            git.cat_file_batch(repo, &objects)
        });
        assert_eq!(
            result.unwrap(),
            [ObjectContent::Present {
                id: commit,
                kind: ObjectKind::Blob,
                bytes: b"hi".to_vec()
            }]
        );
        assert_eq!(call.argv, ["cat-file", "--batch"]);
        assert_eq!(call.stdin, format!("{SHA}\n"));
    }

    #[test]
    fn git_gets_a_cleared_environment_plus_the_child_environment_of_its_operation() {
        let nowhere = OsString::from("/nowhere");
        let one = OsString::from("1");
        let ssh = OsString::from("my-ssh -v");
        let file = OsString::from("file");
        let extra: Vec<(&str, &OsStr)> = vec![
            ("GIT_DIR", &nowhere),
            ("GIT_INDEX_FILE", &nowhere),
            ("GIT_WORK_TREE", &nowhere),
            ("GIT_CONFIG_COUNT", &one),
            ("GIT_CONFIG_KEY_0", &one),
            ("GIT_ALLOW_PROTOCOL", &file),
            ("GIT_SSH_COMMAND", &ssh),
            ("KEEP_ME", &one),
        ];
        // The stand-in is a shell, which adds a few variables of its own.
        let shell_noise = ["PWD", "SHLVL", "_", "OLDPWD"];
        let expected_names = |added: &[&str]| -> Vec<String> {
            let mut names: Vec<String> = [
                "PATH",
                "HOME",
                "ALZ_TEST_RECORD",
                "ALZ_TEST_REPLY",
                "GIT_SSH_COMMAND",
                "KEEP_ME",
            ]
            .iter()
            .chain(added)
            .map(|name| (*name).to_owned())
            .collect();
            names.sort();
            names
        };
        let names = |call: &Recorded| -> Vec<String> {
            call.env
                .keys()
                .filter(|name| !shell_noise.contains(&name.as_str()))
                .cloned()
                .collect()
        };

        let (result, call) = recorded("", &extra, |git| {
            git.clone_source(CloneTransport::Remote, OsStr::new("x"), Path::new("/d"))
        });
        result.unwrap();
        assert_eq!(
            names(&call),
            expected_names(&[
                "GIT_ALLOW_PROTOCOL",
                "GIT_PROTOCOL_FROM_USER",
                "GIT_TERMINAL_PROMPT"
            ])
        );
        assert_eq!(call.env["GIT_TERMINAL_PROMPT"], "0");
        assert_eq!(call.env["GIT_PROTOCOL_FROM_USER"], "0");
        // The user's list is cut down to the transports of a remote clone: it never widens them.
        assert_eq!(call.env["GIT_ALLOW_PROTOCOL"], "");
        assert_eq!(call.env["GIT_SSH_COMMAND"], "my-ssh -v");
        let (result, call) = recorded("", &[], |git| {
            git.clone_source(CloneTransport::Remote, OsStr::new("x"), Path::new("/d"))
        });
        result.unwrap();
        assert_eq!(call.env["GIT_ALLOW_PROTOCOL"], "https:ssh");

        // A local clone keeps GIT_ALLOW_PROTOCOL as it is; everything else of the parent's is
        // gone.
        let (result, call) = recorded("", &extra, |git| {
            git.clone_source(CloneTransport::Local, OsStr::new("x"), Path::new("/d"))
        });
        result.unwrap();
        assert_eq!(
            names(&call),
            expected_names(&["GIT_ALLOW_PROTOCOL", "GIT_TERMINAL_PROMPT"])
        );

        let repository = scratch();
        let rev = GitRef::try_from("main").unwrap();
        let (_, call) = recorded("", &extra, |git| {
            git.rev_parse_commit(repository.path(), ReadScope::PinnedTree, &rev)
        });
        assert_eq!(
            names(&call),
            expected_names(&[
                "GIT_ALLOW_PROTOCOL",
                "GIT_LITERAL_PATHSPECS",
                "GIT_NO_LAZY_FETCH",
                "GIT_NO_REPLACE_OBJECTS",
                "GIT_TERMINAL_PROMPT"
            ])
        );
        assert_eq!(call.env["GIT_NO_REPLACE_OBJECTS"], "1");
        assert_eq!(call.env["GIT_NO_LAZY_FETCH"], "1");
        assert_eq!(call.env["GIT_LITERAL_PATHSPECS"], "1");
    }

    // --- The version floor -------------------------------------------------------------------

    /// A `git` that logs each call to `$ALZ_TEST_LOG` and prints `$ALZ_TEST_VERSION` for
    /// `git version`.
    const VERSIONED_GIT: &str = r#"#!/bin/sh
echo "$@" >> "$ALZ_TEST_LOG"
if [ "$1" = version ]; then printf '%b' "$ALZ_TEST_VERSION"; exit 0; fi
exit 0
"#;

    /// The result of making a runner with a git that prints `version`, and the calls it logged.
    fn runner_with_version(version: &str) -> (Result<GitRunner, GitRunnerError>, Vec<String>) {
        let dir = scratch();
        install_git(&dir, VERSIONED_GIT);
        let path = path_with_bin(&dir);
        let log = dir.join("log").into_os_string();
        let version = OsString::from(version);
        let home = dir.path().as_os_str().to_owned();
        let env = snapshot(&[
            ("PATH", &path),
            ("HOME", &home),
            ("ALZ_TEST_LOG", &log),
            ("ALZ_TEST_VERSION", &version),
        ]);
        let result = GitRunner::with_launcher(launcher(), &env, Deadlines::STANDARD);
        let calls = dir
            .exists("log")
            .then(|| lines(&dir.read("log").unwrap()))
            .into_iter()
            .flatten()
            .collect();
        (result, calls)
    }

    #[test]
    fn a_git_below_2_30_is_refused_naming_the_found_and_the_minimum_version_before_any_other_call()
    {
        let (result, calls) = runner_with_version("git version 2.29.9 (Test Git-1)\\n");
        let refusal = result.unwrap_err();
        assert_eq!(
            refusal.diagnostic().to_string(),
            "error [git] alz needs git 2.30 or later; found 2.29.9 (Test Git-1)"
        );
        assert_eq!(calls, ["version"]);
    }

    #[test]
    fn a_git_at_2_30_is_accepted_and_asked_for_its_version_once() {
        let (result, calls) = runner_with_version("git version 2.30.0\\n");
        assert!(result.is_ok());
        assert_eq!(calls, ["version"]);
    }

    #[test]
    fn a_git_that_prints_no_version_is_refused_as_unreadable() {
        let (result, _) = runner_with_version("hello\\n");
        assert_eq!(
            result.unwrap_err().diagnostic().to_string(),
            "error [git] alz needs git 2.30 or later; found an unreadable version"
        );
    }

    #[test]
    fn no_git_on_the_path_is_found_none() {
        let dir = scratch();
        dir.make_dir("empty").unwrap();
        let path = dir.join("empty").into_os_string();
        let home = dir.path().as_os_str().to_owned();
        let env = snapshot(&[("PATH", &path), ("HOME", &home)]);
        let refusal = GitRunner::with_launcher(launcher(), &env, Deadlines::STANDARD).unwrap_err();
        assert_eq!(
            refusal.diagnostic().to_string(),
            "error [git] alz needs git 2.30 or later; found none"
        );
    }

    #[test]
    fn an_empty_or_relative_path_entry_never_finds_git() {
        let dir = scratch();
        install_git(&dir, VERSIONED_GIT);
        // `bin/git` exists relative to the working directory of the test run only through this
        // entry, which names no absolute folder.
        let home = dir.path().as_os_str().to_owned();
        let path = OsString::from(":.:bin:relative/bin:");
        let env = snapshot(&[("PATH", &path), ("HOME", &home)]);
        let refusal = GitRunner::with_launcher(launcher(), &env, Deadlines::STANDARD).unwrap_err();
        assert_eq!(
            refusal.diagnostic().to_string(),
            "error [git] alz needs git 2.30 or later; found none"
        );
    }

    #[test]
    fn the_git_that_is_version_checked_is_the_git_that_runs() {
        // Two gits on PATH; the first is the one that answers the version and runs the call, the
        // second is never started.
        let dir = scratch();
        dir.make_dir("first").unwrap();
        dir.make_dir("second").unwrap();
        dir.write_executable(
            "first/git",
            "#!/bin/sh\necho first >> \"$ALZ_TEST_LOG\"\nif [ \"$1\" = version ]; then echo 'git version 2.40.0'; fi\n",
        )
        .unwrap();
        dir.write_executable(
            "second/git",
            "#!/bin/sh\necho second >> \"$ALZ_TEST_LOG\"\n",
        )
        .unwrap();
        let mut path = dir.join("first").into_os_string();
        path.push(":");
        path.push(dir.join("second"));
        let log = dir.join("log").into_os_string();
        let home = dir.path().as_os_str().to_owned();
        let env = snapshot(&[("PATH", &path), ("HOME", &home), ("ALZ_TEST_LOG", &log)]);
        let git = GitRunner::with_launcher(launcher(), &env, Deadlines::STANDARD).expect("a git");
        git.checkout(dir.path(), &ObjectId::try_from(SHA).unwrap())
            .expect("the stand-in succeeds");
        assert_eq!(lines(&dir.read("log").unwrap()), ["first", "first"]);
    }

    #[test]
    fn a_launcher_that_cannot_be_started_is_told_as_the_executable_that_alz_cannot_find() {
        let dir = scratch();
        install_git(&dir, VERSIONED_GIT);
        let path = path_with_bin(&dir);
        let home = dir.path().as_os_str().to_owned();
        let env = snapshot(&[("PATH", &path), ("HOME", &home)]);
        let failure =
            GitRunner::with_launcher(Launcher::at("/nonexistent/alz"), &env, Deadlines::STANDARD)
                .unwrap_err();
        assert_eq!(
            failure.diagnostic().to_string(),
            "error [git] alz cannot find its own executable, which it needs to run git"
        );
    }

    /// A git that prints a path, and leaves a background process behind that holds the output
    /// streams open, as an `ssh` that stays on for later connections does.
    const LINGERING_GIT: &str = r#"#!/bin/sh
if [ "$1" = version ]; then echo "git version 2.40.0"; exit 0; fi
sleep 8 &
echo /the/top
"#;

    #[test]
    fn a_process_that_outlives_git_and_keeps_its_streams_does_not_hold_the_call_to_its_deadline() {
        let dir = scratch();
        install_git(&dir, LINGERING_GIT);
        let path = path_with_bin(&dir);
        let home = dir.path().as_os_str().to_owned();
        let env = snapshot(&[("PATH", &path), ("HOME", &home)]);
        let deadlines = Deadlines {
            clone: Duration::from_secs(60),
            other: Duration::from_secs(60),
        };
        let git = GitRunner::with_launcher(launcher(), &env, deadlines).expect("a git");
        let started = Instant::now();
        let top = git.show_toplevel(dir.path()).expect("git's own result");
        assert_eq!(top, Path::new("/the/top"));
        assert!(
            started.elapsed() < Duration::from_secs(6),
            "waited {:?}",
            started.elapsed()
        );
    }

    // --- The deadline ------------------------------------------------------------------------

    /// A fake `ssh` that records its own pid and its parent's in one file, `pids`, which appears
    /// whole through a rename, then sleeps.
    const SLEEPING_SSH: &str = r#"#!/bin/sh
echo "$$ $PPID" > "$ALZ_PROBE_DIR/pids.partial"
mv "$ALZ_PROBE_DIR/pids.partial" "$ALZ_PROBE_DIR/pids"
exec sleep 300
"#;

    /// The pids that the fake `ssh` wrote: its own, then its parent's, which is git.
    fn recorded_pids(dir: &ScratchDir) -> (String, String) {
        let text = String::from_utf8(dir.read("pids").expect("ssh started")).unwrap();
        let mut pids = text.split_whitespace().map(str::to_owned);
        (
            pids.next().expect("ssh's pid"),
            pids.next().expect("git's pid"),
        )
    }

    fn is_alive(pid: &str) -> bool {
        Command::new("kill")
            .args(["-0", pid])
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Whether `pid` has ended but has not been reaped yet, which `kill -0` still accepts.
    fn is_zombie(pid: &str) -> bool {
        Command::new("ps")
            .args(["-o", "stat=", "-p", pid])
            .output()
            .is_ok_and(|output| output.stdout.trim_ascii().starts_with(b"Z"))
    }

    /// Whether `pid` ends within four seconds; a zombie has ended.
    fn eventually_dead(pid: &str) -> bool {
        (0..40).any(|_| {
            let dead = !is_alive(pid) || is_zombie(pid);
            if !dead {
                std::thread::sleep(Duration::from_millis(100));
            }
            dead
        })
    }

    #[test]
    fn a_call_past_its_deadline_is_killed_with_its_whole_process_group_ssh_included() {
        let dir = scratch();
        dir.write_executable("ssh", SLEEPING_SSH).unwrap();
        let ssh = dir.join("ssh").into_os_string();
        let probe = dir.path().as_os_str().to_owned();
        let home = probe.clone();
        let path = real_path();
        let env = snapshot(&[
            ("PATH", &path),
            ("HOME", &home),
            ("GIT_SSH_COMMAND", &ssh),
            ("ALZ_PROBE_DIR", &probe),
        ]);
        let deadlines = Deadlines {
            clone: Duration::from_secs(10),
            other: Duration::from_secs(60),
        };
        let git = GitRunner::with_launcher(launcher(), &env, deadlines).expect("a git");
        let started = Instant::now();
        let failure = git
            .clone_source(
                CloneTransport::Remote,
                OsStr::new("ssh://example.invalid/layer.git"),
                &dir.join("clone"),
            )
            .unwrap_err();
        assert!(
            matches!(failure, GitFailure::TimedOut { after } if after == Duration::from_secs(10)),
            "{failure:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(30));
        let (ssh_pid, git_pid) = recorded_pids(&dir);
        assert!(eventually_dead(&ssh_pid), "ssh {ssh_pid} is alive");
        assert!(eventually_dead(&git_pid), "git {git_pid} is alive");
    }

    // --- An abandoned call ---------------------------------------------------------------------

    const SLOW_CLONE_TEST: &str = "tests::the_slow_clone_of_a_process_that_gets_interrupted";

    /// Runs as the child of the test below: a clone through the runner, with the standard
    /// deadline, whose `ssh` sleeps for five minutes. It never finishes by itself.
    #[test]
    #[ignore = "runs only as the child that interrupting_alz_during_a_slow_clone_leaves_no_git_or_ssh starts"]
    fn the_slow_clone_of_a_process_that_gets_interrupted() {
        let probe = probe_dir_of("ALZ_SLOW_CLONE_DIR");
        let dir = Path::new(&probe);
        let ssh = dir.join("ssh").into_os_string();
        let path = real_path();
        let env = snapshot(&[
            ("PATH", &path),
            ("HOME", &probe),
            ("GIT_SSH_COMMAND", &ssh),
            ("ALZ_PROBE_DIR", &probe),
        ]);
        let git = GitRunner::with_launcher(launcher(), &env, Deadlines::STANDARD).expect("a git");
        let outcome = git.clone_source(
            CloneTransport::Remote,
            OsStr::new("ssh://example.invalid/layer.git"),
            &dir.join("clone"),
        );
        panic!("the clone was not interrupted: {outcome:?}");
    }

    fn probe_dir_of(name: &str) -> OsString {
        EnvSnapshot::capture()
            .vars()
            .get(name)
            .expect("run by the test that starts this one")
            .to_owned()
    }

    #[test]
    fn interrupting_alz_during_a_slow_clone_leaves_no_git_or_ssh() {
        let dir = scratch();
        dir.write_executable("ssh", SLEEPING_SSH).unwrap();
        let mut alz = start_silently(
            &test_executable().expect("the test executable"),
            &["--exact", SLOW_CLONE_TEST, "--ignored", "--nocapture"],
            &[
                ("ALZ_SLOW_CLONE_DIR", dir.path().as_os_str()),
                ("HOME", dir.path().as_os_str()),
                ("PATH", &real_path()),
            ],
        )
        .expect("the child starts");
        let ssh_started = (0..300).any(|_| {
            let started = dir.exists("pids");
            if !started {
                std::thread::sleep(Duration::from_millis(100));
            }
            started
        });
        assert!(ssh_started, "the fake ssh never started");
        let (ssh_pid, git_pid) = recorded_pids(&dir);
        assert!(is_alive(&ssh_pid), "ssh is running before the interrupt");
        let interrupt = Command::new("kill")
            .args(["-INT", &alz.id().to_string()])
            .output()
            .expect("kill runs");
        assert!(interrupt.status.success(), "{interrupt:?}");
        let status = alz.wait().expect("alz ends");
        assert!(!status.success(), "{status:?}");
        assert!(
            eventually_dead(&ssh_pid),
            "ssh {ssh_pid} is alive after alz was interrupted"
        );
        assert!(
            eventually_dead(&git_pid),
            "git {git_pid} is alive after alz was interrupted"
        );
    }

    // --- No controlling terminal -------------------------------------------------------------

    /// A fake `ssh` that tries to open `/dev/tty` and, if it can, writes `$ALZ_PROBE_MARKER`
    /// there. It records what happened in `$ALZ_PROBE_RESULT` and fails.
    const TTY_PROBE_SSH: &str = r#"#!/bin/sh
if ( : > /dev/tty ) 2> "$ALZ_PROBE_RESULT.error"; then
  echo opened > "$ALZ_PROBE_RESULT"
  printf '%s\n' "$ALZ_PROBE_MARKER" > /dev/tty
else
  echo refused > "$ALZ_PROBE_RESULT"
fi
exit 1
"#;

    const TTY_PROBE_TEST: &str = "tests::the_terminal_probe_inside_a_pseudo_terminal";

    /// Reads the probe directory from this process's environment.
    fn probe_dir() -> OsString {
        EnvSnapshot::capture()
            .vars()
            .get("ALZ_TTY_PROBE_DIR")
            .expect("run by a_fake_ssh_cannot_open_the_terminal_git_runs_under")
            .to_owned()
    }

    /// Runs inside a pseudo-terminal, started by the test below. First a control: the fake `ssh`
    /// started from here finds the terminal and writes to it. Then the same fake `ssh`, started
    /// by git through the runner, cannot open `/dev/tty` and writes nothing to it.
    #[test]
    #[ignore = "runs only inside the pseudo-terminal that a_fake_ssh_cannot_open_the_terminal_git_runs_under starts"]
    fn the_terminal_probe_inside_a_pseudo_terminal() {
        let dir = probe_dir();
        let dir = Path::new(&dir);
        let ssh = dir.join("ssh").into_os_string();

        let control = Command::new(&ssh)
            .env("ALZ_PROBE_RESULT", dir.join("control"))
            .env("ALZ_PROBE_MARKER", "CONTROL-WROTE-TO-THE-TERMINAL")
            .output()
            .expect("the fake ssh runs");
        assert_eq!(control.status.code(), Some(1));

        let home = dir.as_os_str().to_owned();
        let path = real_path();
        let result = dir.join("detached").into_os_string();
        let env = snapshot(&[
            ("PATH", &path),
            ("HOME", &home),
            ("GIT_SSH_COMMAND", &ssh),
            ("ALZ_PROBE_RESULT", &result),
            ("ALZ_PROBE_MARKER", OsStr::new("GIT-WROTE-TO-THE-TERMINAL")),
        ]);
        let git = GitRunner::with_launcher(launcher(), &env, Deadlines::STANDARD).expect("a git");
        let failure = git
            .clone_source(
                CloneTransport::Remote,
                OsStr::new("ssh://example.invalid/layer.git"),
                &dir.join("clone"),
            )
            .unwrap_err();
        assert!(matches!(failure, GitFailure::Exited { .. }), "{failure:?}");
    }

    #[test]
    fn a_fake_ssh_cannot_open_the_terminal_git_runs_under() {
        let dir = scratch();
        dir.write_executable("ssh", TTY_PROBE_SSH).unwrap();
        let executable = test_executable().expect("the test executable");
        let test_arguments = ["--exact", TTY_PROBE_TEST, "--ignored", "--nocapture"];
        let mut script = Command::new("script");
        if cfg!(target_os = "macos") {
            script
                .args(["-q", "/dev/null"])
                .arg(&executable)
                .args(test_arguments);
        } else {
            let command = std::iter::once(executable.to_string_lossy().into_owned())
                .chain(test_arguments.iter().map(|argument| (*argument).to_owned()))
                .map(|word| format!("'{word}'"))
                .collect::<Vec<_>>()
                .join(" ");
            script.args(["-q", "-e", "-c", &command, "/dev/null"]);
        }
        let output = script
            .env("ALZ_TTY_PROBE_DIR", dir.path())
            .env("HOME", dir.path())
            .env("PATH", real_path())
            .timeout(Duration::from_secs(60))
            .output()
            .expect("script(1) runs the probe under a pseudo-terminal");
        let terminal = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "the probe failed:\n{terminal}");
        // The control shows that the test would see a leak: with a terminal, the fake ssh opens
        // it, and what it writes comes out of the pseudo-terminal.
        assert_eq!(dir.read("control").unwrap(), b"opened\n");
        assert!(
            terminal.contains("CONTROL-WROTE-TO-THE-TERMINAL"),
            "{terminal}"
        );
        // Started by git through the runner, the same ssh cannot open /dev/tty and writes
        // nothing to the pseudo-terminal.
        assert_eq!(
            dir.read("detached").expect("ssh ran under git"),
            b"refused\n"
        );
        let error = String::from_utf8_lossy(&dir.read("detached.error").unwrap()).into_owned();
        assert!(!error.trim().is_empty(), "the open failed with a message");
        assert!(
            !terminal.contains("GIT-WROTE-TO-THE-TERMINAL"),
            "{terminal}"
        );
    }

    // --- Real git ----------------------------------------------------------------------------

    fn git_in(directory: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(directory)
            .env_clear()
            .env("PATH", real_path())
            .env("HOME", directory)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args([
                "-c",
                "init.defaultBranch=main",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(output.status.success(), "git {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    #[test]
    fn real_git_clones_resolves_checks_out_lists_and_reads_objects() {
        let dir = scratch();
        let source = dir.join("source");
        dir.make_dir("source").unwrap();
        dir.make_dir("source/layer").unwrap();
        dir.write("source/layer/a.md", "hello\n").unwrap();
        dir.write("source/layer/-b", "dash\n").unwrap();
        git_in(&source, &["init", "-q"]);
        git_in(&source, &["add", "-A"]);
        git_in(&source, &["commit", "-q", "-m", "first"]);
        let commit = id(&git_in(&source, &["rev-parse", "HEAD"]));
        let tree = id(&git_in(&source, &["rev-parse", "HEAD^{tree}"]));
        let blob = id(&git_in(&source, &["rev-parse", "HEAD:layer/a.md"]));
        let missing = id("0123456789abcdef0123456789abcdef01234567");

        let home = dir.path().as_os_str().to_owned();
        let path = real_path();
        let git = GitRunner::with_launcher(
            launcher(),
            &snapshot(&[("PATH", &path), ("HOME", &home)]),
            Deadlines::STANDARD,
        )
        .expect("the real git is 2.30 or later");

        let clone = dir.join("clone");
        git.clone_source(CloneTransport::Local, source.as_os_str(), &clone)
            .expect("a clone");
        let main = GitRef::try_from("main").unwrap();
        assert_eq!(
            git.rev_parse_commit(&clone, ReadScope::CacheEntry, &main)
                .unwrap(),
            Some(commit)
        );
        let unknown = GitRef::try_from("no-such-ref").unwrap();
        assert_eq!(
            git.rev_parse_commit(&clone, ReadScope::CacheEntry, &unknown)
                .unwrap(),
            None
        );
        git.checkout(&clone, &commit).expect("a detached checkout");

        let layer = SafeRelPath::try_from("layer").unwrap();
        let at = git.ls_tree_at(&clone, &commit, &layer).unwrap();
        assert_eq!(at.len(), 1);
        assert_eq!(at[0].mode, EntryMode::Directory);
        assert_eq!(at[0].path, b"layer");

        let all = git.ls_tree_recursive(&clone, &tree).unwrap();
        let listed: Vec<&[u8]> = all.iter().map(|entry| entry.path.as_slice()).collect();
        assert_eq!(listed, [&b"layer/-b"[..], b"layer/a.md"]);
        assert!(
            all.iter()
                .all(|entry| entry.mode == EntryMode::File { executable: false })
        );

        assert_eq!(
            git.cat_file_batch_check(&clone, &[blob, missing]).unwrap(),
            [
                ObjectCheck::Present {
                    id: blob,
                    kind: ObjectKind::Blob,
                    size: 6
                },
                ObjectCheck::Missing { id: missing }
            ]
        );
        assert_eq!(
            git.cat_file_batch(&clone, &[blob]).unwrap(),
            [ObjectContent::Present {
                id: blob,
                kind: ObjectKind::Blob,
                bytes: b"hello\n".to_vec()
            }]
        );

        let toplevel = git.show_toplevel(&source.join("layer")).unwrap();
        assert_eq!(toplevel.file_name(), source.file_name());
        assert_eq!(
            git.show_prefix(&source.join("layer")).unwrap(),
            OsString::from("layer/")
        );
    }

    /// A repository with one commit, in `<dir>/source`.
    fn committed_source(dir: &ScratchDir) -> PathBuf {
        let source = dir.join("source");
        dir.make_dir("source").unwrap();
        dir.write("source/a.md", "hello\n").unwrap();
        git_in(&source, &["init", "-q"]);
        git_in(&source, &["add", "-A"]);
        git_in(&source, &["commit", "-q", "-m", "first"]);
        source
    }

    #[test]
    fn a_local_clone_copies_the_objects_and_never_hard_links_them_to_the_source() {
        let dir = scratch();
        let source = committed_source(&dir);
        let home = dir.path().as_os_str().to_owned();
        let path = real_path();
        let git = GitRunner::with_launcher(
            launcher(),
            &snapshot(&[("PATH", &path), ("HOME", &home)]),
            Deadlines::STANDARD,
        )
        .expect("a git");
        let clone = dir.join("clone");
        git.clone_source(CloneTransport::Local, source.as_os_str(), &clone)
            .expect("a clone");
        let linked = Command::new("find")
            .arg(clone.join(".git/objects"))
            .args(["-type", "f", "-links", "+1"])
            .output()
            .expect("find runs");
        assert!(linked.status.success(), "{linked:?}");
        assert_eq!(String::from_utf8_lossy(&linked.stdout).trim(), "");
    }

    #[test]
    fn a_remote_clone_runs_no_transport_helper_even_when_the_users_configuration_allows_it() {
        let dir = scratch();
        // The user's own git configuration lets the `ext` helper run a command.
        dir.write(".gitconfig", "[protocol \"ext\"]\n\tallow = always\n")
            .unwrap();
        let marker = dir.join("helper-ran");
        let url = format!("ext::sh -c touch% {}", marker.display());
        let home = dir.path().as_os_str().to_owned();
        let path = real_path();
        let git = GitRunner::with_launcher(
            launcher(),
            &snapshot(&[("PATH", &path), ("HOME", &home)]),
            Deadlines::STANDARD,
        )
        .expect("a git");
        let failure = git
            .clone_source(CloneTransport::Remote, OsStr::new(&url), &dir.join("clone"))
            .unwrap_err();
        assert!(matches!(failure, GitFailure::Exited { .. }), "{failure:?}");
        assert!(!dir.exists("helper-ran"), "the helper ran");
    }

    #[test]
    fn a_remote_clone_never_widens_the_users_allowed_protocols_to_a_transport_helper() {
        let dir = scratch();
        let marker = dir.join("helper-ran");
        let url = format!("ext::sh -c touch% {}", marker.display());
        let home = dir.path().as_os_str().to_owned();
        let path = real_path();
        let allowed = OsString::from("ext:file:https");
        let git = GitRunner::with_launcher(
            launcher(),
            &snapshot(&[
                ("PATH", &path),
                ("HOME", &home),
                ("GIT_ALLOW_PROTOCOL", &allowed),
            ]),
            Deadlines::STANDARD,
        )
        .expect("a git");
        git.clone_source(CloneTransport::Remote, OsStr::new(&url), &dir.join("clone"))
            .unwrap_err();
        assert!(!dir.exists("helper-ran"), "the helper ran");
    }

    #[test]
    fn a_failed_git_reports_its_status_and_its_standard_error_without_printing_either() {
        let dir = scratch();
        let home = dir.path().as_os_str().to_owned();
        let path = real_path();
        let git = GitRunner::with_launcher(
            launcher(),
            &snapshot(&[("PATH", &path), ("HOME", &home)]),
            Deadlines::STANDARD,
        )
        .expect("a git");
        let failure = git
            .clone_source(
                CloneTransport::Local,
                dir.join("no-such-repository").as_os_str(),
                &dir.join("clone"),
            )
            .unwrap_err();
        match failure {
            GitFailure::Exited { status, stderr } => {
                assert_eq!(status.code(), Some(128));
                assert!(stderr.len() > 1);
            }
            GitFailure::Spawn(_)
            | GitFailure::Io(_)
            | GitFailure::TimedOut { .. }
            | GitFailure::OutputTooLarge
            | GitFailure::Unreadable(_) => panic!("expected an exit status"),
        }
    }
}
