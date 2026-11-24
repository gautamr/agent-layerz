//! The laws of the merge of permissions: security only gets stricter down the stack (spec §7.6,
//! §9.3), and the lists are unions.

use proptest::collection::vec;
use proptest::prelude::{
    Strategy, any, prop_assert, prop_assert_eq, prop_assume, prop_oneof, proptest,
};

use crate::diagnostic::{Code, Level, Where};
use crate::item::{
    Binding, DefaultMode, FloorFlag, PermissionItems, PermissionMode, PermissionRule, Permissions,
};
use crate::layer::LayerName;
use crate::merge::algebra::{ListEdit, OrderedSet};
use crate::merge::testing::{layer, merged, origin_of};
use crate::stack::ResolvedLayer;
use crate::untrusted_text::Stripped;

use super::PermissionState;
use super::tests::{permissions_layer, written};

const RULES: [&str; 5] = [
    "read:.env*",
    "bash:rm -rf *",
    "read",
    "bash:ls",
    "write:docs/**",
];

const MODES: [PermissionMode; 6] = [
    PermissionMode::BypassPermissions,
    PermissionMode::Auto,
    PermissionMode::AcceptEdits,
    PermissionMode::Default,
    PermissionMode::DontAsk,
    PermissionMode::Plan,
];

fn rule(index: usize) -> PermissionRule {
    PermissionRule::try_from(RULES.get(index).copied().unwrap_or("read")).expect("a rule")
}

fn rule_sets() -> impl Strategy<Value = OrderedSet<PermissionRule>> {
    vec(0..RULES.len(), 0..5).prop_map(|indexes| indexes.into_iter().map(rule).collect())
}

fn edits() -> impl Strategy<Value = ListEdit<PermissionRule>> {
    prop_oneof![
        rule_sets().prop_map(ListEdit::Replace),
        (rule_sets(), rule_sets()).prop_map(|(remove, add)| ListEdit::Adjust { remove, add }),
    ]
}

fn modes() -> impl Strategy<Value = Option<DefaultMode>> {
    proptest::option::of((0..MODES.len(), any::<bool>()).prop_map(|(index, floor)| {
        DefaultMode::new(
            MODES.get(index).copied().unwrap_or(PermissionMode::Default),
            if floor { Binding::Floor } else { Binding::Free },
        )
    }))
}

fn files() -> impl Strategy<Value = Permissions> {
    (edits(), edits(), edits(), modes())
        .prop_map(|(allow, ask, deny, mode)| Permissions::new(allow, ask, deny, mode))
}

fn name(index: usize) -> LayerName {
    LayerName::try_from(format!("l{index}").as_str()).expect("a layer name")
}

fn at() -> Where {
    Where::layer_file(
        Stripped::from_untrusted("l"),
        Stripped::from_untrusted("permissions.yaml"),
    )
}

fn fold(files: &[Permissions]) -> PermissionState {
    files
        .iter()
        .enumerate()
        .fold(PermissionState::default(), |state, (index, file)| {
            state.apply(file, &origin_of(name(index)), &at()).0
        })
}

