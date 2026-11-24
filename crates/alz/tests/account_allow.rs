//! `alz account allow` and `--remove` (spec §7.13 "Trust and `alz account allow`", §10.3,
//! TASK-55.7.13), driven through a pseudo-terminal with the `pty-run` driver of `alz-test-support`.
//!
//! Every run has its own `HOME` and no `XDG_*` variable, so the allow state lives in the scratch
//! folder and never in the real one. `--remove` needs no stack, so these cases record an entry for
//! the repository by hand; the cases that offer a declaration of a layer, with its cache and lock,
//! are in the acceptance suite (`test/acceptance/40-*`, `41-*`), which runs this binary.

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
    use std::path::PathBuf;
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use alz_core::account::AgentSessionMarker;
    use alz_test_support::{ScratchDir, pty_run_executable, read_tree};
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
}
