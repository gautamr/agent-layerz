//! The command line, as clap derive types. clap owns the help text, the usage errors and
//! `--version` (decision-26); the commands of the specification are added here, one at a time.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Compose layered coding-agent configuration into repositories.
#[derive(Debug, Parser)]
#[command(name = "alz", version, arg_required_else_help = true)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// The commands of the specification that this binary implements (§10).
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Validate a layer, or a repository's stack manifest and lock.
    Lint(LintArgs),
}

/// The arguments of `alz lint` (§10).
#[derive(Debug, Args)]
pub struct LintArgs {
    /// Lint the layer in this directory instead of the repository in the working directory.
    #[arg(long, value_name = "path")]
    pub layer: Option<PathBuf>,
}
