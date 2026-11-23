use proptest::prelude::{prop_assert_eq, proptest};

use crate::merge::provenance::Action;
use crate::merge::testing::{layer, lines, merged};
use crate::merge::{InstructionBlock, Merged};
use crate::stack::ResolvedLayer;

// The files of acceptance case 02 (§13.2), read from the fixture itself.
const PERSONAL_CORE: &str =
    include_str!("../../../../../test/fixtures/02-instructions/personal/instructions/00-core.md");
const PERSONAL_STYLE: &str =
    include_str!("../../../../../test/fixtures/02-instructions/personal/instructions/10-style.md");
const CLIENT_DOMAIN: &str =
    include_str!("../../../../../test/fixtures/02-instructions/client/instructions/10-domain.md");
const REPO_STYLE: &str = include_str!(
    "../../../../../test/fixtures/02-instructions/repo/.layer/instructions/10-style.md"
);
const REPO_OWN: &str = include_str!(
    "../../../../../test/fixtures/02-instructions/repo/.layer/instructions/50-repo.md"
);
const EXPECTED_AGENTS: &str =
    include_str!("../../../../../test/fixtures/02-instructions/expected/AGENTS.md");

fn merged_model(layers: Vec<ResolvedLayer>) -> Merged {
    let found = merged(layers);
    assert_eq!(lines(&found), Vec::<String>::new());
    found.merged().expect("no error was found").clone()
}

/// The blocks as `(heading, file, body)`.
fn blocks(model: &Merged) -> Vec<(String, String, String)> {
    model
        .instructions()
        .iter()
        .map(|traced| {
            let block: &InstructionBlock = traced.value();
            (
                block.heading().to_string(),
                block.file().as_str().to_owned(),
                block.body().to_owned(),
            )
        })
        .collect()
}

fn case_02() -> Vec<ResolvedLayer> {
    vec![
        layer(
            "personal",
            "personal",
            &[
                ("instructions/00-core.md", PERSONAL_CORE),
                ("instructions/10-style.md", PERSONAL_STYLE),
            ],
        ),
        layer(
            "acme",
            "client",
            &[("instructions/10-domain.md", CLIENT_DOMAIN)],
        ),
        layer(
            "shop-api",
            "repo",
            &[
                ("instructions/10-style.md", REPO_STYLE),
                ("instructions/50-repo.md", REPO_OWN),
            ],
        ),
    ]
}

// Acceptance case 02: "personal + client + repo give one AGENTS.md with three sections;
// mode: remove drops a block".
#[test]
fn personal_client_and_repo_give_three_blocks_in_stack_order_and_remove_drops_one() {
    let model = merged_model(case_02());
    let found = blocks(&model);
    assert_eq!(
        found,
        [
            (
                "## Personal: personal".to_owned(),
                "instructions/00-core.md".to_owned(),
                "- Never claim a check passed that you did not run.".to_owned()
            ),
            (
                "## Client: acme".to_owned(),
                "instructions/10-domain.md".to_owned(),
                "- Orders are immutable once invoiced.".to_owned()
            ),
            (
                "## Repo: shop-api".to_owned(),
                "instructions/50-repo.md".to_owned(),
                "- Run `mise run test` before declaring work done.".to_owned()
            ),
        ]
    );
    assert!(
        found
            .iter()
            .all(|block| !block.2.contains("Prefer short sentences"))
    );
}

