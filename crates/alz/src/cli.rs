//! The command line, as clap derive types. clap owns the help text, the usage errors and
//! `--version` (decision-26); the commands of the specification are added here, one at a time.

use std::path::PathBuf;

use alz_core::account::AccountChoice;
use alz_core::explain::ItemType;
use alz_core::layer::Visibility;
use alz_core::target::TargetName;
use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

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
    Build(BuildArgs),
    /// Fail when the generated files or `layer.lock` differ from a fresh build; writes nothing.
    Check(CheckArgs),
    /// Show the change a build would make, as unified diffs; writes nothing.
    Diff,
    /// Scaffold a repository's layer manifest and repo layer, or a new layer.
    Init(InitArgs),
    /// Validate a layer, or a repository's stack manifest and lock.
    Lint(LintArgs),
    /// List what the stack builds, by section, with the layer that decides each item.
    Ls(LsArgs),
    /// Check the account folders, the Claude version and the folders around the repository.
    Doctor,
    /// Show which layers contributed to an item, then the merged output file.
    Explain(ExplainArgs),
    /// Choose and inspect the account each tool starts with.
    Account(AccountArgs),
}

/// The arguments of `alz account` (§10).
#[derive(Debug, Args)]
pub struct AccountArgs {
    /// What to do with accounts.
    #[command(subcommand)]
    pub command: AccountCommand,
}

/// The subcommands of `alz account` that this binary implements (§10).
#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Print the resolved account per target, its source and what the hook applies; fetches
    /// nothing, runs no git and writes nothing.
    Which(WhichArgs),
    /// Show the account this repository's layers name and, once a person at a terminal confirms
    /// it, record it in `allowed.yaml` so it applies; `--remove` withdraws the entry.
    Allow(AllowArgs),
}

/// The arguments of `alz account allow` (§10). There is no way to answer for the person: no
/// `--yes`, no `-y`, nothing else is accepted.
#[derive(Debug, Args)]
pub struct AllowArgs {
    /// Remove this repository's entry from `allowed.yaml` instead of recording one.
    #[arg(long)]
    pub remove: bool,
}

/// The arguments of `alz account which` (§10).
#[derive(Debug, Args)]
pub struct WhichArgs {
    /// The choice of `alz use`, as the shell function passes it: `<target>=<name>`, once per
    /// target.
    #[arg(long = "use", value_name = "target=name", value_parser = parse_use)]
    pub choices: Vec<UseChoice>,
}

/// One `--use <target>=<name>`: a known target and an account name or `default`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseChoice {
    pub target: TargetName,
    pub choice: AccountChoice,
}

impl WhichArgs {
    /// The choices, or the usage error that names a target given twice.
    ///
    /// # Errors
    ///
    /// A clap error of kind argument conflict when a target is given more than once.
    pub fn use_choices(&self) -> Result<Vec<(TargetName, AccountChoice)>, clap::Error> {
        match TargetName::ALL.into_iter().find(|target| {
            self.choices
                .iter()
                .filter(|choice| choice.target == *target)
                .count()
                > 1
        }) {
            Some(target) => Err(Cli::command().error(
                ErrorKind::ArgumentConflict,
                format!("--use names the target {target} more than once"),
            )),
            None => Ok(self
                .choices
                .iter()
                .map(|choice| (choice.target, choice.choice.clone()))
                .collect()),
        }
    }
}

/// `<target>=<name>`: the target is `claude` or `codex`, the name an account name or `default`.
fn parse_use(text: &str) -> Result<UseChoice, String> {
    let (target, name) = text
        .split_once('=')
        .ok_or_else(|| "expected <target>=<name>".to_owned())?;
    let target = TargetName::try_from(target)
        .map_err(|_unknown| "the target must be claude or codex".to_owned())?;
    let choice = AccountChoice::try_from(name).map_err(|_invalid| {
        "the name must be lower-case letters, digits and '-' (or default)".to_owned()
    })?;
    Ok(UseChoice { target, choice })
}

/// The arguments of `alz ls` (§10.3).
#[derive(Debug, Args)]
pub struct LsArgs {
    /// Also ask Claude what it loaded here (one short model call) and fail on any built item it
    /// did not load.
    #[arg(long)]
    pub live: bool,
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

/// The arguments of `alz build` (§10).
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Turn hand-written agent config into repo-layer sources first (§4.3).
    #[arg(long)]
    pub adopt: bool,
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
