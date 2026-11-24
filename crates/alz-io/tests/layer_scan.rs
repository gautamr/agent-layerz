//! The walk of a layer directory on real trees (spec §5.3 "File names", "`.env` files", §7.13 "No
//! fetch on resolve"; TASK-69, decision-27): names, `.env` files, links and non-regular entries,
//! the entry and byte limits, and the order of the checks.

#[path = "support/machine.rs"]
mod machine;

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use alz_core::git::{BlobId, ObjectFormat};
    use alz_core::layer::{LayerRefusal, LayerScan, Purpose, ReadLayer, scan_verdict};
    use alz_core::lock::{ContentHash, LayerFiles};
    use alz_core::path::SafeRelPath;
    use alz_io::Dir;

    use super::machine::Machine;

    const MIB: u64 = 1024 * 1024;

    /// A machine whose `repo/layer` is the layer to walk.
    fn layer() -> Machine {
        let machine = Machine::new();
        machine.make_dirs("repo/layer");
        machine
    }

    fn open(machine: &Machine) -> Dir {
        Dir::open(&machine.abs("repo/layer"), &machine.folders(None, None)).expect("opened")
    }

    fn scan(machine: &Machine, purpose: Purpose) -> LayerScan {
        open(machine).scan(purpose)
    }

    fn verdict(machine: &Machine, purpose: Purpose) -> Result<ReadLayer, LayerRefusal> {
        scan_verdict(purpose, scan(machine, purpose))
    }

    fn names(files: &LayerFiles) -> Vec<&str> {
        files.iter().map(|(path, _)| path.as_str()).collect()
    }

    fn make_fifo(path: &Path) {
        let status = Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo ran");
        assert!(status.success(), "mkfifo made the pipe");
    }

    fn sparse_file(path: &Path, length: u64) {
        File::create(path)
            .and_then(|file| file.set_len(length))
            .expect("a sparse file");
    }

    fn many_files(machine: &Machine, count: usize) {
        (0..count).for_each(|index| machine.write(&format!("repo/layer/f{index}"), ""));
    }

    #[test]
    fn a_tree_is_hashed_over_its_regular_files_in_code_point_order() {
        let machine = layer();
        machine.make_dirs("repo/layer/a/c");
        machine.write("repo/layer/b.md", "b");
        machine.write("repo/layer/a-b.md", "a-b");
        machine.write("repo/layer/a/c/d", "d");
        machine.write("repo/layer/a/b.md", "ab");
        let read = verdict(&machine, Purpose::Build).expect("a valid layer");
        assert_eq!(names(read.files()), ["a-b.md", "a/b.md", "a/c/d", "b.md"]);
        let expected = LayerFiles::from_contents(
            [
                ("b.md", "b"),
                ("a-b.md", "a-b"),
                ("a/c/d", "d"),
                ("a/b.md", "ab"),
            ]
            .map(|(name, text)| (SafeRelPath::try_from(name).expect("safe"), text.as_bytes())),
        );
        assert_eq!(
            ContentHash::of(read.files()).to_string(),
            ContentHash::of(&expected).to_string()
        );
    }

    #[test]
    fn the_skipped_names_are_neither_entered_nor_counted() {
        let machine = layer();
        machine.write("repo/layer/a.md", "a");
        machine.make_dirs("repo/layer/.git/objects");
        machine.write("repo/layer/.git/objects/x", "x");
        machine.make_dirs("repo/layer/sub/node_modules/pkg");
        machine.write("repo/layer/sub/node_modules/pkg/index.js", "x");
        machine.write("repo/layer/.DS_Store", "x");
        // A skipped name that is a file, as a worktree's `.git` is, is skipped too.
        machine.write("repo/layer/sub/.git", "gitdir: elsewhere");
        let found = scan(&machine, Purpose::Resolution);
        // `a.md` and `sub`: nothing else is listed.
        assert_eq!(found.listed(), 2);
        let read = scan_verdict(Purpose::Resolution, found).expect("valid");
        assert_eq!(names(read.files()), ["a.md"]);
    }

    #[test]
    fn a_skipped_name_that_cannot_be_opened_is_never_opened() {
        let machine = layer();
        machine.make_dirs("repo/layer/.git");
        machine.make_dirs("repo/layer/node_modules");
        let _git = machine.lock_down("repo/layer/.git");
        let _modules = machine.lock_down("repo/layer/node_modules");
        assert!(verdict(&machine, Purpose::Resolution).is_ok());
    }

    #[test]
    fn a_env_file_at_any_depth_is_recorded_and_never_opened() {
        let machine = layer();
        machine.make_dirs("repo/layer/sub/deep");
        for name in [".env", ".env.dev", "sub/.envrc", "sub/deep/.env.local"] {
            machine.write(&format!("repo/layer/{name}"), "SECRET=dummy");
            // Mode 000: opening it for reading would fail, which the scan would record.
            fs::set_permissions(
                machine.path(&format!("repo/layer/{name}")),
                fs::Permissions::from_mode(0o000),
            )
            .expect("locked");
        }
        machine.write("repo/layer/env.md", "not a .env file");
        let read = verdict(&machine, Purpose::Build).expect("an unread .env file is no failure");
        let listed: Vec<&str> = read.env_files().iter().map(SafeRelPath::as_str).collect();
        assert_eq!(
            listed,
            [".env", ".env.dev", "sub/.envrc", "sub/deep/.env.local"]
        );
        assert_eq!(names(read.files()), ["env.md"]);
        assert_eq!(read.without_env_files(), Err(LayerRefusal::EnvFile));
    }

    #[test]
    fn a_folder_named_env_is_entered_and_the_files_in_it_are_hashed() {
        let machine = layer();
        machine.make_dirs("repo/layer/.env");
        machine.write("repo/layer/.env/x", "inside");
        let read = verdict(&machine, Purpose::Build).expect("valid");
        assert_eq!(names(read.files()), [".env/x"]);
        assert_eq!(read.env_files(), []);
    }

    #[test]
    fn a_link_is_counted_and_never_followed_or_hashed() {
        let machine = layer();
        machine.make_dirs("repo/other-folder");
        machine.write("repo/other-folder/secret.md", "outside");
        machine.write("repo/layer/real.md", "real");
        machine.link("repo/layer/to-file", "real.md");
        machine.link("repo/layer/to-folder", "../other-folder");
        machine.link("repo/layer/dangling", "nowhere");
        machine.link("repo/layer/.envrc", "real.md");
        let found = scan(&machine, Purpose::Resolution);
        assert_eq!(found.listed(), 5);
        let read = scan_verdict(Purpose::Resolution, found).expect("valid");
        assert_eq!(names(read.files()), ["real.md"]);
        // A link named like a .env file is a link, not a .env file: nothing is opened.
        assert_eq!(read.env_files(), []);
    }

    #[test]
    fn a_fifo_and_a_socket_are_counted_and_do_not_block_the_walk() {
        let machine = layer();
        machine.write("repo/layer/a.md", "a");
        make_fifo(&machine.path("repo/layer/pipe"));
        make_fifo(&machine.path("repo/layer/.env"));
        machine.make_dirs("repo/layer/sub");
        make_fifo(&machine.path("repo/layer/sub/other-pipe"));
        let _socket = UnixListener::bind(machine.path("repo/layer/sock")).expect("a socket");
        let dir = open(&machine);
        let (sender, receiver) = mpsc::channel();
        let walker = thread::spawn(move || {
            sender.send(dir.scan(Purpose::Resolution)).expect("sent");
        });
        let found = receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("the walk finished without waiting for a writer");
        walker.join().expect("the walk did not panic");
        // a.md, pipe, .env, sub, sub/other-pipe, sock
        assert_eq!(found.listed(), 6);
        let read = scan_verdict(Purpose::Resolution, found).expect("valid");
        assert_eq!(names(read.files()), ["a.md"]);
        // A FIFO named .env is not a regular file, so it is not a .env file.
        assert_eq!(read.env_files(), []);
    }

    #[test]
    fn a_folder_that_cannot_be_opened_and_a_file_that_cannot_be_read_are_unreadable() {
        let machine = layer();
        machine.make_dirs("repo/layer/closed");
        let closed = machine.lock_down("repo/layer/closed");
        assert_eq!(
            verdict(&machine, Purpose::Build).map(|_| ()),
            Err(LayerRefusal::Unreadable)
        );
        drop(closed);

        let machine = layer();
        machine.write("repo/layer/secret.md", "x");
        fs::set_permissions(
            machine.path("repo/layer/secret.md"),
            fs::Permissions::from_mode(0o000),
        )
        .expect("locked");
        for purpose in [Purpose::Resolution, Purpose::Build] {
            assert_eq!(
                verdict(&machine, purpose).map(|_| ()),
                Err(LayerRefusal::Unreadable)
            );
        }
    }

    #[test]
    fn a_deeply_nested_layer_is_walked_from_handles() {
        let machine = layer();
        let nested = "d/".repeat(150);
        machine.make_dirs(&format!("repo/layer/{nested}"));
        machine.write(&format!("repo/layer/{nested}leaf"), "x");
        let read = verdict(&machine, Purpose::Build).expect("valid");
        assert_eq!(read.files().len(), 1);
        assert_eq!(scan(&machine, Purpose::Build).listed(), 151);
    }

    #[test]
    fn a_walk_against_a_tree_records_blob_ids_executable_bits_and_other_entries() {
        let machine = layer();
        machine.write("repo/layer/a.md", "hello\n");
        machine.write("repo/layer/run.sh", "#!/bin/sh\n");
        fs::set_permissions(
            machine.path("repo/layer/run.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("mode set");
        machine.write("repo/layer/.env.dev", "SECRET=1\n");
        machine.link("repo/layer/link.md", "a.md");

        let scan = open(&machine).scan_against_tree(Purpose::Build, ObjectFormat::Sha1);

        let facts: Vec<_> = scan
            .git_files()
            .iter()
            .map(|file| (file.path.as_str().to_owned(), file.executable, file.blob))
            .collect();
        let blob = |text: &str| BlobId::of(ObjectFormat::Sha1, text.as_bytes()).id();
        assert_eq!(facts.len(), 2);
        assert!(facts.contains(&("a.md".to_owned(), false, blob("hello\n"))));
        assert!(facts.contains(&("run.sh".to_owned(), true, blob("#!/bin/sh\n"))));
        assert_eq!(scan.other_entries(), 1);
        assert_eq!(scan.env_files().len(), 1);
        // The plain walk records none of it.
        assert_eq!(open(&machine).scan(Purpose::Build).git_files().len(), 0);
    }

    #[test]
    fn a_file_with_the_owner_execute_bit_keeps_it_in_the_layer_contents() {
        let machine = layer();
        machine.write("repo/layer/a.md", "hello\n");
        machine.write("repo/layer/run.sh", "#!/bin/sh\n");
        fs::set_permissions(
            machine.path("repo/layer/run.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("mode set");
        let (contents, _) = verdict(&machine, Purpose::Build)
            .expect("read")
            .into_parts();
        let path = |text: &str| SafeRelPath::try_from(text).expect("a safe path");
        assert!(contents.is_executable(&path("run.sh")));
        assert!(!contents.is_executable(&path("a.md")));
    }

    #[test]
    fn a_walk_against_a_tree_keeps_the_resolution_limits_where_they_apply() {
        let machine = layer();
        many_files(&machine, 2001);
        let scan = open(&machine).scan_against_tree(Purpose::Resolution, ObjectFormat::Sha1);
        assert_eq!(
            scan_verdict(Purpose::Resolution, scan).map(|_| ()),
            Err(LayerRefusal::TooManyEntries)
        );
        // The build has no limit: the same layer is read in full.
        let scan = open(&machine).scan_against_tree(Purpose::Build, ObjectFormat::Sha1);
        assert_eq!(scan.git_files().len(), 2001);
    }

    #[test]
    fn entry_2000_is_listed_and_the_walk_stops_at_entry_2001() {
        let machine = layer();
        many_files(&machine, 2000);
        assert_eq!(scan(&machine, Purpose::Resolution).listed(), 2000);
        assert!(verdict(&machine, Purpose::Resolution).is_ok());

        machine.write("repo/layer/one-more", "");
        let found = scan(&machine, Purpose::Resolution);
        assert_eq!(found.listed(), 2001);
        assert_eq!(
            scan_verdict(Purpose::Resolution, found).map(|_| ()),
            Err(LayerRefusal::TooManyEntries)
        );
    }

    #[test]
    fn the_walk_never_goes_past_entry_2001() {
        let machine = layer();
        many_files(&machine, 3000);
        assert_eq!(scan(&machine, Purpose::Resolution).listed(), 2001);
    }

    #[test]
    fn files_folders_links_other_entries_and_env_files_all_count_and_skipped_names_do_not() {
        let machine = layer();
        machine.make_dirs("repo/layer/.git");
        many_files(&machine, 1000);
        // 1000 files + 300 folders + 300 links + 200 pipes + 200 .env files = 2000.
        for index in 0..300 {
            machine.make_dirs(&format!("repo/layer/dir{index}"));
            machine.link(&format!("repo/layer/link{index}"), "f0");
        }
        for index in 0..200 {
            make_fifo(&machine.path(&format!("repo/layer/pipe{index}")));
            machine.write(&format!("repo/layer/.env{index}"), "");
        }
        // Everything below a skipped name is invisible, and so is the name.
        (0..500).for_each(|index| machine.write(&format!("repo/layer/.git/x{index}"), ""));
        assert_eq!(scan(&machine, Purpose::Resolution).listed(), 2000);
        assert!(verdict(&machine, Purpose::Resolution).is_ok());

        machine.make_dirs("repo/layer/dir0/inside");
        assert_eq!(
            verdict(&machine, Purpose::Resolution).map(|_| ()),
            Err(LayerRefusal::TooManyEntries)
        );
    }

    #[test]
    fn the_build_has_no_entry_limit() {
        let machine = layer();
        many_files(&machine, 2500);
        assert_eq!(scan(&machine, Purpose::Build).listed(), 2500);
        let read = verdict(&machine, Purpose::Build).expect("no limit");
        assert_eq!(read.files().len(), 2500);
    }

    #[test]
    fn sixteen_mib_of_files_is_read_and_one_byte_more_is_unreadable_in_resolution_only() {
        let machine = layer();
        sparse_file(&machine.path("repo/layer/a"), 8 * MIB);
        sparse_file(&machine.path("repo/layer/b"), 8 * MIB);
        assert_eq!(scan(&machine, Purpose::Resolution).bytes(), 16 * MIB);
        assert!(verdict(&machine, Purpose::Resolution).is_ok());

        machine.write("repo/layer/c", "x");
        assert_eq!(
            verdict(&machine, Purpose::Resolution).map(|_| ()),
            Err(LayerRefusal::Unreadable)
        );
        assert!(verdict(&machine, Purpose::Build).is_ok());
    }

    #[test]
    fn one_file_past_sixteen_mib_stops_reading_at_the_budget() {
        let machine = layer();
        sparse_file(&machine.path("repo/layer/big"), 16 * MIB + 1);
        let found = scan(&machine, Purpose::Resolution);
        // The file was cut at the budget: no more than 16 MiB and one byte were counted.
        assert_eq!(found.bytes(), 16 * MIB + 1);
        assert_eq!(
            scan_verdict(Purpose::Resolution, found).map(|_| ()),
            Err(LayerRefusal::Unreadable)
        );
    }

    #[test]
    fn a_file_of_sixteen_mib_is_read_in_the_build_and_one_byte_more_is_unreadable() {
        let machine = layer();
        sparse_file(&machine.path("repo/layer/big"), 16 * MIB);
        let read = verdict(&machine, Purpose::Build).expect("16 MiB is read");
        assert_eq!(names(read.files()), ["big"]);

        sparse_file(&machine.path("repo/layer/big"), 16 * MIB + 1);
        assert_eq!(
            verdict(&machine, Purpose::Build).map(|_| ()),
            Err(LayerRefusal::Unreadable)
        );
        // A name that is a problem still wins, as in every refusal.
        machine.write("repo/layer/bad\\name", "x");
        assert_eq!(
            verdict(&machine, Purpose::Build).map(|_| ()),
            Err(LayerRefusal::NameNotSafe)
        );
    }

    #[test]
    fn the_build_has_no_budget_in_all_only_the_cap_of_each_file() {
        let machine = layer();
        (0..3).for_each(|index| {
            sparse_file(&machine.path(&format!("repo/layer/f{index}")), 16 * MIB);
        });
        assert_eq!(scan(&machine, Purpose::Build).bytes(), 48 * MIB);
        assert!(verdict(&machine, Purpose::Build).is_ok());
    }

    #[test]
    fn env_files_and_links_add_nothing_to_the_sixteen_mib() {
        let machine = layer();
        sparse_file(&machine.path("repo/layer/a"), 16 * MIB);
        sparse_file(&machine.path("repo/layer/.env"), 5 * MIB);
        machine.link("repo/layer/link", "a");
        make_fifo(&machine.path("repo/layer/pipe"));
        assert_eq!(scan(&machine, Purpose::Resolution).bytes(), 16 * MIB);
    }

    #[test]
    fn after_the_budget_is_used_up_names_are_still_checked() {
        let machine = layer();
        sparse_file(&machine.path("repo/layer/big"), 16 * MIB + 1);
        // A name that is no safe path, valid on every file system, listed after the big file in
        // whatever order the file system lists.
        machine.write("repo/layer/back\\slash", "x");
        assert_eq!(
            verdict(&machine, Purpose::Resolution).map(|_| ()),
            Err(LayerRefusal::NameNotSafe)
        );
    }

    #[test]
    fn a_name_that_is_no_safe_path_is_refused_before_the_budget_and_the_hash() {
        let machine = layer();
        machine.write("repo/layer/a.md", "a");
        machine.make_dirs("repo/layer/folder");
        machine.write("repo/layer/folder/x\\y", "x");
        for purpose in [Purpose::Resolution, Purpose::Build] {
            assert_eq!(
                verdict(&machine, purpose).map(|_| ()),
                Err(LayerRefusal::NameNotSafe)
            );
        }
    }

    #[test]
    fn too_many_entries_win_over_a_bad_name_which_wins_over_an_unreadable_file_in_resolution() {
        let machine = layer();
        machine.write("repo/layer/back\\slash", "x");
        machine.write("repo/layer/secret.md", "x");
        fs::set_permissions(
            machine.path("repo/layer/secret.md"),
            fs::Permissions::from_mode(0o000),
        )
        .expect("locked");
        assert_eq!(
            verdict(&machine, Purpose::Resolution).map(|_| ()),
            Err(LayerRefusal::NameNotSafe)
        );
        many_files(&machine, 2001);
        assert_eq!(
            verdict(&machine, Purpose::Resolution).map(|_| ()),
            Err(LayerRefusal::TooManyEntries)
        );
        // In the build the name comes first and the entry count is not looked at.
        assert_eq!(
            verdict(&machine, Purpose::Build).map(|_| ()),
            Err(LayerRefusal::NameNotSafe)
        );
    }

    /// A name with the byte 0xFF cannot be made on macOS: APFS refuses it with `EILSEQ`
    /// (observed, spec §5.3). These tests therefore run on Linux only.
    #[cfg(target_os = "linux")]
    mod not_utf8 {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        use alz_core::Stripped;

        use super::*;

        fn bad(prefix: &str) -> std::ffi::OsString {
            let mut bytes = prefix.as_bytes().to_vec();
            bytes.push(0xff);
            OsStr::from_bytes(&bytes).to_owned()
        }

        fn assert_names_the_layer_never_the_name(refusal: LayerRefusal) {
            let line = refusal
                .file_name_diagnostic(Stripped::from_untrusted("path:../hub"))
                .expect("the name error")
                .to_string();
            assert_eq!(
                line,
                "error [layer] a file or folder name in layer 'path:../hub' is not valid UTF-8; \
                 rename it"
            );
            assert!(!line.contains('\u{fffd}'));
            assert!(!line.as_bytes().contains(&0xff));
        }

        #[test]
        fn a_file_named_with_the_byte_0xff_is_the_name_error_in_both_purposes() {
            let machine = layer();
            machine.write("repo/layer/a.md", "a");
            fs::write(machine.path("repo/layer").join(bad("x")), "x").expect("made");
            for (purpose, miss) in [
                (Purpose::Resolution, "layer-invalid"),
                (Purpose::Build, "layer-invalid"),
            ] {
                let refusal = verdict(&machine, purpose).map(|_| ()).expect_err("refused");
                assert_eq!(refusal, LayerRefusal::NameNotUtf8);
                assert_eq!(refusal.miss(), miss);
                assert_names_the_layer_never_the_name(refusal);
            }
        }

        #[test]
        fn a_name_with_a_control_or_format_character_is_the_name_error_in_both_purposes() {
            for name in [
                "line\nfeed.md",
                "zero\u{200b}width.md",
                "bell\u{7}.md",
                "sep\u{2028}.md",
            ] {
                let machine = layer();
                machine.write("repo/layer/a.md", "a");
                fs::write(machine.path("repo/layer").join(name), "x").expect("made");
                for purpose in [Purpose::Resolution, Purpose::Build] {
                    let refusal = verdict(&machine, purpose).map(|_| ()).expect_err("refused");
                    assert_eq!(refusal, LayerRefusal::NameControlOrFormat, "{name:?}");
                    assert_eq!(refusal.miss(), "layer-invalid");
                    assert_eq!(
                        refusal
                            .file_name_diagnostic(Stripped::from_untrusted("path:../hub"))
                            .expect("the name error")
                            .to_string(),
                        "error [layer] a file or folder name in layer 'path:../hub' holds a \
                         control or format character; rename it"
                    );
                }
            }
        }

        #[test]
        fn a_folder_with_a_control_character_in_its_name_is_refused_as_well() {
            let machine = layer();
            let folder = machine.path("repo/layer").join("d\u{1b}[31m");
            fs::create_dir(&folder).expect("made");
            fs::write(folder.join("inside.md"), "x").expect("made");
            assert_eq!(
                verdict(&machine, Purpose::Build).map(|_| ()),
                Err(LayerRefusal::NameControlOrFormat)
            );
        }

        #[test]
        fn a_bad_unicode_name_wins_over_a_control_character_which_wins_over_an_unsafe_path() {
            let machine = layer();
            machine.write("repo/layer/back\\slash", "x");
            machine.write("repo/layer/tab\there", "x");
            assert_eq!(
                verdict(&machine, Purpose::Build).map(|_| ()),
                Err(LayerRefusal::NameControlOrFormat)
            );
            fs::write(machine.path("repo/layer").join(bad("x")), "x").expect("made");
            assert_eq!(
                verdict(&machine, Purpose::Build).map(|_| ()),
                Err(LayerRefusal::NameNotUtf8)
            );
        }

        #[test]
        fn a_folder_named_with_the_byte_0xff_is_the_name_error_and_is_still_listed_inside() {
            let machine = layer();
            let folder = machine.path("repo/layer").join(bad("d"));
            fs::create_dir(&folder).expect("made");
            fs::write(folder.join("inside.md"), "x").expect("made");
            let found = scan(&machine, Purpose::Resolution);
            // The folder and the file inside it.
            assert_eq!(found.listed(), 2);
            assert_eq!(
                scan_verdict(Purpose::Build, found).map(|_| ()),
                Err(LayerRefusal::NameNotUtf8)
            );
        }

        #[test]
        fn a_bad_name_wins_over_a_name_that_is_no_safe_path_wherever_it_lies() {
            let machine = layer();
            machine.write("repo/layer/back\\slash", "x");
            let folder = machine.path("repo/layer").join(bad("d"));
            fs::create_dir(&folder).expect("made");
            fs::write(folder.join(bad("deep")), "x").expect("made");
            assert_eq!(
                verdict(&machine, Purpose::Build).map(|_| ()),
                Err(LayerRefusal::NameNotUtf8)
            );
        }

        #[test]
        fn too_many_entries_still_win_over_a_bad_name() {
            let machine = layer();
            fs::write(machine.path("repo/layer").join(bad("x")), "x").expect("made");
            many_files(&machine, 2001);
            assert_eq!(
                verdict(&machine, Purpose::Resolution).map(|_| ()),
                Err(LayerRefusal::TooManyEntries)
            );
        }
    }
}