// The same case, against the expected AGENTS.md of the fixture: its text after the header comment
// is the blocks, each under its heading.
#[test]
fn the_blocks_compose_the_text_of_the_expected_agents_md() {
    let composed = blocks(&merged_model(case_02()))
        .into_iter()
        .map(|(heading, _, body)| format!("{heading}\n\n{body}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let (_header, text) = EXPECTED_AGENTS
        .split_once("\n\n")
        .expect("a header, an empty line, then the text");
    assert_eq!(format!("{composed}\n"), text);
}

#[test]
fn the_removed_block_is_gone_and_the_others_keep_who_made_them() {
    let model = merged_model(case_02());
    let makers: Vec<Vec<(&str, Action)>> = model
        .instructions()
        .iter()
        .map(|block| {
            block
                .provenance()
                .contributions()
                .iter()
                .map(|part| (part.layer().as_str(), part.action()))
                .collect()
        })
        .collect();
    assert_eq!(
        makers,
        [
            vec![("personal", Action::Define)],
            vec![("acme", Action::Define)],
            vec![("shop-api", Action::Define)],
        ]
    );
}

#[test]
fn replace_puts_the_new_block_in_the_place_of_the_layer_that_replaced_it() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[
                ("instructions/00-core.md", "core"),
                ("instructions/10-style.md", "short"),
            ],
        ),
        layer(
            "shop-api",
            "repo",
            &[
                (
                    "instructions/10-style.md",
                    "---\nmode: replace\n---\nlong\n",
                ),
                ("instructions/50-repo.md", "own"),
            ],
        ),
    ]);
    assert_eq!(
        blocks(&model),
        [
            (
                "## Personal: personal".to_owned(),
                "instructions/00-core.md".to_owned(),
                "core".to_owned()
            ),
            (
                "## Repo: shop-api".to_owned(),
                "instructions/10-style.md".to_owned(),
                "long".to_owned()
            ),
            (
                "## Repo: shop-api".to_owned(),
                "instructions/50-repo.md".to_owned(),
                "own".to_owned()
            ),
        ]
    );
    let replaced = model.instructions().get(1).expect("the replacement");
    let makers: Vec<(&str, Action)> = replaced
        .provenance()
        .contributions()
        .iter()
        .map(|part| (part.layer().as_str(), part.action()))
        .collect();
    assert_eq!(
        makers,
        [("personal", Action::Define), ("shop-api", Action::Replace)]
    );
}

#[test]
fn a_file_without_mode_adds_a_block_even_when_a_more_general_layer_has_the_name() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[("instructions/10-style.md", "one")],
        ),
        layer("shop-api", "repo", &[("instructions/10-style.md", "two")]),
    ]);
    let bodies: Vec<String> = blocks(&model).into_iter().map(|block| block.2).collect();
    assert_eq!(bodies, ["one", "two"]);
}

#[test]
fn a_replace_or_remove_without_a_block_to_act_on_is_unresolved() {
    let found = merged(vec![
        layer(
            "personal",
            "personal",
            &[("instructions/00-core.md", "core")],
        ),
        layer(
            "shop-api",
            "repo",
            &[
                ("instructions/10-gone.md", "---\nmode: remove\n---\n"),
                ("instructions/20-new.md", "---\nmode: replace\n---\ntext"),
            ],
        ),
    ]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [unresolved] shop-api:instructions/10-gone.md: mode: remove, but no more general layer has instructions/10-gone.md",
            "error [unresolved] shop-api:instructions/20-new.md: mode: replace, but no more general layer has instructions/20-new.md",
        ]
    );
}

#[test]
fn an_unknown_field_and_an_unknown_mode_are_instructions_errors_that_name_the_file() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[
            ("instructions/00-fields.md", "---\ncolour: red\n---\ntext"),
            ("instructions/10-mode.md", "---\nmode: prepend\n---\ntext"),
            ("instructions/20-add.md", "---\nmode: add\n---\ntext"),
            ("instructions/25-append.md", "---\nmode: append\n---\ntext"),
            ("instructions/30-kind.md", "---\nmode: [replace]\n---\ntext"),
        ],
    )]);
    assert!(found.merged().is_none());
    assert_eq!(
        lines(&found),
        [
            "error [instructions] personal:instructions/00-fields.md:2:1: unknown field 'colour'",
            "error [instructions] personal:instructions/10-mode.md:2:7: mode must be append, replace or remove",
            "error [instructions] personal:instructions/20-add.md:2:7: mode must be append, replace or remove",
            "error [instructions] personal:instructions/30-kind.md:2:7: mode must be text",
        ]
    );
}

#[test]
fn a_block_that_is_not_closed_is_a_frontmatter_error_and_the_other_files_are_still_read() {
    let found = merged(vec![layer(
        "personal",
        "personal",
        &[
            ("instructions/00-open.md", "---\nmode: remove\ntext"),
            ("instructions/10-list.md", "---\n- a\n---\ntext"),
            ("instructions/20-bad.md", "---\nmode: nope\n---\n"),
        ],
    )]);
    assert_eq!(
        lines(&found),
        [
            "error [frontmatter] personal:instructions/00-open.md: frontmatter is not closed with '---'",
            "error [frontmatter] personal:instructions/10-list.md:2:1: frontmatter must be a YAML mapping",
            "error [instructions] personal:instructions/20-bad.md:2:7: mode must be append, replace or remove",
        ]
    );
}

