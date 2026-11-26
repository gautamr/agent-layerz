//! Applying an adoption plan below the repository root (spec §4.3): only the files the plan names,
//! only while they hold the bytes the plan was made from, and never through a link. Each test
//! plans an adoption of a real scratch repository with the real resolver, changes the repository,
//! then applies.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;

    use alz_core::account::AccountFolders;
    use alz_core::adopt::{AdoptionInput, AdoptionPlan, NativeFile, plan};
    use alz_core::env::EnvVars;
    use alz_core::layer::LayerContents;
    use alz_core::lint::parse_document;
    use alz_core::merge::merge;
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_core::stack::{Manifest, ResolvedStack};
    use alz_core::target::Registry;
    use alz_io::{AdoptionBasis, EnvSnapshot, RepoError, RepoRoot, StackResolver};
    use alz_test_support::{ScratchDir, read_tree, running_as_root};

    const MANIFEST: &str =
        "stack:\n  - source: path:./.layer   # the repo layer\ntargets: [claude]\n";
    const FOO: &str = "---\nname: foo\ndescription: Hand-written.\n---\nHAND-WRITTEN body.\n";

    /// A scratch repository with a hand-written agent, its manifest and its repo layer, beside a
    /// folder that no write may reach.
    struct Place {
        scratch: ScratchDir,
        base: PathBuf,
    }

    /// What the planner was given and what it made.
    struct Planned {
        plan: AdoptionPlan,
        natives: BTreeMap<SafeRelPath, NativeFile>,
        manifest_text: Vec<u8>,
        layer: LayerContents,
    }

    impl Planned {
        fn basis(&self) -> AdoptionBasis<'_> {
            AdoptionBasis {
                natives: &self.natives,
                manifest_text: &self.manifest_text,
                layer: &self.layer,
            }
        }
    }

    impl Place {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let base = fs::canonicalize(scratch.path()).expect("a real path");
            for name in ["home", "outside"] {
                scratch.make_dir(name).expect("made");
            }
            scratch.make_dirs("repo/.layer").expect("made");
            scratch
                .write("repo/layer.yaml", MANIFEST)
                .expect("a manifest");
            scratch
                .write(
                    "repo/.layer/layer.yaml",
                    "name: shop\nkind: repo\nvisibility: private\n",
                )
                .expect("a layer");
            scratch.make_dirs("repo/.claude/agents").expect("made");
            scratch
                .write("repo/.claude/agents/foo.md", FOO)
                .expect("an agent");
            scratch.write("outside/secret.txt", "keep").expect("a file");
            Self { scratch, base }
        }

        fn absolute(&self, name: &str) -> AbsolutePath {
            AbsolutePath::try_from(self.base.join(name).to_str().expect("text")).expect("absolute")
        }

        fn root(&self) -> RepoRoot {
            let folders = AccountFolders::from_env(&self.absolute("home"), None, None);
            RepoRoot::open(&self.absolute("repo"), &folders).expect("the repository opens")
        }

        fn plan(&self) -> Planned {
            self.plan_of([(".claude/agents/foo.md", FOO)])
        }

        /// The plan for the native files `files`, each as it is on disk.
        fn plan_of<'a>(&self, files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Planned {
            let env = EnvSnapshot::from_vars(EnvVars::from_iter([(
                OsString::from("HOME"),
                OsString::from(self.base.join("home")),
            )]));
            let manifest_text = self.scratch.read("repo/layer.yaml").expect("read");
            let manifest = Manifest::from_yaml(&parse_document(&manifest_text).expect("YAML"))
                .expect("a manifest");
            let resolver = StackResolver::at(&env, self.absolute("repo"));
            let report = resolver.resolve(&manifest, None);
            let stack: &ResolvedStack = report.stack().expect("the stack resolves");
            let placements = resolver.placements(stack);
            let merging = merge(stack, &manifest, &placements, &Registry::builtin());
            let merged = merging.merged().expect("the stack merges");
            let natives: BTreeMap<SafeRelPath, NativeFile> = files
                .into_iter()
                .map(|(path, text)| {
                    (
                        SafeRelPath::try_from(path).expect("a path"),
                        NativeFile::new(text.as_bytes().to_vec(), false),
                    )
                })
                .collect();
            let adoption = plan(&AdoptionInput {
                stack,
                manifest: &manifest,
                manifest_text: &manifest_text,
                placements: &placements,
                merged,
                natives: &natives,
                agents_md: None,
                registry: &Registry::builtin(),
            })
            .expect("the adoption is planned");
            let layer = stack
                .layers()
                .last()
                .expect("a repo layer")
                .contents()
                .clone();
            Planned {
                plan: adoption,
                natives,
                manifest_text,
                layer,
            }
        }

        fn tree(&self) -> BTreeMap<String, alz_test_support::TreeFile> {
            read_tree(&self.base).expect("a tree")
        }
    }

    #[test]
    fn the_plan_moves_the_native_file_into_the_layer_and_prunes_the_folders_it_left_empty() {
        let place = Place::new();
        let planned = place.plan();
        place
            .root()
            .adopt(&planned.plan, &planned.basis())
            .expect("applied");
        assert_eq!(
            place
                .scratch
                .read_text("repo/.layer/agents/foo.md")
                .unwrap_or_default(),
            FOO
        );
        assert!(!place.scratch.exists("repo/.claude"));
        assert_eq!(
            place
                .scratch
                .read_text("outside/secret.txt")
                .unwrap_or_default(),
            "keep"
        );
    }

    #[test]
    fn a_native_file_that_changed_since_the_plan_is_refused_and_nothing_is_written() {
        let place = Place::new();
        let planned = place.plan();
        place
            .scratch
            .write("repo/.claude/agents/foo.md", "EDITED AFTER THE PLAN\n")
            .expect("edited");
        let before = place.tree();
        let refused = place.root().adopt(&planned.plan, &planned.basis());
        assert!(
            matches!(refused, Err(RepoError::Changed { .. })),
            "{refused:?}"
        );
        assert_eq!(place.tree(), before);
    }

    #[test]
    fn a_native_file_replaced_by_a_link_is_never_followed() {
        let place = Place::new();
        let planned = place.plan();
        place
            .scratch
            .remove_file("repo/.claude/agents/foo.md")
            .expect("removed");
        place
            .scratch
            .make_link("repo/.claude/agents/foo.md", "../../../outside/secret.txt")
            .expect("a link");
        let before = place.tree();
        let refused = place.root().adopt(&planned.plan, &planned.basis());
        assert!(
            matches!(refused, Err(RepoError::Changed { .. })),
            "{refused:?}"
        );
        assert_eq!(place.tree(), before);
    }

    #[test]
    fn a_layer_folder_that_became_a_link_is_never_written_through() {
        let place = Place::new();
        let planned = place.plan();
        place.scratch.make_dirs("outside/layer").expect("made");
        place
            .scratch
            .make_link("repo/.layer/agents", "../../outside/layer")
            .expect("a link");
        let before = place.tree();
        let refused = place.root().adopt(&planned.plan, &planned.basis());
        assert!(refused.is_err(), "{refused:?}");
        assert_eq!(place.tree(), before);
        assert_eq!(place.scratch.list("outside/layer"), Vec::<String>::new());
    }

    #[test]
    fn a_layer_file_that_appeared_since_the_plan_is_never_overwritten() {
        let place = Place::new();
        let planned = place.plan();
        place.scratch.make_dirs("repo/.layer/agents").expect("made");
        place
            .scratch
            .write("repo/.layer/agents/foo.md", "MINE\n")
            .expect("written");
        let before = place.tree();
        let refused = place.root().adopt(&planned.plan, &planned.basis());
        assert!(
            matches!(refused, Err(RepoError::Exists { .. })),
            "{refused:?}"
        );
        assert_eq!(place.tree(), before);
    }

    /// §4.3 "Adoption, in detail": `layer.yaml` is written last. A step that fails after the layer
    /// files are written (here the old name of the agent cannot be unlinked, as its folder is
    /// read-only) leaves `layer.yaml` as it was, though the plan edits it.
    #[test]
    fn layer_yaml_is_written_after_the_layer_files_and_the_moves() {
        if running_as_root() {
            return;
        }
        let place = Place::new();
        let settings = "{\"permissions\": {\"additionalDirectories\": [\"..\"]}}\n";
        place
            .scratch
            .write("repo/.claude/settings.json", settings)
            .expect("written");
        let planned = place.plan_of([
            (".claude/agents/foo.md", FOO),
            (".claude/settings.json", settings),
        ]);
        assert!(
            planned.plan.manifest_edit().is_some(),
            "the plan edits layer.yaml"
        );
        place
            .scratch
            .set_mode("repo/.claude/agents", 0o555)
            .expect("read-only");
        let refused = place.root().adopt(&planned.plan, &planned.basis());
        place
            .scratch
            .set_mode("repo/.claude/agents", 0o755)
            .expect("restored");
        assert!(refused.is_err(), "{refused:?}");
        assert_eq!(
            place
                .scratch
                .read_text("repo/layer.yaml")
                .unwrap_or_default(),
            MANIFEST,
            "layer.yaml is untouched"
        );
        assert!(
            place.scratch.exists("repo/.layer/agents/foo.md"),
            "the layer files were written before the step that failed"
        );
    }
}
