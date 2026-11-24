//! The command line, as clap derive types. clap owns the help text, the usage errors and
//! `--version` (decision-26); the commands of the specification are added here, one at a time.

use std::path::PathBuf;

use alz_core::explain::ItemType;
use alz_core::layer::Visibility;
use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// Resolve, merge and render the stack, then write the generated files and `layer.lock`.
    Build,
    /// Fail when the generated files or `layer.lock` differ from a fresh build; writes nothing.
    Check(CheckArgs),
    /// Show the change a build would make, as unified diffs; writes nothing.
    Diff,
    /// Scaffold a repository's layer manifest and repo layer, or a new layer.
    Init(InitArgs),
    /// Validate a layer, or a repository's stack manifest and lock.
    Lint(LintArgs),
    /// List what the stack builds, by section, with the layer that decides each item.
    Ls,
    /// Show which layers contributed to an item, then the merged output file.
    Explain(ExplainArgs),
}

/// The kind of item `alz explain` takes (§10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ItemTypeArg {
    /// An agent.
    Agent,
    /// A skill.
    Skill,
    /// A command.
    Command,
    /// A path-scoped rule.
    Rule,
    /// A hook.
    Hook,
    /// An MCP server.
    Mcp,
    /// An instruction file, by its file name.
    Instruction,
}

impl From<ItemTypeArg> for ItemType {
    fn from(argument: ItemTypeArg) -> Self {
        match argument {
            ItemTypeArg::Agent => Self::Agent,
            ItemTypeArg::Skill => Self::Skill,
            ItemTypeArg::Command => Self::Command,
            ItemTypeArg::Rule => Self::Rule,
            ItemTypeArg::Hook => Self::Hook,
            ItemTypeArg::Mcp => Self::Mcp,
            ItemTypeArg::Instruction => Self::Instruction,
        }
    }
}

/// The arguments of `alz explain` (§10.3).
#[derive(Debug, Args)]
pub struct ExplainArgs {
    /// The kind of item.
    #[arg(value_enum, value_name = "type")]
    pub item_type: ItemTypeArg,
    /// The item's name; for an instruction, its file name such as `00-core.md`.
    #[arg(value_name = "name")]
    pub name: String,
}

/// The arguments of `alz check` (§10).
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Skip the comparison of the git identity with the declared commit identity (§9.4).
    #[arg(long)]
    pub skip_identity: bool,
}

/// The arguments of `alz lint` (§10).
#[derive(Debug, Args)]
pub struct LintArgs {
    /// Lint the layer in this directory instead of the repository in the working directory.
    #[arg(long, value_name = "path")]
    pub layer: Option<PathBuf>,
}

/// What `alz init` scaffolds (§10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InitKind {
    /// A repository: its `layer.yaml`, repo layer and mise tasks.
    Repo,
    /// A new layer.
    Layer,
}

/// A layer's visibility on the command line (§5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VisibilityArg {
    /// May be published.
    Public,
    /// Not published.
    Private,
    /// Not published and not shared outside the client.
    Confidential,
}

impl From<VisibilityArg> for Visibility {
    fn from(argument: VisibilityArg) -> Self {
        match argument {
            VisibilityArg::Public => Self::Public,
            VisibilityArg::Private => Self::Private,
            VisibilityArg::Confidential => Self::Confidential,
        }
    }
}

/// The arguments of `alz init` (§10).
#[derive(Debug, Args)]
pub struct InitArgs {
    /// What to scaffold.
    #[arg(long, value_enum, default_value_t = InitKind::Repo)]
    pub kind: InitKind,
    /// The layer's name; the folder's name by default.
    #[arg(long, value_name = "name")]
    pub name: Option<String>,
    /// The layer's visibility; `private` for a repository and `public` for a layer by default.
    #[arg(long, value_enum)]
    pub visibility: Option<VisibilityArg>,
    /// A layer below the repo layer, most general first: `github:` and `git:` sources as written,
    /// any other value a `path:` source, `<dir>@<ref>` pinning a ref. Repeatable.
    #[arg(long, value_name = "source")]
    pub stack: Vec<String>,
    /// A hub folder: its `layer/` joins the stack, pinned to its newest `layer-v*` tag.
    #[arg(long, value_name = "dir")]
    pub hub: Option<String>,
}