#[test]
fn a_block_without_text_is_dropped_but_can_still_be_replaced() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[
                ("instructions/00-empty.md", "  \n\n"),
                ("instructions/10-later.md", "\n"),
            ],
        ),
        layer(
            "shop-api",
            "repo",
            &[(
                "instructions/10-later.md",
                "---\nmode: replace\n---\nnow it has text\n",
            )],
        ),
    ]);
    assert_eq!(
        blocks(&model),
        [(
            "## Repo: shop-api".to_owned(),
            "instructions/10-later.md".to_owned(),
            "now it has text".to_owned()
        )]
    );
}

#[test]
fn files_of_a_layer_are_in_code_point_order_of_the_whole_file_name() {
    let model = merged_model(vec![layer(
        "personal",
        "personal",
        &[
            ("instructions/a.md", "dot"),
            ("instructions/a-b.md", "dash"),
            ("instructions/B.md", "upper"),
            ("instructions/é.md", "accent"),
            ("instructions/z.md", "last"),
        ],
    )]);
    let bodies: Vec<String> = blocks(&model).into_iter().map(|block| block.2).collect();
    assert_eq!(bodies, ["upper", "dash", "dot", "last", "accent"]);
}

#[test]
fn the_claude_text_of_each_layer_is_kept_in_stack_order_and_an_empty_one_is_not() {
    let model = merged_model(vec![
        layer(
            "personal",
            "personal",
            &[(
                "targets/claude/CLAUDE.extra.md",
                "---\nx: 1\n---\nUse plan mode.\n",
            )],
        ),
        layer(
            "acme",
            "client",
            &[("targets/claude/CLAUDE.extra.md", " \n")],
        ),
        layer(
            "shop-api",
            "repo",
            &[("targets/claude/CLAUDE.extra.md", "Prefer small diffs.")],
        ),
    ]);
    let found: Vec<(&str, &str)> = model
        .claude_extras()
        .iter()
        .map(|extra| (extra.layer().as_str(), extra.body()))
        .collect();
    assert_eq!(
        found,
        [
            ("personal", "Use plan mode."),
            ("shop-api", "Prefer small diffs.")
        ]
    );
}

#[test]
fn crlf_files_give_the_same_blocks_as_lf_files() {
    let lf = merged_model(vec![layer(
        "personal",
        "personal",
        &[("instructions/00.md", "---\n---\none\n\ntwo\n")],
    )]);
    let crlf = merged_model(vec![layer(
        "personal",
        "personal",
        &[("instructions/00.md", "---\r\n---\r\none\r\n\r\ntwo\r\n")],
    )]);
    assert_eq!(blocks(&lf), blocks(&crlf));
}

proptest! {
    // §7.1 "stack order, then by filename": with only adding files, the blocks come out ordered by
    // the layer's place in the stack and then by the file name's code points, whatever order the
    // files were listed in.
    #[test]
    fn block_order_depends_only_on_stack_order_and_name_order(
        names in proptest::collection::vec(
            proptest::collection::btree_set("[a-z0-9.-]{1,5}", 0..5),
            1..4,
        ),
    ) {
        let build = |reversed: bool| -> Vec<ResolvedLayer> {
            names
                .iter()
                .enumerate()
                .map(|(index, files)| {
                    let paths: Vec<String> = if reversed {
                        files.iter().rev().map(|name| format!("instructions/{name}.md")).collect()
                    } else {
                        files.iter().map(|name| format!("instructions/{name}.md")).collect()
                    };
                    let pairs: Vec<(&str, &str)> =
                        paths.iter().map(|path| (path.as_str(), "text")).collect();
                    layer(&format!("layer-{index}"), "repo", &pairs)
                })
                .collect()
        };
        let expected: Vec<(String, String)> = names
            .iter()
            .enumerate()
            .flat_map(|(index, files)| {
                let mut sorted: Vec<String> =
                    files.iter().map(|name| format!("instructions/{name}.md")).collect();
                sorted.sort();
                sorted.into_iter().map(move |path| (format!("layer-{index}"), path))
            })
            .collect();
        for reversed in [false, true] {
            let found = merged(build(reversed));
            let model = found.merged().expect("no error was found");
            let order: Vec<(String, String)> = model
                .instructions()
                .iter()
                .map(|traced| {
                    (
                        traced.value().heading().name().to_string(),
                        traced.value().file().as_str().to_owned(),
                    )
                })
                .collect();
            prop_assert_eq!(&order, &expected);
        }
    }
}