/// The text of an edit as a string of segments.
fn edit_text(edit: &ListEdit<PermissionRule>) -> String {
    let segment = |sign: &str, set: &OrderedSet<PermissionRule>| {
        if set.is_empty() {
            String::new()
        } else {
            format!(
                "{sign}[{}]",
                set.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    };
    match edit {
        ListEdit::Replace(set) => format!("[{}]", segment("", set).trim_matches(['[', ']'])),
        // An edit of white space alone is no edit (the reference refuses it), so an empty segment
        // adds no separator.
        ListEdit::Adjust { remove, add } => [segment("-", remove), segment("+", add)]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// The `permissions.yaml` that says what `file` says.
fn render(file: &Permissions) -> String {
    let mode = file.default_mode().map_or_else(String::new, |mode| {
        format!(
            "default_mode: {}\nfloor: {}\n",
            mode.mode().as_str(),
            mode.binding() == Binding::Floor
        )
    });
    format!(
        "allow: \"{}\"\nask: \"{}\"\ndeny: \"{}\"\n{mode}",
        edit_text(file.allow()),
        edit_text(file.ask()),
        edit_text(file.deny()),
    )
}

fn deny_of(state: PermissionState) -> Vec<String> {
    written(state.finish().deny())
}

proptest! {
    #[test]
    fn a_file_read_from_its_text_is_the_file(file in files()) {
        let layer = permissions_layer("repo", &render(&file));
        let (items, found) = PermissionItems::read(&layer);
        prop_assert_eq!(found.iter().count(), 0);
        let (_, read) = items.into_file().expect("a file");
        prop_assert_eq!(read, file);
    }

    /// Whatever a more specific layer says, the deny rules and the floor of the stack stay, the
    /// mode is never looser than the floor, and the layer is refused exactly when it tried to
    /// loosen.
    #[test]
    fn a_more_specific_layer_cannot_loosen_the_deny_rules_or_the_default_mode(
        stack in vec(files(), 0..5),
        later in files(),
    ) {
        let before = fold(&stack);
        let (after, found) = before.clone().apply(&later, &origin_of(name(9)), &at());

        let denied_before = deny_of(before.clone());
        let denied_after = deny_of(after.clone());
        prop_assert!(denied_before.iter().all(|rule| denied_after.contains(rule)));

        let floor_before = before.clone().finish().floor();
        let merged_after = after.finish();
        prop_assert!(merged_after.floor() >= floor_before);
        prop_assert!(
            merged_after.floor().is_none()
                || merged_after
                    .default_mode()
                    .is_some_and(|mode| Some(*mode.value()) >= merged_after.floor())
        );

        let before_deny = before.clone().finish();
        let members: OrderedSet<PermissionRule> =
            before_deny.deny().iter().map(|held| held.value().clone()).collect();
        let tried_to_drop = !members.clone().without(&later.deny().apply(&members)).is_empty();
        let tried_looser = later
            .default_mode()
            .is_some_and(|mode| floor_before.is_some_and(|floor| mode.mode() < floor));
        prop_assert_eq!(found.has_errors(), tried_to_drop || tried_looser);
    }

    /// The same law through the text of the files and the whole merge: either the merge refuses,
    /// and only for the floor, or it keeps every deny rule of the stack and a mode no looser than
    /// the floor.
    #[test]
    fn merging_a_further_layer_either_refuses_or_keeps_the_denies_and_the_floor(
        stack in vec(files(), 0..4),
        later in files(),
    ) {
        let layers = |files: &[Permissions]| -> Vec<ResolvedLayer> {
            files
                .iter()
                .enumerate()
                .map(|(index, file)| permissions_layer(&format!("l{index}"), &render(file)))
                .collect()
        };
        let base = merged(layers(&stack));
        prop_assume!(base.merged().is_some());
        let base = base.merged().expect("no error").permissions().clone();

        let every: Vec<Permissions> = stack.iter().cloned().chain([later]).collect();
        let found = merged(layers(&every));
        match found.merged() {
            None => prop_assert!(
                found
                    .diagnostics()
                    .iter()
                    .filter(|problem| problem.level() == Level::Error)
                    .all(|problem| problem.code() == Code::PermissionFloor)
            ),
            Some(model) => {
                let permissions = model.permissions();
                let denied = written(permissions.deny());
                prop_assert!(written(base.deny()).iter().all(|rule| denied.contains(rule)));
                prop_assert!(permissions.floor() >= base.floor());
                prop_assert!(
                    base.floor().is_none()
                        || permissions
                            .default_mode()
                            .is_some_and(|mode| Some(*mode.value()) >= base.floor())
                );
            }
        }
        // Determinism: the same stack gives the same result.
        prop_assert_eq!(found, merged(layers(&every)));
    }
}

/// The plain lists of a layer: the rules it adds to each list, as indexes into [`RULES`].
type Lists = (Vec<usize>, Vec<usize>, Vec<usize>);

fn lists() -> impl Strategy<Value = Lists> {
    let indexes = || vec(0..RULES.len(), 0..4);
    (indexes(), indexes(), indexes())
}

fn plain(name: &str, (allow, ask, deny): &Lists) -> ResolvedLayer {
    let text = |indexes: &[usize]| {
        indexes
            .iter()
            .map(|index| format!("\"{}\"", rule(*index)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    layer(
        name,
        "client",
        &[(
            "permissions.yaml",
            &format!(
                "allow: [{}]\nask: [{}]\ndeny: [{}]\n",
                text(allow),
                text(ask),
                text(deny)
            ),
        )],
    )
}

fn union(first: &Lists, second: &Lists) -> Lists {
    let both = |a: &[usize], b: &[usize]| a.iter().chain(b).copied().collect::<Vec<_>>();
    (
        both(&first.0, &second.0),
        both(&first.1, &second.1),
        both(&first.2, &second.2),
    )
}

/// The three merged lists, as written.
fn merged_lists(layers: Vec<ResolvedLayer>) -> (Vec<String>, Vec<String>, Vec<String>) {
    let found = merged(layers);
    let permissions = found.merged().expect("no error was found").permissions();
    (
        written(permissions.allow()),
        written(permissions.ask()),
        written(permissions.deny()),
    )
}

proptest! {
    /// Grouping the layers differently gives the same lists: the union is associative.
    #[test]
    fn the_lists_combine_associatively(a in lists(), b in lists(), c in lists()) {
        let apart = merged_lists(vec![plain("l0", &a), plain("l1", &b), plain("l2", &c)]);
        let left = merged_lists(vec![plain("l0", &union(&a, &b)), plain("l1", &c)]);
        let right = merged_lists(vec![plain("l0", &a), plain("l1", &union(&b, &c))]);
        prop_assert_eq!(&apart, &left);
        prop_assert_eq!(&apart, &right);
    }

    /// A layer that says what an earlier layer said again adds nothing: the union is idempotent.
    #[test]
    fn the_lists_combine_idempotently(a in lists()) {
        prop_assert_eq!(
            merged_lists(vec![plain("l0", &a), plain("l1", &a)]),
            merged_lists(vec![plain("l0", &a)])
        );
    }

    /// The order of the layers changes only the order of the rules, not which are held.
    #[test]
    fn the_lists_hold_the_same_rules_in_either_order(a in lists(), b in lists()) {
        let same = |x: &[String], y: &[String]| {
            x.iter().all(|rule| y.contains(rule)) && y.iter().all(|rule| x.contains(rule))
        };
        let forward = merged_lists(vec![plain("l0", &a), plain("l1", &b)]);
        let backward = merged_lists(vec![plain("l0", &b), plain("l1", &a)]);
        prop_assert!(same(&forward.0, &backward.0));
        prop_assert!(same(&forward.1, &backward.1));
        prop_assert!(same(&forward.2, &backward.2));
    }
}

/// A file that sets each flag of `settings` that is `Some`.
fn flag_file(settings: [Option<bool>; 3]) -> Permissions {
    FloorFlag::ALL.into_iter().zip(settings).fold(
        Permissions::new(
            ListEdit::default(),
            ListEdit::default(),
            ListEdit::default(),
            None,
        ),
        |file, (flag, value)| value.map_or(file.clone(), |value| file.with_flag(flag, value)),
    )
}

proptest! {
    #[test]
    fn a_flag_that_is_true_stays_true_down_the_stack(
        stack in vec(proptest::array::uniform3(proptest::option::of(any::<bool>())), 0..6),
    ) {
        stack.iter().enumerate().try_fold(PermissionState::default(), |before, (index, settings)| {
            let (after, _) = before.clone().apply(&flag_file(*settings), &origin_of(name(index)), &at());
            for flag in FloorFlag::ALL {
                if before.clone().finish().is_set(flag) {
                    prop_assert!(after.clone().finish().is_set(flag), "{flag:?}");
                }
            }
            Ok(after)
        })?;
    }
}
