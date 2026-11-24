//! `alz account which` (spec §10.3): the account each target resolves to, its source, the steps
//! skipped and what the hook applies. It fetches nothing, runs no git and writes nothing: the
//! command is handed no `GitRunner`, `RepoRoot` or `StateDir`, so it cannot.
//!
//! The report goes to standard output, the warnings after it to standard error. The exit code is
//! 0 when every target resolves (warnings included) and 1 when one fails closed; a usage error
//! is 2 and is decided before this runs.

use std::io::{self, Write};

use alz_core::Diagnostics;
use alz_core::account::{resolve, which_report};
use alz_io::{CachedLayers, Home};

use crate::cli::WhichArgs;
use crate::command::Surroundings;
use crate::command::account::facts::gather_with;
use crate::report::emit_diagnostics;

/// Runs `alz account which`; the `--use` choices are checked by the caller.
pub fn run(
    args: &WhichArgs,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    run_at(
        args,
        surroundings,
        Home::locate(surroundings.env),
        stdout,
        stderr,
    )
}

/// As [`run`], with `$HOME` already located: the seam for a test of a process that has none.
fn run_at(
    args: &WhichArgs,
    surroundings: Surroundings<'_>,
    home: Home,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let choices = match args.use_choices() {
        Ok(choices) => choices,
        Err(error) => return crate::report::Rendered::of(error).emit(stdout, stderr),
    };
    let gathered = match gather_with(surroundings.env, surroundings.directory, &choices, home) {
        Ok(gathered) => gathered,
        Err(diagnostic) => return emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr),
    };
    let layers = CachedLayers::of(
        gathered.search.repository(),
        &gathered.folders,
        &gathered.home,
        surroundings.env,
    );
    let resolution = resolve(&gathered.facts, &layers);
    let report = which_report(&gathered.facts, &resolution, gathered.warnings);
    if write_through(stdout, &report.to_string()).is_err() {
        return 1;
    }
    let warned = emit_diagnostics(report.warnings(), stderr);
    report.exit_code().max(warned)
}

fn write_through(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use alz_core::env::EnvVars;
    use alz_core::path::AbsolutePath;
    use alz_io::{EnvSnapshot, Home, WorkingDirectory};
    use alz_test_support::ScratchDir;

    use super::run_at;
    use crate::cli::WhichArgs;
    use crate::command::Surroundings;

    /// TASK-72: with `HOME` unset and no user database entry, `alz account which` exits 0, warns
    /// once that `HOME` is not an absolute path and reports the default account for every target.
    /// The binary's own `HOME` lookup cannot be made to find nothing from outside, so this runs
    /// the command in process with the lookup's answer given, `Home::locate_with(env, || None)`.
    #[test]
    fn with_home_unset_and_no_user_database_entry_every_target_resolves_to_default() {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in ["config", "state", "cache", "repo/.layer"] {
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
        // No HOME at all; every other place the command may look at is inside the scratch folder.
        let env = EnvSnapshot::from_vars(
            [
                ("XDG_CONFIG_HOME", "config"),
                ("XDG_STATE_HOME", "state"),
                ("XDG_CACHE_HOME", "cache"),
            ]
            .into_iter()
            .map(|(name, folder)| (name.into(), scratch.join(folder).into_os_string()))
            .collect::<EnvVars>(),
        );
        assert!(env.home().is_none(), "the environment has no HOME");
        let home = Home::locate_with(&env, || None);
        assert_eq!(home, Home::NotAbsolute);
        let repository = scratch.join("repo");
        let directory = WorkingDirectory::at(
            AbsolutePath::try_from(repository.to_str().expect("text")).expect("absolute"),
        );
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code = run_at(
            &WhichArgs {
                choices: Vec::new(),
            },
            Surroundings {
                env: &env,
                directory: &directory,
            },
            home,
            &mut stdout,
            &mut stderr,
        );
        let stdout = String::from_utf8(stdout).expect("text");
        let stderr = String::from_utf8(stderr).expect("text");
        assert_eq!(code, 0, "{stderr}");
        assert!(
            stdout.contains("claude: default (source: default) CLAUDE_CONFIG_DIR unset"),
            "{stdout}"
        );
        assert!(
            stdout.contains("codex: default (source: default) CODEX_HOME unset"),
            "{stdout}"
        );
        assert_eq!(
            stderr
                .lines()
                .filter(|line| line.contains("[account-home]"))
                .count(),
            1,
            "{stderr}"
        );
        assert_eq!(stderr.lines().count(), 1, "{stderr}");
    }
}
