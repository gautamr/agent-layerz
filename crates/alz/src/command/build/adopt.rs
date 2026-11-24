//! The first half of `alz build --adopt` (spec §4.3, §10.3): observe the native files, plan the
//! adoption in full, and apply it through the repository root. The build then runs again from the
//! manifest, which the caller does with the pipeline this returns.

use std::collections::BTreeMap;
use std::io::{self, Write};

use alz_core::adopt::{
    ADOPT_ALWAYS, AdoptionInput, AdoptionPlan, Candidates, candidates, plan as plan_adoption,
};
use alz_core::path::SafeRelPath;
use alz_core::render::write_plan::Observed;
use alz_core::{Diagnostics, Stripped};
use alz_io::{AdoptionBasis, RepoError, RepoRoot};

use crate::command::Surroundings;
use crate::command::build::collision_diagnostics;
use crate::command::pipeline::{self, Computed, Pipeline};
use crate::report::emit_diagnostics;

/// What a refused adoption ends with.
const NOTHING_ADOPTED: &str = "nothing was adopted; fix the problems above and rerun\n";

/// The file `CLAUDE.md` shares paragraphs with.
const AGENTS_MD: &str = "AGENTS.md";

/// Adopts what `built` found, then builds again. `Ok` holds the pipeline of the second run, or
/// `built` itself when it has an error, which the caller reports as any build's. `Err` is the exit
/// code of a run that stops here.
pub fn adopt_first<W: Write>(
    surroundings: Surroundings<'_>,
    repository: &RepoRoot,
    built: Pipeline,
    stdout: &mut impl Write,
    stderr: &mut W,
) -> Result<Pipeline, u8> {
    let Some(computed) = built.computed.as_ref() else {
        return Ok(built);
    };
    let stop = |diagnostics: &Diagnostics, stderr: &mut W| {
        emit_diagnostics(
            &built.diagnostics.clone().merge(diagnostics.clone()),
            stderr,
        )
    };
    let found = match observe(repository, computed) {
        Ok(found) => found,
        Err(error) => {
            return Err(stop(
                &Diagnostics::new().with(pipeline::write_failure(&error)),
                stderr,
            ));
        }
    };
    if !found.unadoptable().is_empty() {
        return Err(stop(&collision_diagnostics(found.unadoptable()), stderr));
    }
    let input = AdoptionInput {
        stack: &computed.stack,
        manifest: &computed.manifest,
        manifest_text: &computed.manifest_text,
        placements: &computed.placements,
        merged: &computed.merged,
        natives: found.natives(),
        agents_md: found.agents_md(),
    };
    let adoption = match plan_adoption(&input) {
        Ok(adoption) => adoption,
        Err(refused) => {
            let code = stop(refused.diagnostics(), stderr);
            return Err(write_text(stderr, NOTHING_ADOPTED).map_or(1, |()| code));
        }
    };
    apply(repository, computed, &found, &adoption, stdout, stderr)?;
    Ok(pipeline::compute(surroundings, repository))
}

/// Looks at the repository: the collisions of the build and the files adoption always takes,
/// into what adoption takes and what stops it.
fn observe(repository: &RepoRoot, computed: &Computed) -> Result<Candidates, RepoError> {
    let Some((plan, disk)) = pipeline::plan_with_disk(repository, computed)? else {
        return Ok(nothing());
    };
    let beside = ADOPT_ALWAYS
        .iter()
        .chain(&[AGENTS_MD])
        .filter_map(|path| SafeRelPath::try_from(*path).ok())
        .map(|path| repository.look_at(&path).map(|seen| (path, seen)))
        .collect::<Result<BTreeMap<SafeRelPath, Observed>, RepoError>>()?;
    Ok(candidates(
        plan.collisions(),
        &disk,
        &beside,
        computed.previous.as_ref(),
    ))
}

fn nothing() -> Candidates {
    candidates(
        &[],
        &alz_core::render::write_plan::Disk::default(),
        &BTreeMap::new(),
        None,
    )
}

/// Writes the plan, prints its notes to standard error and what became of each file to standard
/// output. `Err` is the exit code when the writer refuses: the repository is then as it was,
/// unless the operating system failed part way.
fn apply(
    repository: &RepoRoot,
    computed: &Computed,
    found: &Candidates,
    adoption: &AdoptionPlan,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), u8> {
    let notes = found
        .notes()
        .into_iter()
        .chain(adoption.notes().iter().cloned())
        .fold(Diagnostics::new(), Diagnostics::with);
    emit_diagnostics(&notes, stderr);
    let Some(layer) = computed.stack.layers().last() else {
        return Err(1);
    };
    let basis = AdoptionBasis {
        natives: found.natives(),
        manifest_text: &computed.manifest_text,
        layer: layer.contents(),
    };
    if let Err(error) = repository.adopt(adoption, &basis) {
        return Err(emit_diagnostics(
            &Diagnostics::new().with(pipeline::write_failure(&error)),
            stderr,
        ));
    }
    let report = adoption
        .report()
        .iter()
        .map(|adopted| {
            let from = Stripped::from_untrusted(adopted.from());
            if adopted.to().is_empty() {
                format!("adopted {from} (nothing left to keep)\n")
            } else {
                let to = adopted
                    .to()
                    .iter()
                    .map(|path| Stripped::from_untrusted(path).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("adopted {from} \u{2192} {to}\n")
            }
        })
        .collect::<String>();
    write_text(stdout, &report).map_err(|io::Error { .. }| 1)
}

fn write_text(out: &mut impl Write, text: &str) -> io::Result<()> {
    out.write_all(text.as_bytes()).and_then(|()| out.flush())
}
