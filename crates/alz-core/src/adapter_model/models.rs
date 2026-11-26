//! The model routing resolved for one target (spec §7.8).

use alz_adapter::{Models, TargetId};

use crate::merge::Merged;

/// The aliases that have a model for `target`, the roles and the model of the default alias.
pub(super) fn of(merged: &Merged, target: &TargetId) -> Models {
    let routing = merged.models();
    let mut models = Models::default();
    models.aliases = routing
        .aliases()
        .keys()
        .filter_map(|alias| {
            routing
                .model(alias, target)
                .map(|concrete| (alias.as_str().to_owned(), concrete.as_str().to_owned()))
        })
        .collect();
    models.roles = routing
        .roles()
        .iter()
        .map(|(role, alias)| (role.as_str().to_owned(), alias.value().as_str().to_owned()))
        .collect();
    models.default = routing
        .for_default(target)
        .map(|concrete| concrete.as_str().to_owned());
    models
}
