//! `alz init [--kind repo|layer] [--name <name>] [--visibility <v>] [--stack <source>]...
//! [--hub <dir>]` (spec §10, §10.3): scaffolds a repository's `layer.yaml` and repo layer, or a
//! new layer.
//!
//! `alz-core` decides what is written and `alz-io`'s repository root writes it, create-only: a
//! file that exists, a link included, is never replaced. Everything that can be refused is
//! checked before the first file is written. After the scaffold of a repository, the stack is
//! resolved as `alz build` does and a declared commit identity is set in `.git/config` (§9.4).

use std::ffi::OsString;
use std::fmt;
use std::io::{self, Write};
use std::path::Path;

use alz_core::Stripped;
use alz_core::identity::Command as IdentityCommand;
use alz_core::layer::Visibility;
use alz_core::render::write_plan::Observed;
use alz_core::scaffold::{
    Hub, RepoOptions, RepoSurroundings, ScaffoldError, Scaffolded, layer_files, layer_name,
    manifest_path, mise_config_paths, repo_files, repo_layer_path, stack_entry,
};
use alz_core::stack::StackEntry;
use alz_core::yaml::MAX_BYTES;
use alz_io::{Deadlines, EnvSnapshot, GitRunner, ReadOnlyDir, RepoError, RepoRoot, WalkError};

use crate::cli::{InitArgs, InitKind};
use crate::command::Surroundings;
use crate::command::identity::{self, Step};
use crate::command::pipeline;

/// Why `alz init` stopped.
#[derive(Debug)]
enum Failure {
    /// What was asked cannot be scaffolded.
    Refused(ScaffoldError),
    /// The working directory is not known or cannot be written to.
    WorkingDirectory,
    /// The working directory is the home folder or in an account folder (§9.1).
    AccountFolder,
    /// The repository root failed.
    Root(RepoError),
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(error) => error.fmt(formatter),
            Self::WorkingDirectory => formatter.write_str("cannot open the working directory"),
            Self::AccountFolder => formatter.write_str(
                "the working directory touches an account folder or the home folder; alz never writes there",
            ),
            Self::Root(error) => error.fmt(formatter),
        }
    }
}

impl From<ScaffoldError> for Failure {
    fn from(error: ScaffoldError) -> Self {
        Self::Refused(error)
    }
}

impl From<RepoError> for Failure {
    fn from(error: RepoError) -> Self {
        Self::Root(error)
    }
}

/// What a run printed, and how it ended.
#[derive(Debug, Default)]
struct Finished {
    stdout: String,
    stderr: String,
    code: u8,
}

/// What is decided before the first file is written.
struct Plan {
    root: RepoRoot,
    scaffolded: Scaffolded,
    /// The line about the hub's client layer, when it joined the stack.
    hub_notice: Option<String>,
    kind: InitKind,
}

/// Runs `alz init`, prints what it created on `stdout` and its errors on `stderr`, and returns
/// the exit code.
pub fn run(
    args: &InitArgs,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let finished = init(args, surroundings);
    let written =
        write_all(stdout, &finished.stdout).and_then(|()| write_all(stderr, &finished.stderr));
    written.map_or(1, |()| finished.code)
}

fn write_all(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}

fn init(args: &InitArgs, surroundings: Surroundings<'_>) -> Finished {
    match plan(args, surroundings) {
        Err(failure) => Finished {
            stderr: format!("error [init] {failure}\n"),
            code: 1,
            ..Finished::default()
        },
        Ok(plan) => {
            let created = create(&plan);
            match (plan.kind, created.code) {
                (InitKind::Repo, 0) => created.then(&identity_after_scaffold(&plan, surroundings)),
                (InitKind::Repo | InitKind::Layer, _) => created,
            }
        }
    }
}

impl Finished {
    /// What this run printed, followed by `later`'s, which ends it.
    fn then(self, later: &Self) -> Self {
        Self {
            stdout: format!("{}{}", self.stdout, later.stdout),
            stderr: format!("{}{}", self.stderr, later.stderr),
            code: later.code,
        }
    }
}

/// The note that ends the step when a layer source cannot be resolved yet.
const NOT_RESOLVED_YET: &str = "note [identity] the stack could not be resolved yet, so no commit identity was set; `alz build --adopt` will set it\n";

/// Resolves the stack of the new repository as `alz build` does, and sets the commit identity it
/// declares (§9.4, §10.3). A source that is not there to read yet gets only a note. Every other
/// problem with the stack or a declaration is a warning and nothing is written, a refused source
/// or `account` declaration included; the scaffold succeeded, so the exit code stays 0 unless a
/// check after a write failed.
fn identity_after_scaffold(plan: &Plan, surroundings: Surroundings<'_>) -> Finished {
    let built = pipeline::compute(surroundings, &plan.root);
    let (not_there_yet, others) = built.diagnostics.clone().split_source_not_there_yet();
    let note = if not_there_yet.iter().next().is_some() {
        NOT_RESOLVED_YET
    } else {
        ""
    };
    let warnings = Finished {
        stderr: format!("{}{note}", others.errors_as_warnings()),
        ..Finished::default()
    };
    let step = built
        .computed
        .as_ref()
        .and_then(identity::declared_of)
        .zip(surroundings.directory.path());
    let Some((declared, directory)) = step else {
        return warnings;
    };
    let outcome =
        Step::new(declared, surroundings.env, &plan.root, directory).set_up(IdentityCommand::Init);
    warnings.then(&Finished {
        stdout: outcome.stdout,
        stderr: outcome.diagnostics.to_string(),
        code: u8::from(outcome.diagnostics.has_errors()),
    })
}

