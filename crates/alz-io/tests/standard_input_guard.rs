//! Only `alz-io`'s `terminal` module reads standard input (spec §7.13 "Trust and `alz account
//! allow`", TASK-55.7.11). The `clippy.toml` of `alz-core`, `alz-io` and `alz` bans
//! `std::io::stdin`; this scan backs it where clippy does not look, and fails on any other source
//! file of the three crates that names the standard input handle.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    /// What makes a source file name the process's standard input: the function, or the type.
    const HANDLES: [&str; 2] = ["stdin()", "io::Stdin"];

    fn sources_below(folder: &Path) -> Vec<PathBuf> {
        fs::read_dir(folder)
            .expect("a source folder")
            .flat_map(|entry| {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    sources_below(&path)
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    vec![path]
                } else {
                    Vec::new()
                }
            })
            .collect()
    }

    #[test]
    fn only_the_terminal_module_names_the_standard_input_handle() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the crates folder");
        let terminal = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/terminal.rs");
        let offenders: Vec<PathBuf> = ["alz", "alz-core", "alz-io"]
            .into_iter()
            .flat_map(|name| sources_below(&crates.join(name).join("src")))
            .filter(|path| *path != terminal)
            .filter(|path| {
                let text = fs::read_to_string(path).expect("a source file");
                HANDLES.iter().any(|handle| text.contains(handle))
            })
            .collect();
        assert!(offenders.is_empty(), "reads standard input: {offenders:?}");
    }

    #[test]
    fn the_terminal_module_is_where_the_scan_looks_for_the_handle() {
        let terminal = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/terminal.rs");
        let text = fs::read_to_string(terminal).expect("the terminal module");
        assert!(HANDLES.iter().any(|handle| text.contains(handle)));
    }
}
