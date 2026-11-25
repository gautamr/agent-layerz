//! `alz account allow` and `--remove` (spec §7.13 "Trust and `alz account allow`", §10.3,
//! TASK-55.7.13), driven through a pseudo-terminal with the `pty-run` driver of `alz-test-support`.
//!
//! Every run has its own `HOME` and no `XDG_*` variable, so the allow state lives in the scratch
//! folder and never in the real one. `--remove` and the refusals need no stack, so those cases
//! record an entry for the repository by hand; the cases that offer a declaration of a layer, with
//! its cache and lock, build `fixtures/32-account-allow` (a client layer pinned at a tag) first and
//! then answer the prompts through the pseudo-terminal.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
#[expect(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    reason = "the test drives the binary as a child under the pty driver, as alz-io's terminal test does"
)]
mod tests {
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use alz_core::account::AgentSessionMarker;
    use alz_test_support::{ScratchDir, copy_tree, pty_run_executable, read_tree};
    use assert_cmd::cargo::cargo_bin_cmd;

    const PATIENCE: Duration = Duration::from_secs(30);
    const HEADER: &str =
        "# written by alz account allow; deleting the file withdraws every approval\n";

    /// A repository in an isolated world, with an entry for it in `allowed.yaml`.
    struct World {
        scratch: ScratchDir,
        root: PathBuf,
    }

    struct Finished {
        code: Option<i32>,
        stdout: String,
        stderr: String,
    }

    impl World {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            for folder in ["home", "repo/.layer"] {
                scratch.make_dirs(folder).expect("made");
            }
            scratch
                .write(
                    "repo/layer.yaml",
                    "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
                )
                .expect("written");
            scratch
                .write(
                    "repo/.layer/layer.yaml",
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                )
                .expect("written");
            let root = scratch
                .real_path()
                .expect("the real path of the scratch folder")
                .join("repo");
            Self { scratch, root }
        }