/// Checks everything that can be refused, and decides what is written.
fn plan(args: &InitArgs, surroundings: Surroundings<'_>) -> Result<Plan, Failure> {
    let directory = surroundings
        .directory
        .path()
        .ok_or(Failure::WorkingDirectory)?;
    let root = RepoRoot::at(surroundings.env, directory).map_err(|error| match error {
        WalkError::Refused(_) => Failure::AccountFolder,
        WalkError::Missing
        | WalkError::NotADirectory
        | WalkError::TooManyLinks
        | WalkError::Io(_) => Failure::WorkingDirectory,
    })?;
    if present(&root, manifest_path())? {
        return Err(ScaffoldError::LayerYamlExists.into());
    }
    let folder = directory.as_str().rsplit('/').next().unwrap_or_default();
    let name = layer_name(args.name.as_deref(), folder)?;
    let visibility = args.visibility.map_or(
        match args.kind {
            InitKind::Repo => Visibility::Private,
            InitKind::Layer => Visibility::Public,
        },
        Visibility::from,
    );
    match args.kind {
        InitKind::Layer => Ok(Plan {
            root,
            scaffolded: layer_files(&name, visibility),
            hub_notice: None,
            kind: args.kind,
        }),
        InitKind::Repo => {
            let given = args
                .stack
                .iter()
                .map(|value| stack_entry(value))
                .collect::<Result<Vec<_>, _>>()?;
            let hub = args.hub.as_deref().map(Hub::new).transpose()?;
            let client = hub
                .as_ref()
                .map(|hub| client_layer(hub, surroundings.env))
                .transpose()?
                .flatten();
            let (stack, hub_notice) = with_client_layer(given, client);
            let found = RepoSurroundings {
                repo_layer_exists: present(&root, repo_layer_path())?,
                mise_config_exists: mise_config_paths()
                    .iter()
                    .map(|path| present(&root, path))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .any(|exists| exists),
            };
            let scaffolded = repo_files(
                RepoOptions {
                    name: &name,
                    visibility,
                    stack: &stack,
                    hub: hub.as_ref(),
                },
                found,
            );
            Ok(Plan {
                root,
                scaffolded,
                hub_notice,
                kind: args.kind,
            })
        }
    }
}

/// Whether anything, a link included, is at `path`.
fn present(root: &RepoRoot, path: &alz_core::path::SafeRelPath) -> Result<bool, RepoError> {
    root.look_at(path)
        .map(|observed| observed != Observed::Absent)
}

/// The hub's client layer, `<hub>/layer`, when it holds a `layer.yaml`, pinned to the newest tag
/// `layer-v*` of the repository around it. A hub that git cannot be asked about is not pinned.
fn client_layer(hub: &Hub, env: &EnvSnapshot) -> Result<Option<StackEntry>, ScaffoldError> {
    let folder = Path::new(hub.as_str()).join("layer");
    let holds_layer = ReadOnlyDir::open(&folder)
        .is_ok_and(|opened| opened.read_file(manifest_path(), MAX_BYTES).is_ok());
    if !holds_layer {
        return Ok(None);
    }
    let tag = newest_layer_tag(env, &folder);
    hub.client_entry(tag.as_deref()).map(Some)
}

/// The first tag of `git tag --list 'layer-v*' --sort=-v:refname` run in `folder`, if the run
/// succeeds and lists one.
fn newest_layer_tag(env: &EnvSnapshot, folder: &Path) -> Option<OsString> {
    GitRunner::new(env, Deadlines::STANDARD)
        .ok()
        .and_then(|runner| runner.layer_tags(folder).ok())
        .and_then(|tags| tags.into_iter().next())
}

/// The stack with the hub's client layer after the given layers, unless a given layer is already
/// that source, and the line that says so.
fn with_client_layer(
    given: Vec<StackEntry>,
    client: Option<StackEntry>,
) -> (Vec<StackEntry>, Option<String>) {
    match client {
        Some(entry) if given.iter().all(|one| one.source() != entry.source()) => {
            let at = entry
                .pin()
                .map_or_else(String::new, |pin| format!(" at {pin}"));
            let notice = format!(
                "using the hub's client layer {}{at}\n",
                Stripped::from_untrusted(&entry.source().to_string())
            );
            (given.into_iter().chain([entry]).collect(), Some(notice))
        }
        Some(_) | None => (given, None),
    }
}

/// Creates the files, in order, and reports them; the first failure stops the run after the files
/// that were created.
fn create(plan: &Plan) -> Finished {
    let created = plan.scaffolded.files().iter().map(|file| {
        plan.root
            .create_new(file.path(), file.text().as_bytes())
            .map(|()| file.path().as_str())
    });
    let mut lines = String::new();
    let mut failure = None;
    for result in created {
        match result {
            Ok(path) => {
                lines.push_str("created ");
                lines.push_str(path);
                lines.push('\n');
            }
            Err(error) => {
                failure = Some(Failure::from(error));
                break;
            }
        }
    }
    let advice = plan.scaffolded.mise_advice().unwrap_or_default();
    let before = format!(
        "{}{advice}{lines}",
        plan.hub_notice.as_deref().unwrap_or_default()
    );
    match failure {
        Some(failure) => Finished {
            stdout: before,
            stderr: format!("error [init] {failure}\n"),
            code: 1,
        },
        None => Finished {
            stdout: format!(
                "{before}{}",
                match plan.kind {
                    InitKind::Repo =>
                        "next: run `alz build --adopt` to take over existing agent config, or `alz build`\n",
                    InitKind::Layer => "next: run `alz lint`\n",
                }
            ),
            stderr: String::new(),
            code: 0,
        },
    }
}
