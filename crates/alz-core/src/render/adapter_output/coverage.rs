//! The coverage report of an adapter against the security items of the model (spec §8.0 **Output
//! checks.**, **Coverage report.** and **Security items a target cannot enforce.**).
//!
//! The core cannot check what a native key means, only that every item is accounted for: each
//! item of the model once, no item the model does not hold, and no item reported `not expressed`.
//! An opt-out of that last rule comes with TASK-127, so until then it is always the error.

use std::collections::{BTreeMap, BTreeSet};

use alz_adapter::{
    Coverage, DefaultModeFloor, DenyRule, Enforcement, FloorFieldItem, LockedHook, SecurityItemId,
    SecurityItems, SettingLayers, TargetId,
};

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, MessagePart, Where};
use crate::untrusted_text::{QuotedKey, Stripped};

/// The file that §8.0 names as the place of an item that a target cannot enforce.
const PLACE: &str = "permissions.yaml";

/// The errors `target` of a coverage report: each item of the model that it leaves out or names
/// twice, each item it names that the model does not hold, and each item it reports as not
/// expressed, once for each layer that set the item.
pub(super) fn check(target: &TargetId, security: &SecurityItems, report: &Coverage) -> Diagnostics {
    let reported = report.entries().iter().fold(
        BTreeMap::<&SecurityItemId, Vec<Enforcement>>::new(),
        |mut by_item, entry| {
            by_item
                .entry(&entry.item)
                .or_default()
                .push(entry.enforcement);
            by_item
        },
    );
    let held = security.ids().collect::<BTreeSet<_>>();
    let accounted = held
        .iter()
        .filter_map(|item| match reported.get(item).map_or(0, Vec::len) {
            1 => None,
            0 => Some(("leaves out", item)),
            _ => Some(("names twice", item)),
        })
        .map(|(fault, item)| {
            Message::new("the coverage report ")
                .then(fault)
                .then(" ")
                .and(describe(item))
        });
    let foreign = reported
        .keys()
        .filter(|item| !held.contains(**item))
        .map(|item| {
            Message::new("the coverage report names ")
                .and(describe(item))
                .then(", which the model does not hold")
        });
    accounted
        .chain(foreign)
        .map(|rule| error(target, rule))
        .chain(not_expressed(target, security, &reported))
        .fold(Diagnostics::new(), Diagnostics::with)
}

/// The errors for the items that the report states as not expressed.
fn not_expressed<'a>(
    target: &'a TargetId,
    security: &'a SecurityItems,
    reported: &'a BTreeMap<&SecurityItemId, Vec<Enforcement>>,
) -> impl Iterator<Item = Diagnostic> + 'a {
    let refused = move |item: SecurityItemId| {
        reported
            .get(&item)
            .is_some_and(|all| all.contains(&Enforcement::NotExpressed))
    };
    let deny_rules = security
        .deny_rules
        .iter()
        .filter(move |item| refused(SecurityItemId::DenyRule(item.rule.clone())))
        .flat_map(move |item| deny_rule(target, item));
    let floor_fields = security
        .floor_fields
        .iter()
        .filter(move |item| refused(SecurityItemId::FloorField(item.field)))
        .flat_map(move |item| floor_field(target, item));
    let default_mode = security
        .default_mode_floor
        .iter()
        .filter(move |_| refused(SecurityItemId::DefaultModeFloor))
        .flat_map(move |item| default_mode_floor(target, item));
    let locked_hooks = security
        .locked_hooks
        .iter()
        .filter(move |item| refused(SecurityItemId::LockedHook(item.id.clone())))
        .map(move |item| locked_hook(target, item));
    deny_rules
        .chain(floor_fields)
        .chain(default_mode)
        .chain(locked_hooks)
}

