//! `alz doctor` (spec §8.4, §10.3): the account folders against the built names, the Claude
//! version against the verified range, the files Claude also loads from around the repository,
//! and the build's own diagnostics.
//!
//! The shared steps run first; their diagnostics are part of the report and not printed
//! separately, and an error in them is a problem and not a stop. The report goes to standard
//! output.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;

use alz_core::doctor::Version;
use alz_core::doctor::{ClaudeFacts, Facts, Installed, Report};
use alz_core::live::Built;
use alz_core::{Diagnostics, Stripped};
use alz_io::{
    Purpose, ToolRunner, blocked_folders, claude_account_folders, client_root_of,
    observe_ancestors, observe_client_root, observe_workspace,
};

use crate::command::Surroundings;
use crate::command::pipeline::{self, Pipeline};
use crate::report::emit_diagnostics;

/// Runs `alz doctor` and returns the exit code: 0 when it found no problem, else 1.
pub fn run(surroundings: Surroundings<'_>, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let repository = match pipeline::open_repository(surroundings) {
        Ok(repository) => repository,
        Err(diagnostic) => return emit_diagnostics(&Diagnostics::new().with(diagnostic), stderr),
    };
    let Some(root) = surroundings.directory.path() else {
        return 1;
    };
    let built = pipeline::compute(surroundings, &repository);
    let facts = facts_of(surroundings, Path::new(root.as_str()), built);
    let report = Report::of(&facts);
    match write_text(stdout, &report.to_string()) {
        Ok(()) => u8::from(report.problems() > 0),
        Err(_) => 1,
    }
}

/// What the report is made from. The file system is looked at here and nowhere else.
fn facts_of(surroundings: Surroundings<'_>, root: &Path, pipeline: Pipeline) -> Facts {
    let names = pipeline
        .computed
        .as_ref()
        .map(|computed| Built::of(&computed.merged));
    let manifest = pipeline.manifest.as_ref();
    let builds_for_claude =
        manifest.is_none_or(|manifest| manifest.targets().contains(alz_core::target::claude()));
    let client_root = manifest
        .and_then(|manifest| manifest.workspace().hub())
        .and_then(|hub| client_root_of(root, Path::new(hub.as_str())));
    Facts {
        root: Stripped::from_untrusted(&root.to_string_lossy()),
        claude: builds_for_claude.then(|| ClaudeFacts {
            accounts: claude_account_folders(surroundings.env),
            installed: installed_version(surroundings, root),
            built: names.unwrap_or_default(),
        }),
        client_root: client_root.as_deref().map(observe_client_root),
        workspace: manifest.map_or_default(|manifest| {
            manifest
                .workspace()
                .add_dirs()
                .iter()
                .map(|dir| observe_workspace(root, dir.as_str()))
                .collect()
        }),
        ancestors: observe_ancestors(root, client_root.as_deref()),
        blocked: blocked_folders(surroundings.env),
        build: pipeline.diagnostics,
    }
}

/// `<claude> --version`: the version it printed, or not found when there is no program, it fails,
/// it is too slow or prints no version.
fn installed_version(surroundings: Surroundings<'_>, root: &Path) -> Installed {
    ToolRunner::claude(surroundings.env, Purpose::Version)
        .ok()
        .and_then(|runner| runner.run(&[OsString::from("--version")], root).ok())
        .filter(|output| output.success)
        .and_then(|output| Version::find_in(&String::from_utf8_lossy(&output.stdout)))
        .map_or(Installed::NotFound, Installed::Version)
}

fn write_text(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}
