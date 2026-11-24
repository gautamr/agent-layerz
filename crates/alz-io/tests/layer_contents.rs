//! The walk keeps the bytes it hashed (spec §5.3, §6; TASK-55.5.2): the digests of a layer's
//! contents are the files the content hash covers, on every layer of the acceptance fixtures, and
//! the bytes are the file's bytes.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use alz_core::account::AccountFolders;
    use alz_core::layer::{LayerContents, Purpose, scan_verdict};
    use alz_core::lock::{ContentHash, LayerFiles, is_env_file, is_skipped_name};
    use alz_core::path::{AbsolutePath, SafeRelPath};
    use alz_io::Dir;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .canonicalize()
            .expect("the fixtures are there")
    }

    fn absolute(path: &Path) -> AbsolutePath {
        AbsolutePath::try_from(path.to_str().expect("text")).expect("absolute")
    }

    /// Every folder below `root`, `root` included, that holds a `layer.yaml`.
    fn layer_folders(root: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(folder) = pending.pop() {
            for entry in fs::read_dir(&folder)
                .expect("listed")
                .map(|entry| entry.expect("entry"))
            {
                let kind = entry.file_type().expect("a file type");
                if kind.is_dir() && !is_skipped_name(&entry.file_name().to_string_lossy()) {
                    pending.push(entry.path());
                }
            }
            if folder.join("layer.yaml").is_file() {
                found.push(folder);
            }
        }
        found.sort();
        found
    }

    /// The files of `folder` as the spec's walk lists them, read with the standard library and
    /// no knowledge of the walk under test: no `.env` file, no skipped name, folders entered,
    /// links and other entries left out.
    fn independent(folder: &Path, prefix: &str) -> Vec<(SafeRelPath, Vec<u8>)> {
        fs::read_dir(folder)
            .expect("listed")
            .map(|entry| entry.expect("entry"))
            .flat_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let kind = entry.file_type().expect("a file type");
                let path = format!("{prefix}{name}");
                if is_skipped_name(&name) {
                    vec![]
                } else if kind.is_dir() {
                    independent(&entry.path(), &format!("{path}/"))
                } else if kind.is_file() {
                    let path = SafeRelPath::try_from(path.as_str()).expect("a safe path");
                    if is_env_file(&path) {
                        vec![]
                    } else {
                        vec![(path, fs::read(entry.path()).expect("read"))]
                    }
                } else {
                    vec![]
                }
            })
            .collect()
    }

    fn walked(folder: &Path) -> Option<(LayerContents, Vec<SafeRelPath>)> {
        let dir = Dir::open(
            &absolute(folder),
            &AccountFolders::from_env(&absolute(Path::new("/nonexistent-home")), None, None),
        )
        .expect("opened");
        scan_verdict(Purpose::Build, dir.scan(Purpose::Build))
            .ok()
            .map(alz_core::layer::ReadLayer::into_parts)
    }

    #[test]
    fn the_digests_of_every_fixture_layers_contents_equal_the_files_of_its_own_bytes() {
        let folders = layer_folders(&fixtures());
        let walked: Vec<_> = folders
            .iter()
            .filter_map(|folder| walked(folder).map(|found| (folder, found)))
            .collect();
        assert!(
            walked.len() >= 110,
            "{} of {} fixture layers passed the walk",
            walked.len(),
            folders.len()
        );
        for (folder, (contents, _)) in walked {
            let listed = independent(folder, "");
            let expected = LayerFiles::from_contents(
                listed
                    .iter()
                    .map(|(path, bytes)| (path.clone(), bytes.as_slice())),
            );
            assert_eq!(contents.files(), expected, "{}", folder.display());
            assert_eq!(
                ContentHash::of(&contents.files()),
                ContentHash::of(&expected),
                "{}",
                folder.display()
            );
            let held: Vec<(SafeRelPath, Vec<u8>)> = contents
                .iter()
                .map(|(path, bytes)| (path.clone(), bytes.to_vec()))
                .collect();
            let mut sorted = listed;
            sorted.sort();
            assert_eq!(held, sorted, "{}", folder.display());
        }
    }
}
