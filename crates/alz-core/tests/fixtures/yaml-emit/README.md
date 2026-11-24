`samples.json`: the golden samples of alz's YAML emitter (decision-28, spec §10.2 "Generated
YAML"; TASK-55.3). A list of `{ name, class, input, yaml }`; `class` is the kind of text or
structure the sample covers. `crates/alz-core/src/render/yaml/tests.rs` reproduces every sample
byte for byte, except those with a `drift` field.

A sample with a `drift` field holds a `yaml` text that spec §10.2 says alz does not write (TASK-73):
a character it escapes where plain YAML writers leave it raw, one escaped space where others
write a backslash twice and a space, a text it quotes so that alz's reader reads it back, or an
integer alz reads exactly. The tests of the rule check alz's output for each such `input`
(`escaped_characters_are_never_written_raw...`, `a_space_between_line_feeds_is_one_escaped_space...`
and the tests of `is_read_differently`). The labels `reference-reads-differently` and
`escaped-characters-the-reference-writes-raw` in this file are the names the samples were
first given.

The samples are one file because `alz-core` bans the file system, so a test cannot list a
directory; the file is read with `include_str!`.

How it is kept: by hand, with no other tool. Add a sample by writing its `input` and the `yaml` the
spec requires (and a `drift` field only for a text alz does not write); the test prints the first
sample that differs.
