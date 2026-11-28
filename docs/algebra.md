# The algebra of agent-layerz

`alz` merges layers with a small set of algebraic structures. Each one has written laws, and a
property test checks each law on thousands of generated inputs. This page states the
structures, their laws, what each law buys you, and where its test lives. The rules themselves
are in [LAYERING-SPEC §7](LAYERING-SPEC.md#7-merge-rules); the code is in
[`crates/alz-core/src/merge/algebra/`](../crates/alz-core/src/merge/algebra/).

## The model

A stack is a list of layers, most general first: personal, then client, then repo.

```text
L₁, L₂, …, Lₙ          (L₁ most general, Lₙ the repo layer)
```

For each item type (agents, skills, permissions, settings and the rest), every layer contributes
a value vᵢ, possibly empty. Each type has a binary operation `⊕`, written
`earlier.merge(later)` in the code, where the right operand is the more specific value. The
merged item is a left fold:

```text
v = e ⊕ v₁ ⊕ v₂ ⊕ … ⊕ vₙ        e is the identity: "this layer says nothing"
```

The build is then a pure function of its inputs: the layers' content and the pinned revisions
in `layer.lock`.

```text
build : (layers, lock) → files                  same inputs, byte-identical files (§6)
check : build(layers, lock) = files in the repo  what `alz check` decides
```

## The structures

| Structure | `a ⊕ b` | Algebra | Laws under test | Used for |
| --- | --- | --- | --- | --- |
| [`Last<T>`](../crates/alz-core/src/merge/algebra/last.rs) | `b` when set, else `a` | the last-writer-wins monoid | associative; unset is the identity on both sides | an agent's scalar fields |
| [`Replace<K, V>`](../crates/alz-core/src/merge/algebra/replace.rs) | union of maps, `b`'s value wins per key | a monoid | associative; the empty map is the identity | skills, commands, MCP servers, model aliases, instruction blocks by file name, passthrough files by path |
| [`OrderedSet<T>`](../crates/alz-core/src/merge/algebra/ordered_set.rs) | union, keeping each item's first occurrence | an idempotent monoid; as a set, a join-semilattice | associative, idempotent, commutative as a set, the empty set is the identity, no item held twice | `allow`, `ask` and `deny` lists, a rule's paths, hook order |
| [`ListEdit<T>`](../crates/alz-core/src/merge/algebra/list_edit.rs) | composition of `+[…]`, `-[…]` and `[…]` edits | a monoid acting on lists | composition is associative; applying `a ⊕ b` equals applying `a` then `b`; the empty edit is the identity | an agent's tools and skills, permission list edits |
| [`Tree::deep_merge`](../crates/alz-core/src/merge/algebra/deep_merge.rs) | maps merge per key, lists by `OrderedSet` union, any other value: `b` wins | a monoid on kind-consistent trees | associative when every path keeps one kind; a tree merged with itself is settled | `settings.yaml`, an agent's native fields |
| [`Floor<T>`](../crates/alz-core/src/merge/algebra/floor.rs) | `max(a, b)` on a scale where larger is stricter | a bounded join-semilattice | associative, commutative, idempotent, unset is the identity, monotone | `default_mode` with `floor: true`, an overlay's `permission_mode`, `locked` hooks, floor settings |
| [`covers`](../crates/alz-core/src/merge/algebra/narrowing.rs) | not an operation: `a ≽ b` when tool entry `a` grants everything `b` does | a preorder | reflexive, transitive | narrowing an agent's tools in an overlay |

### Last-writer-wins: `Last` and `Replace`

`Last` is the simplest monoid there is: the right operand wins when it says anything. `Replace`
is the same rule applied key by key, so a repo layer's `review` skill replaces the personal
layer's `review` skill and leaves every other skill alone.

### Union: `OrderedSet`

Permission lists only grow down the stack, and listing a rule twice must not change the
result. That is idempotence (`a ⊕ a = a`). Read as plain sets, the lists form a
join-semilattice ordered by inclusion. The sequence order is kept for readable output: the
earlier list sets the order, so the operation is commutative as a set but not as a sequence.

### Edits: `ListEdit`

A layer may edit an inherited list instead of restating it: `+[a]` adds, `-[b]` removes, and
`[c]` replaces. Edits compose into one edit, and composition is a monoid action on lists:

```text
apply(a ⊕ b, xs) = apply(b, apply(a, xs))
```

So a stack of edits can be folded into one edit before it meets the base list, and the result is
the same as applying them one by one.

### Settings: `Tree::deep_merge`

Settings are trees. Maps merge key by key, lists are unioned, and any other value is replaced.
This is associative only when every path keeps the same kind (map, list or scalar) in all the
trees merged. The test
`a_path_that_changes_kind_is_not_associative_which_is_why_the_law_is_for_consistent_trees` shows
the counterexample. Take a list, then a scalar, then a list: the left fold gives the last list,
but merging the last two first gives both lists. The spec chooses the left fold, so the law is
stated for kind-consistent trees only.

### Security: `Floor`

Security settings sit on a scale ordered from loosest to strictest. For permission modes:

```text
bypass_permissions < auto < accept_edits < default < dont_ask < plan
```

`Floor` merges with `max`, the join of that order, so for every value `b` a later layer writes:

```text
a ⊕ b ≥ a                                             monotone: a layer can tighten, never loosen
```

This one inequality is why a repo layer cannot loosen a personal layer's permission floor or
unlock its locked hook. A deny rule is never removed downstream either: the permissions merge
refuses an edit that drops one. Both are property tests:

- `a_more_specific_layer_cannot_loosen_the_deny_rules_or_the_default_mode`
- `merging_a_further_layer_either_refuses_or_keeps_the_denies_and_the_floor`

They are in [`merge/permissions/laws.rs`](../crates/alz-core/src/merge/permissions/laws.rs).

### Narrowing: `covers`

An overlay may narrow an agent's tools but not widen them. `covers(a, b)` holds when entry `a`
grants everything `b` does. For example, `docs/**` covers `docs/adr/**`, and `mise run *` covers
`mise run test` but not `mise run a; b`. The relation is a preorder: reflexive and transitive.
Transitivity matters. Each overlay checks its entries only against the agent it refines, and
transitivity makes the check hold for the whole chain of overlays. That is also why
`docs/**/**` covers only itself: the plain rule would break transitivity.

## Why the laws matter

| Law | What it gives you |
| --- | --- |
| Associativity | You can merge part of a stack on its own, such as a client hub's layers, and the result does not change. Bracketing never matters, so a long stack and a pre-merged one give the same merged items. |
| Identity | A layer that does not mention an item leaves it alone, and an empty layer is harmless. |
| Idempotence | Repeating a rule, a path or a hook changes nothing, so duplicates in sources cannot drift the output. |
| Commutativity as a set | The permissions you end up with do not depend on which layer listed a rule first; only the printed order does. |
| Monotonicity | Security only gets stricter down the stack. A repository cannot weaken what its owner or client requires. |
| Determinism | Output is a function of the inputs, so `alz check` in CI is an equation, not a heuristic. |

## How the laws are tested

Each law is a [`proptest`](https://proptest-rs.github.io/proptest/) property beside the
structure. proptest generates random values, checks the equation, and shrinks a failure to a
minimal counterexample. For example, the associativity of `Floor`:

```rust
proptest! {
    #[test]
    fn merging_is_associative(a in floors(), b in floors(), c in floors()) {
        prop_assert_eq!(
            a.clone().merge(b.clone()).merge(c.clone()),
            a.merge(b.merge(c)),
        );
    }
}
```

The merge of each item type then has its own law tests over whole layers. They are in
`merge/permissions/laws.rs`, `merge/settings/laws.rs` and `merge/passthrough/laws.rs`. The
acceptance tests of [spec §13](LAYERING-SPEC.md#13-acceptance-tests) check the same guarantees
end to end on fixture repositories. [spec-coverage.md](spec-coverage.md) maps each requirement
to its test.

## Further reading
