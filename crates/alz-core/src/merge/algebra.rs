//! The structures that combine values (spec §7, doc-6 §3.5). Each is a small algebra with an
//! inherent `merge(self, later)`, where `later` is the more specific value, and its laws are
//! property tests next to it:
//!
//! - [`Last`] and [`Replace`]: the most specific value wins. Associative, with an identity.
//! - [`OrderedSet`]: a union that keeps the first occurrence. Associative, idempotent and
//!   commutative as a set.
//! - [`ListEdit`]: `+[..]`, `-[..]` and `[..]` edits of a list. Composition is associative and
//!   applying a composition equals applying the edits one after the other.
//! - [`Tree::deep_merge`]: the left fold of settings. Associative over kind-consistent trees.
//! - [`Floor`]: a join on an ordered type, so a later value can raise a setting and never lower it.

mod deep_merge;
mod floor;
mod last;
mod list_edit;
mod ordered_set;
mod replace;

pub use deep_merge::{FloatBits, Tree};
pub use floor::Floor;
pub use last::Last;
pub use list_edit::ListEdit;
pub use ordered_set::OrderedSet;
pub use replace::Replace;
