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
