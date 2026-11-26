//! One harness for spec §8 **What an adapter may rely on**, for every adapter: it lists the keys an
//! adapter writes from its structured output (§8.0 **Output.**) and checks each against the facts
//! file that the adapter's descriptor names (§8.0 **Descriptor.**).
//!
//! An adapter's test gives the harness its output, the facts read from the descriptor's file, a
//! [`SubjectMap`] that says which sections of the facts each kind of key rests on, and the list of
//! exemptions it has today. The harness is test support, not the adapter contract, because it
//! reads a file of the repository; the pure crates stay free of effects, and an adapter crate
//! takes it as a dev-dependency.
mod check;
mod key_path;
mod listing;
mod record;

#[cfg(test)]
mod tests;

pub use check::{Findings, SubjectMap, assert_rests_on_facts, check};
pub use key_path::{KeyPath, Part};
pub use listing::{WrittenFile, written_files};
pub use record::{Facts, FactsError, Subject, is_unverified};