        /// `fixtures/32-account-allow` in a scratch folder: the repository `repo` with its client
        /// layer `hub/layer`, pinned at the tag `v1` of the repository `hub`.
        fn pinned() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let fixture =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/32-account-allow");
            copy_tree(&fixture, &scratch.join("work")).expect("the fixture is copied");
            scratch.make_dir("home").expect("made");
            let root = scratch
                .real_path()
                .expect("the real path of the scratch folder")
                .join("work/repo");
            let world = Self { scratch, root };
            world.hub_vcs(&["init", "--quiet", "--initial-branch=main"]);
            world.commit_hub();
            world
        }

        fn hub_vcs(&self, args: &[&str]) {
            let status = Command::new("git")
                .arg("-C")
                .arg(self.scratch.join("work/hub"))
                .args(args)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"))
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@example.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@example.com")
                .stdout(Stdio::null())
                .status()
                .expect("the version control tool runs");
            assert!(status.success(), "{args:?}");
        }

        /// Commits the hub as it is and moves the tag `v1` to the commit.
        fn commit_hub(&self) {
            self.hub_vcs(&["add", "-A"]);
            self.hub_vcs(&["commit", "--quiet", "--message", "hub"]);
            self.hub_vcs(&["tag", "--force", "v1"]);
        }

        /// `alz` with standard input closed and no terminal, as a script runs it.
        fn plain(&self, args: &[&str]) -> Finished {
            let alz = Command::new(cargo_bin_cmd!("alz").get_program());
            let output = self
                .command(alz, &[], args)
                .env("PATH", "/usr/bin:/bin")
                .stdin(Stdio::null())
                .output()
                .expect("alz runs");
            Finished {
                code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }

        const STATE_FILE: &'static str = "home/.local/state/agent-layerz/allowed.yaml";

        /// Records the entry of this repository, private as `alz account allow` writes it.
        fn allowed(&self, claude: &str, codex: &str) -> String {
            let text = format!(
                "{HEADER}- root: {}\n  account:\n    claude: {claude}\n    codex: {codex}\n  source: /layers/client\n",
                self.root.display()
            );
            self.scratch
                .make_dirs("home/.local/state/agent-layerz")
                .expect("made");
            for folder in [
                "home/.local",
                "home/.local/state",
                "home/.local/state/agent-layerz",
            ] {
                self.scratch.set_mode(folder, 0o700).expect("private");
            }
            self.scratch
                .write(Self::STATE_FILE, &text)
                .expect("written");
            self.scratch
                .set_mode(Self::STATE_FILE, 0o600)
                .expect("private");
            text
        }

        fn command(&self, program: Command, markers: &[(&str, &str)], args: &[&str]) -> Command {
            let mut command = program;
            command
                .args(args)
                .current_dir(&self.root)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", self.scratch.join("home"));
            command.envs(markers.iter().copied());
            command
        }

        /// `alz` under a pseudo-terminal, as a person's terminal.
        fn on_terminal(&self, markers: &[(&str, &str)], args: &[&str]) -> Running {
            let alz = cargo_bin_cmd!("alz");
            let mut command = Command::new(pty_run_executable().expect("the driver is built"));
            command
                .args(["--deadline-ms", "60000", "--"])
                .arg(alz.get_program());
            Running::start(self.command(command, markers, args))
        }

        /// `alz` with standard input on a pipe: no terminal.
        fn on_pipe(&self, args: &[&str]) -> Running {
            let alz = Command::new(cargo_bin_cmd!("alz").get_program());
            Running::start(self.command(alz, &[], args))
        }
    }

    /// The child running, with its streams read on threads.
    struct Running {
        process: Child,
        input: Option<ChildStdin>,
        stderr: Receiver<Vec<u8>>,
        seen: String,
        stdout: JoinHandle<String>,
    }

    impl Running {
        fn start(mut command: Command) -> Self {
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut process = command.spawn().expect("the child starts");
            let input = process.stdin.take();
            let mut stdout = process.stdout.take().expect("stdout is piped");
            let mut stderr = process.stderr.take().expect("stderr is piped");
            let (chunks, received) = mpsc::channel();
            thread::spawn(move || {
                let mut chunk = [0_u8; 4096];
                while let Ok(read) = stderr.read(&mut chunk) {
                    let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                        break;
                    };
                    if chunks.send(bytes.to_vec()).is_err() {
                        break;
                    }
                }
            });
            Self {
                process,
                input,
                stderr: received,
                seen: String::new(),
                stdout: thread::spawn(move || {
                    let mut all = String::new();
                    stdout.read_to_string(&mut all).expect("text");
                    all
                }),
            }
        }

        fn wait_for_stderr(&mut self, needle: &str) {
            while !self.seen.contains(needle) {
                let chunk = self
                    .stderr
                    .recv_timeout(PATIENCE)
                    .unwrap_or_else(|_| panic!("no {needle:?} on stderr: {:?}", self.seen));
                self.seen.push_str(&String::from_utf8_lossy(&chunk));
            }
        }

        fn type_in(&mut self, bytes: &[u8]) {
            let input = self.input.as_mut().expect("the input is open");
            input.write_all(bytes).expect("the driver reads");
            input.flush().expect("flushed");
        }

        /// Offers input to a program that may refuse before it reads anything: a pipe that the
        /// program has already closed is the expected end, any other failure is a bug of the test.
        fn offer(&mut self, bytes: &[u8]) {
            let input = self.input.as_mut().expect("the input is open");
            let written = input.write_all(bytes).and_then(|()| input.flush());
            match written {
                Err(error) if error.kind() != std::io::ErrorKind::BrokenPipe => {
                    panic!("the input is written: {error}")
                }
                Ok(()) | Err(_) => {}
            }
        }

        /// Closes what the driver reads: it types the terminal's end-of-file character.
        fn end_input(&mut self) {
            self.input = None;
        }

        fn finish(mut self) -> Finished {
            self.input = None;
            let code = self.process.wait().expect("the child ends").code();
            let rest: Vec<Vec<u8>> = self.stderr.iter().collect();
            for chunk in rest {
                self.seen.push_str(&String::from_utf8_lossy(&chunk));
            }
            Finished {
                code,
                stdout: self.stdout.join().expect("the reader ends"),
                stderr: self.seen,
            }
        }
    }

    fn prompt(name: &str, world: &World) -> String {
        format!(
            "Type {name} to remove it for \"{}\" (source \"/layers/client\"): ",
            world.root.display()
        )
    }

    #[test]
    fn remove_asks_for_each_recorded_name_and_removes_the_entry_after_both_answers() {
        let world = World::new();
        world.allowed("client-a", "client-a-api");
        let mut run = world.on_terminal(&[], &["account", "allow", "--remove"]);
        run.wait_for_stderr(&prompt("client-a", &world));
        run.type_in(b" client-a\t\n");
        run.wait_for_stderr(&prompt("client-a-api", &world));
        run.type_in(b"client-a-api\n");
        let finished = run.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished.stdout.contains(
                "allowed now: claude=client-a, codex=client-a-api from \"/layers/client\""
            ) && finished.stdout.contains("removed the entry for"),
            "{}",
            finished.stdout
        );
        assert_eq!(
            world.scratch.read_text(World::STATE_FILE).expect("read"),
            format!("{HEADER}[]\n")
        );
    }

    #[test]
    fn a_wrong_answer_ctrl_c_and_the_end_of_input_each_end_not_confirmed_and_change_nothing() {
        for (label, answer) in [
            ("a wrong name", Some(&b"client-b\n"[..])),
            ("y", Some(&b"y\n"[..])),
            ("the name and then Ctrl-D", Some(&b"client-a\x04"[..])),
            ("Ctrl-C", Some(&b"\x03"[..])),
            ("the end of input", None),
        ] {
            let world = World::new();
            let before = world.allowed("client-a", "client-a");
            let mut run = world.on_terminal(&[], &["account", "allow", "--remove"]);
            run.wait_for_stderr("Type client-a to remove it");
            match answer {
                Some(bytes) => run.type_in(bytes),
                None => run.end_input(),
            }
            let finished = run.finish();
            assert_eq!(finished.code, Some(1), "{label}: {}", finished.stderr);
            assert!(
                finished.stderr.contains("not confirmed; nothing changed"),
                "{label}: {}",
                finished.stderr
            );
            assert_eq!(
                world.scratch.read_text(World::STATE_FILE).expect("read"),
                before,
                "{label}"
            );
        }
    }

    #[test]
    fn two_equal_names_are_asked_once() {
        let world = World::new();
        world.allowed("client-a", "client-a");
        let mut run = world.on_terminal(&[], &["account", "allow", "--remove"]);
        run.wait_for_stderr(&prompt("client-a", &world));
        run.type_in(b"client-a\n");
        let finished = run.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert_eq!(finished.stderr.matches("Type client-a").count(), 1);
    }

    #[test]
    fn with_no_entry_it_says_nothing_to_remove_creates_nothing_and_asks_nothing() {
        let world = World::new();
        let before = read_tree(world.scratch.path()).expect("tree");
        let finished = world
            .on_terminal(&[], &["account", "allow", "--remove"])
            .finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(finished.stdout.contains("nothing to remove: "));
        assert!(!finished.stderr.contains("Type "));
        assert_eq!(read_tree(world.scratch.path()).expect("tree"), before);
    }

    #[test]
    fn a_pipe_on_standard_input_is_refused_before_anything_is_asked_or_written() {
        for args in [
            &["account", "allow"][..],
            &["account", "allow", "--remove"][..],
        ] {
            let world = World::new();
            let before = read_tree(world.scratch.path()).expect("tree");
            let mut run = world.on_pipe(args);
            run.offer(b"client-a\nclient-a\n");
            let finished = run.finish();
            assert_eq!(finished.code, Some(1), "{}", finished.stderr);
            assert!(
                finished
                    .stderr
                    .contains("error [account-allow] alz account allow")
                    && finished.stderr.contains(
                        "must be run by a person at a terminal: standard input is not a terminal"
                    )
                    && finished.stderr.ends_with("nothing was changed\n"),
                "{}",
                finished.stderr
            );
            assert_eq!(finished.stdout, "");
            assert_eq!(read_tree(world.scratch.path()).expect("tree"), before);
        }
    }

    #[test]
    fn each_agent_session_marker_is_refused_on_a_terminal_by_name_and_never_by_value() {
        for marker in AgentSessionMarker::ALL {
            for value in ["", "secret-value-4f1c9"] {
                let world = World::new();
                let before = world.allowed("client-a", "client-a");
                let run =
                    world.on_terminal(&[(marker.name(), value)], &["account", "allow", "--remove"]);
                let finished = run.finish();
                assert_eq!(finished.code, Some(1), "{}", finished.stderr);
                assert!(
                    finished.stderr.contains(&format!(
                        "an agent session is running here ({} set)",
                        marker.name()
                    )),
                    "{}",
                    finished.stderr
                );
                assert!(!finished.stderr.contains("standard input is not a terminal"));
                assert!(!finished.stderr.contains("secret-value-4f1c9"));
                assert!(!finished.stderr.contains("Type "));
                let ide = "Claude Code also sets CLAUDECODE in an IDE's integrated terminal";
                assert_eq!(
                    finished.stderr.contains(ide),
                    marker == AgentSessionMarker::ClaudeCode
                );
                assert_eq!(
                    world.scratch.read_text(World::STATE_FILE).expect("read"),
                    before
                );
            }
        }
    }

    #[test]
    fn a_state_that_alz_refuses_is_named_before_any_question_and_nothing_is_written() {
        let world = World::new();
        let before = world.allowed("client-a", "client-a");
        world
            .scratch
            .set_mode(World::STATE_FILE, 0o660)
            .expect("group-writable");
        let tree = read_tree(world.scratch.path()).expect("tree");
        for args in [
            &["account", "allow"][..],
            &["account", "allow", "--remove"][..],
        ] {
            let finished = world.on_terminal(&[], args).finish();
            assert_eq!(finished.code, Some(1), "{}", finished.stderr);
            assert!(
                finished.stderr.contains("group or others can write it")
                    && finished.stderr.contains("nothing was changed")
                    && !finished.stderr.contains("Type "),
                "{}",
                finished.stderr
            );
        }
        assert_eq!(read_tree(world.scratch.path()).expect("tree"), tree);
        assert_eq!(
            world.scratch.read_text(World::STATE_FILE).expect("read"),
            before
        );
    }

    #[test]
    fn there_is_no_way_to_answer_for_the_person() {
        for argument in ["--yes", "-y", "--force", "extra"] {
            let world = World::new();
            let finished = world
                .on_terminal(&[], &["account", "allow", argument])
                .finish();
            assert_eq!(finished.code, Some(2), "{}", finished.stderr);
            assert_eq!(finished.stdout, "");
            assert_ne!(finished.stderr, "");
        }
    }

    /// Spec §7.13 "Trust and `alz account allow`" and "Resolution": a layer's declaration, offered
    /// from the layers of the repository (fixture 32, a client layer pinned at a tag), is applied
    /// only after a person confirms it by typing each account name; the entry that is recorded
    /// is the real root, the accepted map and the layer's real path; a second run says
    /// `already allowed` without a question; and `alz account which` then reports the `layer` step.
    #[test]
    fn a_declared_account_applies_only_after_the_person_types_its_names_and_is_then_recorded() {
        let world = World::pinned();
        for folder in [".claude-client-a", ".codex-client-a-api"] {
            world
                .scratch
                .make_dir(format!("home/{folder}"))
                .expect("made");
        }
        let built = world.plain(&["build"]);
        assert_eq!(built.code, Some(0), "{}", built.stderr);
        let before = world.plain(&["account", "which"]);
        assert_eq!(before.code, Some(0), "{}", before.stderr);
        assert!(
            before.stdout.contains("claude: default (source: default)")
                && before
                    .stdout
                    .contains("layer step skipped (no allowed.yaml)"),
            "{}",
            before.stdout
        );

        let mut run = world.on_terminal(&[], &["account", "allow"]);
        run.wait_for_stderr("Type client-a ");
        run.type_in(b"client-a\n");
        run.wait_for_stderr("Type client-a-api ");
        run.type_in(b"client-a-api\n");
        let finished = run.finish();
        assert_eq!(
            finished.code,
            Some(0),
            "{}{}",
            finished.stdout,
            finished.stderr
        );
        let hub_layer = world
            .scratch
            .real_path()
            .expect("real")
            .join("work/hub/layer");
        assert_eq!(
            world.scratch.read_text(World::STATE_FILE).expect("read"),
            format!(
                "{HEADER}- root: {}\n  account:\n    claude: client-a\n    codex: client-a-api\n  source: {}\n",
                world.root.display(),
                hub_layer.display()
            ),
            "{}",
            finished.stdout
        );

        let which = world.plain(&["account", "which"]);
        assert_eq!(which.code, Some(0), "{}", which.stderr);
        assert!(
            which.stdout.contains("claude: client-a (source: layer)")
                && which.stdout.contains("codex: client-a-api (source: layer)"),
            "{}",
            which.stdout
        );

        let again = world.on_terminal(&[], &["account", "allow"]).finish();
        assert_eq!(again.code, Some(0), "{}", again.stderr);
        assert!(again.stdout.contains("already allowed"), "{}", again.stdout);
        assert!(!again.stderr.contains("Type "), "{}", again.stderr);
    }

    /// Spec §7.13 "A named account that cannot apply fails closed" and "Trust": once the layer's
    /// declaration changes, an allowed repository never gets the new account and never the default
    /// one; `alz account which` fails closed and names `alz account allow` as the fix, and
    /// nothing but `alz account allow` changes the allow state.
    #[test]
    fn a_changed_declaration_of_an_allowed_repository_fails_closed_and_asks_for_a_new_approval() {
        let world = World::pinned();
        for folder in [".claude-client-a", ".codex-client-a-api"] {
            world
                .scratch
                .make_dir(format!("home/{folder}"))
                .expect("made");
        }
        assert_eq!(world.plain(&["build"]).code, Some(0));
        let mut run = world.on_terminal(&[], &["account", "allow"]);
        run.wait_for_stderr("Type client-a ");
        run.type_in(b"client-a\n");
        run.wait_for_stderr("Type client-a-api ");
        run.type_in(b"client-a-api\n");
        assert_eq!(run.finish().code, Some(0));
        let state = world.scratch.read_text(World::STATE_FILE).expect("read");

        world
            .scratch
            .write(
                "work/hub/layer/layer.yaml",
                "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\naccount:\n  claude: client-b\n  codex: client-a-api\n",
            )
            .expect("written");
        world.commit_hub();
        assert_eq!(world.plain(&["build"]).code, Some(0));
        let which = world.plain(&["account", "which"]);
        assert_eq!(which.code, Some(1), "{}{}", which.stdout, which.stderr);
        assert!(
            which.stdout.contains("claude: fails closed")
                && which.stdout.contains("alz account allow"),
            "{}",
            which.stdout
        );
        assert!(
            !which.stdout.contains("claude: default") && !which.stdout.contains("client-b ("),
            "{}",
            which.stdout
        );
        assert_eq!(
            world.scratch.read_text(World::STATE_FILE).expect("read"),
            state,
            "nothing but alz account allow changes the state"
        );
    }

    /// The pinned world after `alz build` and a typed approval of the declaration: the state of
    /// a person who cloned the repository and ran `alz account allow`. With `Some(text)` the
    /// layer's `layer.yaml` is that text first, and its map names only `claude`.
    fn approved(layer_yaml: Option<&str>) -> World {
        let world = World::pinned();
        for folder in [".claude-client-a", ".codex-client-a-api"] {
            world
                .scratch
                .make_dir(format!("home/{folder}"))
                .expect("made");
        }
        if let Some(text) = layer_yaml {
            world
                .scratch
                .write("work/hub/layer/layer.yaml", text)
                .expect("written");
            world.commit_hub();
        }
        assert_eq!(world.plain(&["build"]).code, Some(0));
        let mut run = world.on_terminal(&[], &["account", "allow"]);
        run.wait_for_stderr("Type client-a ");
        run.type_in(b"client-a\n");
        if layer_yaml.is_none() {
            run.wait_for_stderr("Type client-a-api ");
            run.type_in(b"client-a-api\n");
        }
        assert_eq!(run.finish().code, Some(0));
        world
    }

    /// Spec §13 item 20: in a fresh clone the tools start on the default account, and after
    /// `allow` the named account applies; a ref bump of the declaring layer keeps it, because
    /// the account is what is approved and not the layer's revision (§7.13).
    #[test]
    fn a_ref_bump_of_the_declaring_layer_keeps_the_approved_account() {
        let world = approved(None);
        let state = world.scratch.read_text(World::STATE_FILE).expect("read");
        world
            .scratch
            .write(
                "work/hub/layer/instructions/20-more.md",
                "# More\n\nText.\n",
            )
            .expect("written");
        world.hub_vcs(&["add", "-A"]);
        world.hub_vcs(&["commit", "--quiet", "--message", "more"]);
        world.hub_vcs(&["tag", "v2"]);
        let manifest = world
            .scratch
            .read_text("work/repo/layer.yaml")
            .expect("read");
        world
            .scratch
            .write(
                "work/repo/layer.yaml",
                manifest.replace("ref: v1", "ref: v2"),
            )
            .expect("written");
        let built = world.plain(&["build"]);
        assert_eq!(built.code, Some(0), "{}", built.stderr);
        let which = world.plain(&["account", "which"]);
        assert_eq!(which.code, Some(0), "{}{}", which.stdout, which.stderr);
        assert!(
            which.stdout.contains("claude: client-a (source: layer)")
                && which.stdout.contains("codex: client-a-api (source: layer)"),
            "{}",
            which.stdout
        );
        assert_eq!(
            world.scratch.read_text(World::STATE_FILE).expect("read"),
            state
        );
    }

    const CACHE: &str = "home/.cache/agent-layerz";

    /// How a test spoils what `alz build` left behind.
    type Tamper = fn(&World);

    /// The one index entry of the pinned layer, as a name below the cache.
    fn index_entry(world: &World) -> String {
        world
            .scratch
            .list(CACHE)
            .into_iter()
            .find(|name| name.starts_with("pathref@"))
            .expect("an index entry")
    }

    /// `layer.lock` with each line that `change` returns a new text for replaced. `change` gets
    /// the line and the line after it; the lock is pretty JSON with one key on each line.
    fn rewrite_lock(world: &World, change: impl Fn(&str, Option<&str>) -> Option<String>) {
        let lock = world
            .scratch
            .read_text("work/repo/layer.lock")
            .expect("read");
        let lines: Vec<&str> = lock.lines().collect();
        let rewritten = lines
            .iter()
            .enumerate()
            .map(|(at, line)| {
                change(line, lines.get(at + 1).copied()).unwrap_or_else(|| (*line).to_owned())
            })
            .collect::<Vec<_>>()
            .join("\n");
        world
            .scratch
            .write("work/repo/layer.lock", format!("{rewritten}\n"))
            .expect("written");
    }

    fn spoil_lock_sha(world: &World) {
        rewrite_lock(world, |line, _| {
            line.trim_start()
                .starts_with("\"sha\": ")
                .then(|| format!("      \"sha\": \"{}\",", "z".repeat(40)))
        });
    }

    fn spoil_lock_content_hash(world: &World) {
        rewrite_lock(world, |line, next| {
            (line.trim_start().starts_with("\"contentHash\": ")
                && next.is_some_and(|next| next.contains("\"name\": \"client\"")))
            .then(|| format!("      \"contentHash\": \"sha256:{}\",", "0".repeat(64)))
        });
    }

    fn move_index_to_another_path(world: &World) {
        let name = index_entry(world);
        let (_, sha) = name.rsplit_once('-').expect("a sha");
        world
            .scratch
            .rename(
                format!("{CACHE}/{name}"),
                format!("{CACHE}/pathref@{}-{sha}", "0".repeat(64)),
            )
            .expect("renamed");
    }

    fn malform_index(world: &World) {
        let name = index_entry(world);
        world
            .scratch
            .write(format!("{CACHE}/{name}"), "not an id\n")
            .expect("written");
    }

    fn link_index(world: &World) {
        let name = index_entry(world);
        let content = world
            .scratch
            .read_text(format!("{CACHE}/{name}"))
            .expect("read");
        world.scratch.write("elsewhere", content).expect("written");
        world
            .scratch
            .remove_file(format!("{CACHE}/{name}"))
            .expect("removed");
        world
            .scratch
            .make_link(format!("{CACHE}/{name}"), world.scratch.join("elsewhere"))
            .expect("linked");
    }

    /// Spec §13 item 20: a pinned `path:` layer resolves through the cache index; a lock `sha`
    /// that is not hex, an index entry for another path, a malformed index entry, a symlinked
    /// index entry and a layer whose content hash differs from the lock are each a cache miss
    /// through `alz account which`, and the hint says only to run `alz build`.
    #[test]
    fn each_cache_miss_of_a_pinned_layer_fails_closed_and_the_hint_says_only_to_run_build() {
        let cases: [(&str, Tamper, &str); 5] = [
            (
                "a lock sha that is not hex",
                spoil_lock_sha,
                "layer.lock cannot be read",
            ),
            (
                "an index entry for another path",
                move_index_to_another_path,
                "the cache index entry is missing",
            ),
            (
                "a malformed index entry",
                malform_index,
                "the cache index entry is not valid",
            ),
            (
                "a symlinked index entry",
                link_index,
                "the cache index entry is not valid",
            ),
            (
                "a content hash that differs from the lock",
                spoil_lock_content_hash,
                "a layer no longer matches layer.lock",
            ),
        ];
        for (what, tamper, reason) in cases {
            let world = approved(None);
            tamper(&world);
            let which = world.plain(&["account", "which"]);
            assert_eq!(which.code, Some(1), "{what}: {}", which.stdout);
            for target in ["claude", "codex"] {
                assert!(
                    which.stdout.contains(&format!("{target}: fails closed")),
                    "{what}: {}",
                    which.stdout
                );
            }
            assert!(
                which.stdout.contains(&format!(
                    "layer step skipped (cache miss: {reason}); fails closed"
                )),
                "{what}: {}",
                which.stdout
            );
            assert!(
                which.stdout.contains("fix: alz build")
                    && !which.stdout.contains("fix: alz account allow"),
                "{what}: {}",
                which.stdout
            );
        }
    }

    /// Spec §13 item 20: twelve `alz account allow` processes at once, for twelve repositories
    /// that share one client layer, leave one entry per root in sorted order, and
    /// `alz account allow --remove` removes only the entry of the root it runs in.
    #[test]
    fn twelve_concurrent_allows_leave_one_sorted_entry_per_root_and_remove_takes_only_its_own() {
        let world = World::new();
        let base = world.scratch.real_path().expect("a real path");
        world.scratch.make_dir("client").expect("made");
        world
            .scratch
            .write(
                "client/layer.yaml",
                "name: client\nkind: client\nvisibility: private\naccount: client-a\n",
            )
            .expect("written");
        let roots: Vec<PathBuf> = (0..12)
            .map(|number| base.join(format!("repos/repo-{number:02}")))
            .collect();
        for number in 0..12 {
            let folder = format!("repos/repo-{number:02}");
            world
                .scratch
                .make_dirs(format!("{folder}/.layer"))
                .expect("made");
            world
                .scratch
                .write(
                    format!("{folder}/layer.yaml"),
                    "stack:\n  - source: path:../../client\n  - source: path:./.layer\ntargets: [claude]\n",
                )
                .expect("written");
            world
                .scratch
                .write(
                    format!("{folder}/.layer/layer.yaml"),
                    "name: shop-api\nkind: repo\nvisibility: private\n",
                )
                .expect("written");
        }
        for root in &roots {
            let alz = Command::new(cargo_bin_cmd!("alz").get_program());
            let built = world
                .command(alz, &[], &["build"])
                .current_dir(root)
                .stdin(Stdio::null())
                .output()
                .expect("alz runs");
            assert!(built.status.success(), "{:?}", built.stderr);
        }
        let on_terminal_in = |root: &Path, args: &[&str]| -> Running {
            let alz = cargo_bin_cmd!("alz");
            let mut command = Command::new(pty_run_executable().expect("the driver is built"));
            command
                .args(["--deadline-ms", "60000", "--"])
                .arg(alz.get_program());
            let mut command = world.command(command, &[], args);
            command.current_dir(root);
            Running::start(command)
        };
        let mut running: Vec<Running> = roots
            .iter()
            .map(|root| on_terminal_in(root, &["account", "allow"]))
            .collect();
        for run in &mut running {
            run.wait_for_stderr("Type client-a ");
        }
        for run in &mut running {
            run.type_in(b"client-a\n");
        }
        for run in running {
            let finished = run.finish();
            assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        }
        let entries = |text: &str| -> Vec<String> {
            text.lines()
                .filter_map(|line| line.strip_prefix("- root: "))
                .map(str::to_owned)
                .collect()
        };
        let expected: Vec<String> = roots
            .iter()
            .map(|root| root.display().to_string())
            .collect();
        let state = world.scratch.read_text(World::STATE_FILE).expect("read");
        assert_eq!(entries(&state), expected, "{state}");

        let removed = roots.get(5).expect("a sixth root");
        let mut run = on_terminal_in(removed, &["account", "allow", "--remove"]);
        run.wait_for_stderr("Type client-a to remove it for");
        run.type_in(b"client-a\n");
        let finished = run.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        let after = world.scratch.read_text(World::STATE_FILE).expect("read");
        let without: Vec<String> = expected
            .iter()
            .filter(|root| **root != removed.display().to_string())
            .cloned()
            .collect();
        assert_eq!(entries(&after), without, "{after}");
    }

    /// Spec §13 item 24 (TASK-50.11): a declaration made by the repository's own layer is never
    /// offered by `alz account allow`: it is printed as a warning and not asked about, whether
    /// or not an external client layer declares as well, and nothing but the confirmed client
    /// account is written.
    #[test]
    fn the_repository_layers_own_declaration_is_refused_by_allow_and_never_offered() {
        const REPO_DECLARES: &str =
            "name: shop-api\nkind: repo\nvisibility: private\naccount: client-b\n";
        let alone = World::new();
        assert_eq!(alone.plain(&["build"]).code, Some(0));
        alone
            .scratch
            .write("repo/.layer/layer.yaml", REPO_DECLARES)
            .expect("written");
        let finished = alone.on_terminal(&[], &["account", "allow"]).finish();
        assert_eq!(finished.code, Some(1), "{}", finished.stderr);
        assert!(
            finished
                .stderr
                .contains("warning [account-declaration-ignored] the layer \"shop-api\"")
                && finished
                    .stderr
                    .contains("error [account-allow] the layers name no account here")
                && !finished.stderr.contains("Type "),
            "{}",
            finished.stderr
        );
        assert!(
            !alone.scratch.exists(World::STATE_FILE),
            "nothing is offered, so nothing is written"
        );

        let world = World::new();
        world.scratch.make_dir("client").expect("made");
        world
            .scratch
            .write(
                "client/layer.yaml",
                "name: client\nkind: client\nvisibility: private\naccount: client-a\n",
            )
            .expect("written");
        world
            .scratch
            .write(
                "repo/layer.yaml",
                "stack:\n  - source: path:../client\n  - source: path:./.layer\ntargets: [claude]\n",
            )
            .expect("written");
        assert_eq!(world.plain(&["build"]).code, Some(0));
        world
            .scratch
            .write("repo/.layer/layer.yaml", REPO_DECLARES)
            .expect("written");
        let mut run = world.on_terminal(&[], &["account", "allow"]);
        run.wait_for_stderr("Type client-a ");
        run.type_in(b"client-a\n");
        let finished = run.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stderr
                .contains("warning [account-declaration-ignored] the layer \"shop-api\"")
                && !finished.stderr.contains("client-b")
                && !finished.stdout.contains("client-b"),
            "{}",
            finished.stderr
        );
        let state = world.scratch.read_text(World::STATE_FILE).expect("read");
        assert!(
            state.contains("claude: client-a") && !state.contains("client-b"),
            "{state}"
        );
    }

    /// Spec §13 item 24 (TASK-50.11): the source of an external client layer is shown quoted
    /// before the prompt, and a hub folder whose name holds ESC, a zero-width character and a
    /// look-alike letter prints no control character: each is `\u{XXXX}`.
    #[test]
    fn a_hub_folder_name_with_control_characters_is_shown_quoted_and_escaped_by_allow() {
        let world = World::new();
        let hub = "hub\u{1b}[31m\u{200b}\u{430}";
        world.scratch.make_dir(hub).expect("made");
        world
            .scratch
            .write(
                format!("{hub}/layer.yaml"),
                "name: client\nkind: client\nvisibility: private\naccount: client-a\n",
            )
            .expect("written");
        world
            .scratch
            .write(
                "repo/layer.yaml",
                "stack:\n  - source: path:../link\n  - source: path:./.layer\ntargets: [claude]\n",
            )
            .expect("written");
        world
            .scratch
            .make_link("link", world.scratch.join(hub))
            .expect("linked");
        let built = world.plain(&["build"]);
        assert_eq!(built.code, Some(0), "{}", built.stderr);
        let mut run = world.on_terminal(&[], &["account", "allow"]);
        run.wait_for_stderr("Type client-a ");
        run.type_in(b"nope\n");
        let finished = run.finish();
        assert_eq!(finished.code, Some(1), "{}", finished.stderr);
        let real = world.scratch.real_path().expect("a real path");
        let quoted = format!(
            "\"{}/hub\\u{{001B}}[31m\\u{{200B}}\\u{{0430}}\"",
            real.display()
        );
        assert!(
            finished
                .stdout
                .contains(&format!("recorded source: {quoted}")),
            "{}",
            finished.stdout
        );
        assert!(
            finished.stderr.contains(&format!("(source {quoted})")),
            "{}",
            finished.stderr
        );
        let shown = format!("{}{}", finished.stdout, finished.stderr);
        assert!(
            !shown
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r'))
                && !shown.contains(['\u{200b}', '\u{430}']),
            "{shown:?}"
        );
        assert!(
            !world.scratch.exists(World::STATE_FILE),
            "a wrong answer writes nothing"
        );
    }

    /// Spec §13 item 20: a changed per-target map, a target added or removed included, skips
    /// step 4 for every target, and `alz account which` warns with the old and new values.
    #[test]
    fn a_target_added_or_removed_skips_the_layer_step_for_every_target() {
        const BOTH: &str = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\naccount:\n  claude: client-a\n  codex: client-a-api\n";
        const CLAUDE_ONLY: &str = "name: client\nkind: client\nvisibility: private\nversion: 0.1.0\naccount:\n  claude: client-a\n";
        const OLD_BOTH: &str = "claude=client-a, codex=client-a-api";
        const NEW_ONE: &str = "claude=client-a";
        let cases = [
            ("a target removed", None, CLAUDE_ONLY, OLD_BOTH, NEW_ONE),
            ("a target added", Some(CLAUDE_ONLY), BOTH, NEW_ONE, OLD_BOTH),
        ];
        for (what, approved_text, changed, old, new) in cases {
            let world = approved(approved_text);
            let state = world.scratch.read_text(World::STATE_FILE).expect("read");
            world
                .scratch
                .write("work/hub/layer/layer.yaml", changed)
                .expect("written");
            world.commit_hub();
            assert_eq!(world.plain(&["build"]).code, Some(0), "{what}");
            let which = world.plain(&["account", "which"]);
            assert_eq!(
                which.code,
                Some(1),
                "{what}: {}{}",
                which.stdout,
                which.stderr
            );
            for target in ["claude", "codex"] {
                assert!(
                    which.stdout.contains(&format!("{target}: fails closed")),
                    "{what}: {}",
                    which.stdout
                );
            }
            assert!(
                !which.stdout.contains("client-a (source: layer)")
                    && !which.stdout.contains("client-a-api (source: layer)"),
                "{what}: {}",
                which.stdout
            );
            assert!(
                which.stderr.contains("warning [account-changed]")
                    && which.stderr.contains(&format!("was {old} from"))
                    && which.stderr.contains(&format!("now {new} from")),
                "{what}: {}",
                which.stderr
            );
            assert!(which.stdout.contains("fix: alz account allow"), "{what}");
            assert_eq!(
                world.scratch.read_text(World::STATE_FILE).expect("read"),
                state,
                "{what}"
            );
        }
    }
}
