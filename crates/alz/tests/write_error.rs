//! The error `write` of spec §10.1 "Build and command diagnostics", through the binary: a folder
//! of the repository that cannot be read stops `alz build` and `alz check` with `error [write]`
//! that names it, exit code 1, nothing on standard output and nothing written (§4.3 "The search
//! for `AGENTS.override.md`"). Every run has its own home, `XDG_*` folders and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree, running_as_root};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    fn repository() -> ScratchDir {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in [
            "home",
            "config",
            "state",
            "cache",
            "repo/.layer",
            "repo/locked",
        ] {
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
        scratch
    }

    fn alz(scratch: &ScratchDir, command: &str) -> (i32, String, String) {
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(scratch.join("repo"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .timeout(Duration::from_secs(60))
            .arg(command)
            .output()
            .expect("alz runs and ends in time");
        (
            output.status.code().expect("an exit code"),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Spec §10.1 (`write`) and §4.3: a folder that cannot be listed is the error `write` naming
    /// it, for `build` and for `check`, with exit code 1, and the repository is unchanged.
    #[test]
    fn a_folder_that_cannot_be_listed_is_the_error_write_naming_it_and_nothing_is_written() {
        // The superuser ignores modes, so the failure this test makes cannot happen.
        if running_as_root() {
            return;
        }
        let scratch = repository();
        scratch.set_mode("repo/locked", 0o000).expect("locked");
        let before = read_tree(&scratch.join("repo/.layer")).expect("read");
        for command in ["build", "check"] {
            let (code, stdout, stderr) = alz(&scratch, command);
            assert_eq!(code, 1, "{command}: {stdout}{stderr}");
            assert_eq!(stdout, "", "{command}");
            let line = stderr
                .lines()
                .find(|line| line.starts_with("error [write]"))
                .unwrap_or_else(|| panic!("{command}: no write error: {stderr}"));
            assert!(line.contains("locked"), "{command}: {line}");
        }
        scratch.set_mode("repo/locked", 0o755).expect("unlocked");
        assert_eq!(
            read_tree(&scratch.join("repo/.layer")).expect("read"),
            before
        );
        assert!(!scratch.exists("repo/layer.lock"));
        assert!(!scratch.exists("repo/AGENTS.md"));
        assert!(!scratch.exists("repo/.claude"));
    }
}