fn deny_rule<'a>(
    target: &'a TargetId,
    item: &'a DenyRule,
) -> impl Iterator<Item = Diagnostic> + 'a {
    each_layer(&item.set_by, move |layer| {
        cannot(
            target,
            Message::new("the deny rule '")
                .then(Stripped::from_untrusted(&item.rule))
                .then("' of layer ")
                .then(Stripped::from_untrusted(layer.as_str())),
            "list the rule under unenforced.",
            ".deny",
        )
    })
}

fn floor_field<'a>(
    target: &'a TargetId,
    item: &'a FloorFieldItem,
) -> impl Iterator<Item = Diagnostic> + 'a {
    each_layer(&item.set_by, move |layer| {
        cannot(
            target,
            Message::new("the floor field ")
                .then(Stripped::from_untrusted(&item.field.to_string()))
                .then(" of layer ")
                .then(Stripped::from_untrusted(layer.as_str())),
            "list the field under unenforced.",
            ".floor_fields",
        )
    })
}

fn default_mode_floor<'a>(
    target: &'a TargetId,
    item: &'a DefaultModeFloor,
) -> impl Iterator<Item = Diagnostic> + 'a {
    each_layer(&item.set_by, move |layer| {
        cannot(
            target,
            Message::new("the default_mode floor ")
                .then(Stripped::from_untrusted(&item.mode.to_string()))
                .then(" of layer ")
                .then(Stripped::from_untrusted(layer.as_str())),
            "set unenforced.",
            ".default_mode to true",
        )
    })
}

fn locked_hook(target: &TargetId, item: &LockedHook) -> Diagnostic {
    located(
        &item.defined_by,
        cannot(
            target,
            Message::new("the locked hook ")
                .then(QuotedKey::from_untrusted(&item.id))
                .then(" of layer ")
                .then(Stripped::from_untrusted(item.defined_by.as_str())),
            "list the hook under unenforced.",
            ".hooks",
        ),
    )
}

/// One error for each layer of `layers`, at its `permissions.yaml`.
fn each_layer<'a>(
    layers: &'a SettingLayers,
    message: impl Fn(&alz_adapter::LayerName) -> Message + 'a,
) -> impl Iterator<Item = Diagnostic> + 'a {
    layers
        .iter()
        .map(move |layer| located(layer, message(layer)))
}

fn located(layer: &alz_adapter::LayerName, message: Message) -> Diagnostic {
    Diagnostic::error(
        Code::Target,
        Some(Where::layer_file(
            Stripped::from_untrusted(layer.as_str()),
            Stripped::from_untrusted(PLACE),
        )),
        message,
    )
}

/// `the '<target>' adapter cannot express <what>; remove the target, or <how>` where `how` is the
/// text before the target, the target and the text after it, then ` in that layer's
/// permissions.yaml`.
fn cannot(target: &TargetId, what: Message, before: &'static str, after: &'static str) -> Message {
    Message::new("the '")
        .then(MessagePart::printable(target))
        .then("' adapter cannot express ")
        .and(what)
        .then("; remove the target, or ")
        .then(before)
        .then(MessagePart::printable(target))
        .then(after)
        .then(" in that layer's permissions.yaml")
}

/// The error `target` for a coverage report that breaks `rule`.
fn error(target: &TargetId, rule: Message) -> Diagnostic {
    Diagnostic::error(
        Code::Target,
        None,
        Message::new("the '")
            .then(MessagePart::printable(target))
            .then("' adapter returned output that is refused: ")
            .and(rule),
    )
}

/// An item as a message names it.
fn describe(item: &SecurityItemId) -> Message {
    match item {
        SecurityItemId::DenyRule(rule) => Message::new("the deny rule '")
            .then(Stripped::from_untrusted(rule))
            .then("'"),
        SecurityItemId::FloorField(field) => {
            Message::new("the floor field ").then(Stripped::from_untrusted(&field.to_string()))
        }
        SecurityItemId::DefaultModeFloor => Message::new("the default_mode floor"),
        SecurityItemId::LockedHook(id) => {
            Message::new("the locked hook ").then(QuotedKey::from_untrusted(id))
        }
    }
}
