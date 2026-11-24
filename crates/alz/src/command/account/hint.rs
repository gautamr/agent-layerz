//! The account note of `alz build` and `alz check` (spec §7.13 "Trust", §10.3 "The account note").
//!
//! It is printed only for a person at a terminal, on standard error, and never changes the exit
//! code, standard output or a generated file. It reads what `alz account which` reads (the
//! facts, the root walk, the layer cache) and nothing else: no fetch, no git, no write.

use alz_core::Diagnostics;
use alz_core::account::{HintFacts, Repository, Stack, allow_hint};
use alz_core::merge::Declared;
use alz_io::{CachedLayers, PersonAtTerminal};

use crate::command::Surroundings;
use crate::command::account::facts::gather;
use crate::command::pipeline::Computed;

/// The note for the repository the command ran in, or no diagnostic at all: when standard input
/// is not a terminal, an agent-session marker is set, the root is none where an account can
/// apply (at or above `$HOME`, in an account folder or in a configured folder), or there is
/// nothing to say. `declares` is whether the merged configuration names an account.
pub fn allow_note(surroundings: Surroundings<'_>, declares: bool) -> Diagnostics {
    if PersonAtTerminal::check(surroundings.env).is_err() {
        return Diagnostics::new();
    }
    let Ok(gathered) = gather(surroundings.env, surroundings.directory, &[]) else {
        return Diagnostics::new();
    };
    let Repository::Root { path, .. } = &gathered.facts.repository else {
        return Diagnostics::new();
    };
    let layers = CachedLayers::of(
        gathered.search.repository(),
        &gathered.folders,
        &gathered.home,
        surroundings.env,
    );
    let declaration = layers.declaration();
    let entry = gathered
        .facts
        .allowed
        .entries
        .iter()
        .find(|entry| entry.root() == path);
    allow_hint(&HintFacts {
        declaration: &declaration,
        entry,
        problem: gathered.allow_problem.as_ref(),
        declares,
    })
    .map_or_default(|hint| Diagnostics::new().with(hint.into_diagnostic()))
}

/// Whether the merged configuration names an account: one layer's declaration applies.
pub fn names_an_account(computed: &Computed) -> bool {
    matches!(
        computed.merged.account().declared(),
        Declared::Winner { .. }
    )
}
