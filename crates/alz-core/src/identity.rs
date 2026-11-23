//! The commit identity of a layer (spec §7.12) as validated types.

mod commit_identity;

pub use commit_identity::{
    CommitIdentity, EmailAddress, EmailError, IdentityName, IdentityNameError,
};
