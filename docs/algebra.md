# The algebra of agent-layerz

Most of what a layer can set merges through a small algebraic structure with written laws, and
a property test checks each law on generated inputs. Where a law alone cannot keep the stack
safe, the merge also refuses some values: it stops with an error instead of loosening anything.
This page states the structures and their laws, says what each law gives you and where its test
is, and lists the exceptions. The rules themselves are in
[LAYERING-SPEC §7](LAYERING-SPEC.md#7-merge-rules), and the code is in
[`crates/alz-core/src/merge/`](../crates/alz-core/src/merge/).

## The model

A stack is a list of layers, most general first: personal, then client, then repo.

```text
L₁, L₂, …, Lₙ          (L₁ most general, Lₙ the repo layer)
```

For an item that merges through a structure, every layer contributes a value vᵢ, possibly empty.
The structure has a binary operation `⊕`, written `earlier.merge(later)` in the code, whose right
operand is the more specific value. The merged item is a left fold:

```text
v = e ⊕ v₁ ⊕ v₂ ⊕ … ⊕ vₙ        e is the identity: "this layer says nothing"
```

The build as a whole is a pure function of its inputs: the layers' content and the revisions
pinned in `layer.lock`.

```text
build : (layers, lock) → files       same inputs give byte-identical files (spec §6)
check : files in the repo = build(layers, lock)
        and the git identity matches the declared one, unless --skip-identity
```

## The structures

| Structure | `a ⊕ b` | Algebra | Laws under test | Used for |
| --- | --- | --- | --- | --- |
| [`Last<T>`](../crates/alz-core/src/merge/algebra/last.rs) | `b` when set, else `a` | the last-writer-wins monoid | associative; unset is the identity on both sides | an agent's scalar fields |
| [`Replace<K, V>`](../crates/alz-core/src/merge/algebra/replace.rs) | union of maps, `b`'s value wins per key | a monoid | associative; the empty map is the identity on both sides | skills, commands, MCP servers, model aliases, passthrough files by path |
| [`OrderedSet<T>`](../crates/alz-core/src/merge/algebra/ordered_set.rs) | union, keeping each item's first occurrence | an idempotent monoid; as a set, a join-semilattice | associative, idempotent, commutative as a set, the empty set is the identity on both sides, no item held twice | `allow`, `ask` and `deny` lists, a rule's paths, the lists inside settings |
| [`ListEdit<T>`](../crates/alz-core/src/merge/algebra/list_edit.rs) | the edit that does `a`, then `b` | a monoid of `+[…]`, `-[…]` and `[…]` edits that acts on lists without duplicates | composition is associative; applying `a ⊕ b` equals applying `a`, then `b`; the empty edit is the identity up to applying it | an agent's tools and skills, edits of the permission lists |
| [`RuleChain`](../crates/alz-core/src/merge/rules.rs) | a defining file discards what came before; an extending file adds to it | a monoid | associative, with an identity | path-scoped rules |
| [`Tree::deep_merge`](../crates/alz-core/src/merge/algebra/deep_merge.rs) | maps merge per key, lists by `OrderedSet` union, any other value: `b` wins | a semigroup on trees of one shape | associative when every path keeps one kind; at the top of `settings.yaml`, the empty settings are the identity; merging a layer twice changes nothing more | `settings.yaml` |
| [`NativeMap`](../crates/alz-core/src/item/agent/ordered_tree.rs) | the same deep merge, keeping the order in which keys first appeared | a semigroup | associative, in value and in key order | an agent's native fields (`claude:`, `codex:`) |
| [`Floor<T>`](../crates/alz-core/src/merge/algebra/floor.rs) | `max(a, b)` on an order where larger is stricter | a bounded join-semilattice | associative, commutative, idempotent, unset is the identity, monotone | `default_mode` with `floor: true` |
| [`covers`](../crates/alz-core/src/merge/algebra/narrowing.rs) | not an operation: `a ≽ b` when tool entry `a` grants everything `b` does | a preorder | reflexive, transitive | the tools an overlay may leave an agent |

### Last-writer-wins: `Last` and `Replace`

`Last` keeps the right operand when it says anything. `Replace` applies the same rule key by key,
so a repo layer's `review` skill replaces the personal layer's `review` skill and leaves every
other skill alone.

### Union: `OrderedSet`

Listing a rule twice must not change the result. That is idempotence (`a ⊕ a = a`). Read as
plain sets, the lists form a join-semilattice ordered by inclusion. The sequence order is kept
for readable output: the earlier list sets the order, so the operation is commutative as a set
but not as a sequence.

### Edits: `ListEdit`

A layer may edit an inherited list instead of restating it: `+[a]` adds, `-[b]` removes and
`[c]` replaces. Edits form a monoid that acts on lists without duplicates:

```text
apply(a ⊕ b, xs) = apply(b, apply(a, xs))
```

So a stack of edits can be folded into one edit before it meets the base list, and the result is
the same as applying them one by one. The test of the identity compares the results of applying
edits, not the edits themselves.

### Rules: `RuleChain`

A path-scoped rule is named by its file. A layer that defines the rule discards everything
before it; a layer with `mode: extend` appends its body and adds its globs. Each layer's say on
one rule is a `RuleChain`, and their merge is associative with an identity.

### Settings: `Tree::deep_merge`

Settings are trees. Maps merge key by key, lists are unioned, and any other value is replaced.
Three laws hold, each within limits:

- **Associativity**, when every path keeps the same kind (map, list or scalar) in all the trees
  merged. The test
  `a_path_that_changes_kind_is_not_associative_which_is_why_the_law_is_for_consistent_trees`
  shows the counterexample. Take a list, then a scalar, then a list: the left fold gives the last
  list, but merging the last two first gives both lists. The spec chooses the left fold, so the
  law is stated for trees of one shape.
- **An identity** only at the top of `settings.yaml`, where the empty settings change nothing.
  A scalar has no identity.
- **Merging a layer twice changes nothing more**: `(a ⊕ b) ⊕ b = a ⊕ b`, for lists without
  repeated items. Plain idempotence fails when a list repeats an item, since `[1, 1] ⊕ [1, 1]`
  is `[1]`. Once a tree has been merged with itself it is settled:
  `(t ⊕ t) ⊕ (t ⊕ t) = t ⊕ t`.

An agent's native fields merge the same way through `NativeMap`, which also keeps the order in
which keys first appeared, so generated files list them in a stable order.

### Security: `Floor`, and refusing what loosens

Permission modes are ordered from loosest to strictest:

```text
bypass_permissions < auto < accept_edits < default < dont_ask < plan
```

A `default_mode` declared with `floor: true` is a `Floor`. Its merge is `max`, the join of that
order, so for every later value `b`:

```text
a ⊕ b ≥ a            monotone: the floor can rise, never fall
```

A join on its own would quietly absorb a looser value. That is not what a person who set a
floor wants to happen, so wherever security is at stake the merge refuses instead. A more
specific layer that tries to loosen any of these gets a build error that names the layer:

| What a layer tries | Refused by | Property test |
| --- | --- | --- |
| A `default_mode` looser than a floor | `permission-floor` error | `a_more_specific_layer_cannot_loosen_the_deny_rules_or_the_default_mode` |
| Removing an inherited deny rule | the permissions merge | `merging_a_further_layer_either_refuses_or_keeps_the_denies_and_the_floor` |
| Clearing a permission floor flag once set | the permissions merge | `a_flag_that_is_true_stays_true_down_the_stack` |
| Disabling a locked hook | the hooks merge | `no_later_layer_can_disable_a_locked_hook` |
| Loosening a floor key in `settings.yaml` | the settings merge | `no_floor_key_loosens_down_the_stack` |
| An overlay's `permission_mode` looser than its agent's | the agents merge | `an_overlay_never_loosens_the_permission_mode` |

So the merge is partial: defined when the more specific layer keeps every floor, and an error
otherwise. On the values where it is defined, security only gets stricter down the stack.

### Narrowing: `covers`

An overlay may narrow an agent's tools but not widen them, unless it sets `allow_widen: true`,
which a public layer may not. `covers(a, b)` holds when entry `a` grants everything `b` does. For
example, `docs/**` covers `docs/adr/**`, and `mise run *` covers `mise run test` but not
`mise run a; b`. The relation is a preorder: reflexive and transitive. Transitivity matters:
each overlay checks its entries only against the agent it refines, and transitivity makes the
check hold for the whole chain of overlays. It is also why `docs/**/**` covers only itself,
because the plain rule would break transitivity.

## What is not a single structure

Some items merge by rules that no one structure states:

- **Instructions** are concatenated in stack order, each layer's blocks under that layer's
  heading. A file with `mode: replace` or `mode: remove` acts on the blocks of more general layers
  with the same file name, and a file without `mode` adds a block.
- **Hooks** are entries and disables. A later layer may disable a hook a more general one
  defined, a layer may not disable its own, and an id defined twice in one layer is an error.
- **Commit identity** comes from the first layer, or wholly from one private client layer
  (spec §7.12).

These are why the laws are stated per structure, not for whole layers. Merging two layers into
one before the rest of the stack can change the result: a hook defined in one and disabled in the
other becomes a layer that disables its own hook, which is an error.

## Why the laws matter

| Law | What it gives you |
| --- | --- |
| Associativity | Folding a structure's values in any grouping gives the same value, so the code is free to fold a stack step by step or all at once. |
| Identity | A layer that says nothing about an item leaves it alone. |
| Idempotence | A repeated rule or path changes nothing, so duplicates in sources cannot change the output. |
| Commutativity as a set | When layers only add rules, the permissions you end up with do not depend on which layer listed a rule first; only the printed order does. With edits, order matters: `[a]` then `+[b]` gives both rules, and `+[b]` then `[a]` gives `a` only. |
| Monotonicity, with refusal | Security only gets stricter down the stack. A repository cannot weaken what its owner or client requires, and an attempt is an error, not a silent change. |
| Determinism | Output is a function of the inputs, so `alz check` in CI is an equation, not a heuristic. |

## How the laws are tested

Each law is a [`proptest`](https://proptest-rs.github.io/proptest/) property beside the
structure. proptest generates random values (256 cases per property by default), checks the
equation, and shrinks a failure to a minimal counterexample. For example, the associativity of
`Floor`:

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

The merge of each item type has its own property tests over whole layers too, for example in
`merge/permissions/laws.rs`, `merge/settings/laws.rs`, `merge/passthrough/laws.rs`,
`merge/hooks/tests.rs`, `merge/rules/tests.rs` and `merge/agents/overlay_strictness.rs`. The
acceptance tests of [spec §13](LAYERING-SPEC.md#13-acceptance-tests) check the same guarantees
end to end on fixture repositories, and [spec-coverage.md](spec-coverage.md) maps each
requirement to its test.

## Further reading

A path from practice to theory. Each entry says what it adds to this page.

### Start here: laws as tests

- John Hughes, *How to Specify It! A Guide to Writing Properties of Pure Functions* (TFP 2019,
  LNCS 12053, 2020). <https://doi.org/10.1007/978-3-030-47147-7_4>. Five ways to find the
  properties of a pure function, compared by cost and by the bugs they find.
- Koen Claessen, John Hughes, *QuickCheck: A Lightweight Tool for Random Testing of Haskell
  Programs* (ICFP 2000). <https://doi.org/10.1145/351240.351266>. The paper that started
  property-based testing; proptest descends from it.
- *The Proptest Book*. <https://proptest-rs.github.io/proptest/>. Strategies, shrinking and
  recursive values: what the law tests here use to generate layers and trees.
- Fred Hebert, *Property-Based Testing with PropEr, Erlang, and Elixir* (Pragmatic Bookshelf,
  2019). <https://pragprog.com/titles/fhproper/property-based-testing-with-proper-erlang-and-elixir/>.
  Designing generators and finding properties; the ideas carry over to proptest.
- Sandy Maguire, *Algebra-Driven Design* (Leanpub, 2020). <https://leanpub.com/algebra-driven-design>.
  Design an abstraction by its laws first, then derive the code and its tests from them.

### Monoids and orders

- Brent A. Yorgey, *Monoids: Theme and Variations* (Haskell Symposium 2012).
  <https://doi.org/10.1145/2364506.2364520>, free copy <https://core.ac.uk/download/76383233.pdf>.
  Building a library from monoids, including a monoid acting on another structure, the shape of
  `ListEdit`.
- B. A. Davey, H. A. Priestley, *Introduction to Lattices and Order*, 2nd ed. (Cambridge
  University Press, 2002). <https://doi.org/10.1017/CBO9780511809088>. Partial orders,
  semilattices and monotone maps: the mathematics of `Floor` and `covers`.
- Brendan Fong, David I. Spivak, *Seven Sketches in Compositionality* (2018; Cambridge
  University Press, 2019). <https://arxiv.org/abs/1803.05316>. Chapter 1 covers preorders, joins,
  monotone maps and Galois connections.
- Bartosz Milewski, *Category Theory for Programmers* (2014–2019).
  <https://bartoszmilewski.com/2014/10/28/category-theory-for-programmers-the-preface/>, free PDF
  <https://github.com/hmemcpy/milewski-ctfp-pdf>. Monoids, functors and composition, for
  programmers.

### Merging that converges

- Marc Shapiro, Nuno Preguiça, Carlos Baquero, Marek Zawirski, *Conflict-Free Replicated Data
  Types* (SSS 2011). <https://doi.org/10.1007/978-3-642-24550-3_29>, with the report *A
  comprehensive study of Convergent and Commutative Replicated Data Types*
  <https://hal.inria.fr/inria-00555588>. Merges that are joins on a semilattice converge: the
  laws of `OrderedSet` and `Floor`.
- Paulo Sérgio Almeida, *Approaches to Conflict-free Replicated Data Types* (ACM Computing
  Surveys, 2024). <https://arxiv.org/abs/2310.18220>. A current survey of state-based merges and
  how to compose them.
- Martin Kleppmann, Alastair R. Beresford, *A Conflict-Free Replicated JSON Datatype* (IEEE TPDS,
  2017). <https://arxiv.org/abs/1608.03960>. A deterministic merge of nested maps and lists, a
  relative of `Tree::deep_merge`.
- Lindsey Kuper, Ryan R. Newton, *LVars: Lattice-based Data Structures for Deterministic
  Parallelism* (FHPC 2013). <https://doi.org/10.1145/2502323.2502326>. Writes that only move up a
  lattice give deterministic results.
- Neil Conway, William R. Marczak, Peter Alvaro, Joseph M. Hellerstein, David Maier, *Logic and
  Lattices for Distributed Programming* (SoCC 2012). <https://doi.org/10.1145/2391229.2391230>.
  Programs built from lattices and monotone functions between them.
- Joseph M. Hellerstein, Peter Alvaro, *Keeping CALM: When Distributed Consistency is Easy*
  (CACM, 2020). <https://arxiv.org/abs/1901.01930>. Why monotone programs need no coordination.

### Deterministic builds

- Andrey Mokhov, Neil Mitchell, Simon Peyton Jones, *Build Systems à la Carte* (ICFP 2018).
  <https://doi.org/10.1145/3236774>. Build systems as pure functions over a store.
- Eelco Dolstra, *The Purely Functional Software Deployment Model* (PhD thesis, Utrecht, 2006).
  <https://dspace.library.uu.nl/handle/1874/7540>. The work behind Nix: an output is a function
  of its declared inputs.
- Reproducible Builds. <https://reproducible-builds.org/>. Practical sources of
  non-determinism and how to remove them.

### Related configuration systems

- *The Logic of CUE*. <https://cuelang.org/docs/concept/the-logic-of-cue/>. Every value sits in
  one lattice and merging is the meet, so order never matters. Comparing it with this page
  shows where `alz` differs: here the more specific layer wins.
- NixOS Manual, *Option Definitions*. <https://nixos.org/manual/nixos/stable/#sec-option-definitions>.
  Priorities, ordering and per-type merges in a large layered configuration.
- Paul Hoffman, James M. Snell, *RFC 7396: JSON Merge Patch* (2014).
  <https://www.rfc-editor.org/rfc/rfc7396>. The common deep-merge rule, where arrays are
  replaced whole; `alz` unions them instead.
- Scott Wlaschin, *Domain Modeling Made Functional* (Pragmatic Bookshelf, 2018).
  <https://pragprog.com/titles/swdddf/domain-modeling-made-functional/>. Types that make invalid
  states unrepresentable, and a pure core with effects at the edges, as `alz-core` and `alz-io`
  are split.

### Courses

- David I. Spivak, Brendan Fong, *18.S097 Applied Category Theory* (MIT OpenCourseWare, 2019).
  <https://ocw.mit.edu/courses/18-s097-applied-category-theory-january-iap-2019/>. Lectures that
  follow *Seven Sketches*, with problem sets.
- Brendan Fong, Bartosz Milewski, David I. Spivak, *Programming with Categories* (MIT, 2020).
  <http://brendanfong.com/programmingcats.html>, lectures
  <https://www.youtube.com/playlist?list=PLhgq-BqyZ7i7MTGhUROZy3BOICnVixETS>. Category theory
  through programming, with no background assumed.
- Bartosz Milewski, *Category Theory*, Part I (lectures).
  <https://www.youtube.com/playlist?list=PLbgaMIhjbmEnaH_LTkxLI7FMa2HsnawM_>.

### Beyond sampling: proofs

- *The Kani Rust Verifier*. <https://model-checking.github.io/kani/>. A model checker that proves
  a property for all inputs within a bound, where proptest samples them. The laws on this page
  are candidates for Kani proofs.
