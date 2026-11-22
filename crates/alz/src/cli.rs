//! The command line, as clap derive types. clap owns the help text, the usage errors and
//! `--version` (decision-26); the commands of the specification are added here, one at a time.

use clap::Parser;

/// Compose layered coding-agent configuration into repositories.
#[derive(Debug, Parser)]
#[command(name = "alz", version, arg_required_else_help = true)]
pub struct Cli {}
