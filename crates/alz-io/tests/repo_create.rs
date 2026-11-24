//! Creating files below the repository root (spec §4.3, `alz init`): create-only, never through a
//! link, in a temporary repository with a folder beside it that must never change.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use alz_core::account::AccountFolders;
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_core::render::write_plan::Observed;
    use alz_io::{RepoError, RepoRoot};
    use alz_test_support::ScratchDir;

    struct Place {
        _scratch: ScratchDir,
        base: PathBuf,
    }

    impl Place {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let base = fs::canonicalize(scratch.path()).expect("a real path");
            for name in ["repo", "outside", "home"] {
                fs::create_dir(base.join(name)).expect("made");
            }
            Self {
                _scratch: scratch,
                base,
            }
        }

        fn root(&self) -> RepoRoot {
            let absolute = |name: &str| {
                AbsolutePath::try_from(self.base.join(name).to_str().expect("text"))
                    .expect("absolute")
            };
            let folders = AccountFolders::from_env(&absolute("home"), None, None);
            RepoRoot::open(&absolute("repo"), &folders).expect("the repository opens")
        }

        fn repo(&self, relative: &str) -> PathBuf {
            self.base.join("repo").join(relative)
        }

        fn names(&self, folder: &str) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(self.base.join(folder))
                .expect("a folder")
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    fn safe(path: &str) -> SafeRelPath {
        SafeRelPath::try_from(path).expect("a safe path")
    }

    #[test]
    fn a_file_is_created_with_its_folders_mode_0644_and_no_temporary_file_left() {
        let place = Place::new();
        place
            .root()
            .create_new(&safe(".layer/instructions/50-repo.md"), b"- text\n")
            .expect("created");
        let path = place.repo(".layer/instructions/50-repo.md");
        assert_eq!(fs::read_to_string(&path).expect("read"), "- text\n");
        assert_eq!(
            fs::metadata(&path).expect("a mode").permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(place.names("repo/.layer/instructions"), ["50-repo.md"]);
    }

    #[test]
    fn a_file_that_exists_is_never_overwritten() {
        let place = Place::new();
        fs::write(place.repo("layer.yaml"), "mine\n").expect("written");
        let result = place.root().create_new(&safe("layer.yaml"), b"generated\n");
        assert!(
            matches!(result, Err(RepoError::Exists { .. })),
            "{result:?}"
        );
        assert_eq!(
            fs::read_to_string(place.repo("layer.yaml")).expect("read"),
            "mine\n"
        );
        assert_eq!(place.names("repo"), ["layer.yaml"]);
    }

    #[test]
    fn a_link_at_the_path_is_refused_and_its_target_is_not_written() {
        let place = Place::new();
        std::os::unix::fs::symlink("../outside/target", place.repo("layer.yaml")).expect("linked");
        let result = place.root().create_new(&safe("layer.yaml"), b"generated\n");
        assert!(
            matches!(result, Err(RepoError::Exists { .. })),
            "{result:?}"
        );
        assert_eq!(place.names("outside"), Vec::<String>::new());
        assert_eq!(place.names("repo"), ["layer.yaml"]);
    }

    #[test]
    fn a_link_in_a_folder_above_the_path_is_never_written_through() {
        let place = Place::new();
        std::os::unix::fs::symlink("../outside", place.repo("sub")).expect("linked");
        let result = place.root().create_new(&safe("sub/a.md"), b"generated\n");
        assert!(
            matches!(result, Err(RepoError::Changed { .. })),
            "{result:?}"
        );
        assert_eq!(place.names("outside"), Vec::<String>::new());
    }

    #[test]
    fn a_folder_at_the_path_is_refused() {
        let place = Place::new();
        fs::create_dir(place.repo("layer.yaml")).expect("made");
        let result = place.root().create_new(&safe("layer.yaml"), b"generated\n");
        assert!(
            matches!(result, Err(RepoError::Exists { .. })),
            "{result:?}"
        );
    }

    #[test]
    fn what_is_at_a_path_is_looked_at_without_following_a_link() {
        let place = Place::new();
        fs::write(place.base.join("outside/target"), "x").expect("written");
        std::os::unix::fs::symlink("../outside/target", place.repo("link")).expect("linked");
        fs::write(place.repo("file"), "x").expect("written");
        let root = place.root();
        assert_eq!(root.look_at(&safe("link")).expect("looked"), Observed::Link);
        assert_eq!(
            root.look_at(&safe("missing")).expect("looked"),
            Observed::Absent
        );
        assert!(matches!(
            root.look_at(&safe("file")).expect("looked"),
            Observed::Regular { .. }
        ));
    }
}
