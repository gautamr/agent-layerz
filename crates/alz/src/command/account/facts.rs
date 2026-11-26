//! What `alz account which` finds out before it decides (spec §7.13): the environment, `$HOME`,
//! the working directory's place, the repository root, the two account files and the account
//! folders, turned into the values [`resolve`](alz_core::account::resolve) is told.
//!
//! Each effect happens once, here, through `alz-io`; nothing is fetched, no git runs and nothing
//! is written. The working directory is the one the process captured at its start: it is placed
//! once, and the walk climbs by the parents of that real path.

use std::path::Path;

use alz_core::account::AccountFamily;
use alz_core::account::{
    AccountChoice, AccountFolders, AllowFileProblem, AllowedFact, ConfigFact, EnvironmentFact,
    ExistingFolders, Explicit, ExplicitName, Facts, FileFact, FileOutcome, HomeFact, ManifestFact,
    NoRepository, VariableFact,
};
use alz_core::path::AbsolutePath;
use alz_core::{Code, Diagnostic, Diagnostics, Message, Stripped};
use alz_io::{
    AccountFiles, AccountFolderNames, EnvSnapshot, FileStatus, Home, Loaded, ManifestState, NoRoot,
    ReadOnlyDir, Repository, RepositoryPaths, RootSearch, WorkingDirectory, allow_file_problem,
    find_root,
};

/// Everything resolution is told, the warnings found on the way, and what the layers are read
/// from when step 4 asks for them.
#[derive(Debug)]
pub struct Gathered {
    pub facts: Facts,
    /// `$HOME`, the two files and the root walk, in that order.
    pub warnings: Diagnostics,
    pub search: RootSearch,
    pub folders: AccountFolders,
    pub home: Home,
    /// The two files as found from the repository root, for the writer of `allowed.yaml`.
    pub files: AccountFiles,
    /// What makes `allowed.yaml` unusable as a whole, when something does.
    pub allow_problem: Option<AllowFileProblem>,
}

/// Finds out everything `alz account which` needs, from the environment and the working
/// directory the process started with. `choices` are the `--use` choices, already valid.
pub fn gather(
    env: &EnvSnapshot,
    directory: &WorkingDirectory,
    choices: &[(AccountFamily, AccountChoice)],
) -> Result<Gathered, Diagnostic> {
    gather_with(env, directory, choices, Home::locate(env))
}

/// As [`gather`], with `$HOME` already located. This is the seam that lets a test stand for a
/// process whose `HOME` is unset and whose user has no user database entry (TASK-72).
pub fn gather_with(
    env: &EnvSnapshot,
    directory: &WorkingDirectory,
    choices: &[(AccountFamily, AccountChoice)],
    home: Home,
) -> Result<Gathered, Diagnostic> {
    let files = AccountFiles::new(home.clone(), env, None);
    let folders = files.folders().clone();
    let real_home = files.real_home();
    let homes: Vec<&AbsolutePath> = [home.path(), real_home.as_ref()]
        .into_iter()
        .flatten()
        .collect();
    let root_folder = ReadOnlyDir::open(Path::new("/")).map_err(|_unreadable| {
        Diagnostic::error(
            Code::Account,
            None,
            Message::new("cannot read the file system root"),
        )
    })?;
    // The working directory is placed once; the walk gets the place by value.
    let search = find_root(
        directory.clone().resolve(&folders),
        &folders,
        &homes,
        &root_folder,
    );
    let files = match search.repository() {
        Repository::Root(root) => files.in_repository(RepositoryPaths::new(
            root.path().clone(),
            root.path().clone(),
        )),
        Repository::NoRoot(_) => files,
    };
    let config = files.user_config();
    let allowed = files.allow_state();
    let warnings = files
        .home_warning()
        .into_iter()
        .fold(Diagnostics::new(), Diagnostics::with)
        .merge(config.warnings().clone())
        .merge(allowed.warnings().clone())
        .merge(search.warnings().clone());
    let allow_problem = allow_file_problem(&allowed);
    let facts = Facts {
        home: home_fact(&home, &files),
        environment: environment_fact(env),
        explicit: Explicit {
            run: None,
            used: choices
                .iter()
                .map(|(target, choice)| (*target, ExplicitName::Valid(choice.clone())))
                .collect(),
        },
        config: ConfigFact {
            file: file_fact(&config, &files, "~/.config/agent-layerz/accounts.yaml"),
            default_name: config.value().default_name().cloned(),
            folders: config
                .value()
                .folders()
                .iter()
                .map(|folder| (folder.real().clone(), folder.choice().clone()))
                .collect(),
        },
        allowed: AllowedFact {
            file: file_fact(&allowed, &files, "~/.local/state/agent-layerz/allowed.yaml"),
            entries: allowed.value().entries().to_vec(),
            unresolved: allowed
                .value()
                .unresolved()
                .iter()
                .map(|entry| entry.root().clone())
                .collect(),
        },
        repository: repository_fact(search.repository()),
        known_directory: search.known_directory().cloned(),
        folders: existing_folders(&home),
    };
    Ok(Gathered {
        facts,
        warnings,
        search,
        folders,
        home,
        files,
        allow_problem,
    })
}

