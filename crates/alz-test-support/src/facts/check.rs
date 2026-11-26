//! The check of spec §8 **What an adapter may rely on**: every key an adapter writes rests on a
//! recorded fact, for any adapter.

use std::collections::BTreeSet;
use std::fmt::{self, Debug};

use alz_adapter::Output;

use super::key_path::KeyPath;
use super::listing::written_files;
use super::record::{Facts, Subject};

/// What an adapter says about its output: which facts each written file and key rests on. This is
/// the adapter's subject map; the harness does the rest.
///
/// A [`Kind`](SubjectMap::Kind) names a kind of output (an agent field, a settings key, a file
/// place). A key is recorded when a verified row of the subject's sections names it in a code
/// span; a file location is recorded when the subject has a verified row at all.
pub trait SubjectMap {
    /// The kinds of output the adapter writes. Exemptions are listed by kind and key.
    type Kind: Copy + Ord + Debug;

    /// The kind and the facts for the location of `file`, or `None` when the adapter lists no such
    /// place, which the check reports as an unplaced file.
    fn location(&self, file: &str) -> Option<(Self::Kind, Subject)>;

    /// The kind and the facts for the key at `path` in `file`, or `None` when the key is not alz's
    /// to choose (a server id, a variable name) or another check covers it.
    fn key(&self, file: &str, path: &KeyPath) -> Option<(Self::Kind, Subject)>;
}

/// What the check found: an empty value is a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Findings<K: Ord> {
    /// Files the subject map has no place for.
    pub unplaced: BTreeSet<String>,
    /// Keys (a location has the key `""`) that the facts do not record and that are not exempt.
    pub unrecorded: BTreeSet<(K, String)>,
    /// Exemptions that no output uses any more: nothing the adapter writes is that kind and key.
    pub unused_exemptions: BTreeSet<(K, String)>,
    /// Exemptions that the facts record now, so the exemption has to go.
    pub recorded_exemptions: BTreeSet<(K, String)>,
}

impl<K: Ord> Findings<K> {
    /// Whether nothing was found.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.unplaced.is_empty()
            && self.unrecorded.is_empty()
            && self.unused_exemptions.is_empty()
            && self.recorded_exemptions.is_empty()
    }
}

impl<K: Ord + Debug> fmt::Display for Findings<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "unplaced files: {:?}", self.unplaced)?;
        writeln!(
            formatter,
            "keys that no verified row of the facts records: {:?}",
            self.unrecorded
        )?;
        writeln!(
            formatter,
            "exemptions no output uses any more: {:?}",
            self.unused_exemptions
        )?;
        write!(
            formatter,
            "exemptions the facts record now: {:?}",
            self.recorded_exemptions
        )
    }
}

/// Checks that every file location and key that `output` writes is recorded by `facts`, as `map`
/// says which facts each rests on. `exempt` lists the kind and key of what rests on no recorded
/// fact today, which makes it a ratchet: an unrecorded key not listed fails, and a listed one that
/// the facts record now, or that no output writes any more, fails too.
#[must_use]
pub fn check<M: SubjectMap>(
    facts: &Facts,
    map: &M,
    output: &Output,
    exempt: &[(M::Kind, &str)],
) -> Findings<M::Kind> {
    let files = written_files(output);
    let unplaced = files
        .iter()
        .filter(|written| map.location(&written.file).is_none())
        .map(|written| written.file.clone())
        .collect();
    let claims: Vec<(M::Kind, String, bool)> = files
        .iter()
        .flat_map(|written| {
            let location = map
                .location(&written.file)
                .map(|(kind, subject)| (kind, String::new(), facts.records_location(&subject)));
            let keys = written.keys.iter().filter_map(|path| {
                let (kind, subject) = map.key(&written.file, path)?;
                let name = path.name()?;
                Some((kind, name.to_owned(), facts.records_name(&subject, name)))
            });
            location.into_iter().chain(keys).collect::<Vec<_>>()
        })
        .collect();
    let written: BTreeSet<(M::Kind, String)> = claims
        .iter()
        .map(|(kind, key, _)| (*kind, key.clone()))
        .collect();
    let unrecorded_all: BTreeSet<(M::Kind, String)> = claims
        .iter()
        .filter(|(_, _, recorded)| !recorded)
        .map(|(kind, key, _)| (*kind, key.clone()))
        .collect();
    let exempt: BTreeSet<(M::Kind, String)> = exempt
        .iter()
        .map(|(kind, key)| (*kind, (*key).to_owned()))
        .collect();
    Findings {
        unplaced,
        unrecorded: unrecorded_all.difference(&exempt).cloned().collect(),
        unused_exemptions: exempt.difference(&written).cloned().collect(),
        recorded_exemptions: exempt
            .intersection(&written)
            .filter(|one| !unrecorded_all.contains(*one))
            .cloned()
            .collect(),
    }
}

/// Fails the test, with the findings, unless `output` rests on recorded facts only (see
/// [`check`]).
///
/// # Panics
///
/// When [`check`] finds anything: that is the failure.
#[track_caller]
pub fn assert_rests_on_facts<M: SubjectMap>(
    facts: &Facts,
    map: &M,
    output: &Output,
    exempt: &[(M::Kind, &str)],
) {
    let findings = check(facts, map, output, exempt);
    assert!(
        findings.is_clean(),
        "an output key rests on no verified row of the facts file, or an exemption is stale:\n{findings}"
    );
}