fn home_fact(home: &Home, files: &AccountFiles) -> HomeFact {
    match home {
        Home::Absolute(given) => HomeFact::Absolute {
            given: given.clone(),
            real: files.real_home(),
        },
        Home::NotAbsolute => HomeFact::NotAbsolute,
    }
}

/// An account variable and its marker, by value; a value that does not read as text is shown
/// with its replacement characters, which keeps it different from any marker alz wrote.
fn variable_fact(env: &EnvSnapshot, variable: &str, marker: &str) -> VariableFact {
    env.vars()
        .get(variable)
        .map_or(VariableFact::Unset, |value| VariableFact::Set {
            value: value.to_string_lossy().into_owned(),
            marker: env
                .vars()
                .get(marker)
                .map(|marker| marker.to_string_lossy().into_owned()),
        })
}

fn environment_fact(env: &EnvSnapshot) -> EnvironmentFact {
    EnvironmentFact {
        claude: variable_fact(env, "CLAUDE_CONFIG_DIR", "ALZ_SET_CLAUDE_CONFIG_DIR"),
        codex: variable_fact(env, "CODEX_HOME", "ALZ_SET_CODEX_HOME"),
        manager_active: env.mise_or_direnv_active(),
    }
}

/// The file as the report shows it; `fallback` is the default place, shown when `$HOME` is not an
/// absolute path and so no path was made.
fn file_fact<T>(file: &Loaded<T>, files: &AccountFiles, fallback: &str) -> FileFact {
    FileFact {
        shown: file.path().map_or_else(
            || Stripped::from_untrusted(fallback),
            |path| files.shown(path),
        ),
        outcome: match file.status() {
            FileStatus::Read => FileOutcome::Read,
            FileStatus::NotFound => FileOutcome::NotFound,
            FileStatus::Ignored(reason) => FileOutcome::Ignored(reason.after(Message::new(""))),
        },
    }
}

fn repository_fact(repository: &Repository) -> alz_core::account::Repository {
    match repository {
        Repository::Root(root) => alz_core::account::Repository::Root {
            path: root.path().clone(),
            manifest: match root.manifest() {
                ManifestState::Read(_) => ManifestFact::Read,
                ManifestState::Unreadable => ManifestFact::Unreadable,
                ManifestState::Invalid => ManifestFact::Invalid,
            },
        },
        Repository::NoRoot(why) => alz_core::account::Repository::None(match why {
            NoRoot::NotFound => NoRepository::NotFound,
            NoRoot::AccountFolder => NoRepository::AccountFolder,
            NoRoot::ConfiguredFolder(_) => NoRepository::ConfiguredFolder,
            NoRoot::Unresolvable => NoRepository::Unresolvable,
        }),
    }
}

/// The account folders of one listing of `$HOME`; none when `$HOME` is not an absolute path.
fn existing_folders(home: &Home) -> ExistingFolders {
    home.path().map_or_else(ExistingFolders::default, |home| {
        let listed = AccountFolderNames::list(home);
        let of = |target| listed.names(target).cloned().collect::<Vec<_>>();
        ExistingFolders::new(of(AccountFamily::Claude), of(AccountFamily::Codex))
    })
}
