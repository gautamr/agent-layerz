`samples.json`: the golden samples of alz's stable JSON emitter (decision-33, spec §6 "Name
order"; TASK-55.3.7). A list of `{ name, input, json }`, where `input` is the value as JSON and
`json` is the text the emitter must write: two-space indentation, a line feed at the end, keys in
code point order. `crates/alz-core/src/render/json/tests.rs` reproduces every sample byte for byte.

The samples are one file because `alz-core` bans the file system, so a test cannot list a
directory; the file is read with `include_str!`.

Not here: a key that differs above U+FFFF or in U+E000-U+FFFF, and an object with a whole-number
key next to another key. The spec orders both by code point and as plain strings (TASK-64); the
unit tests state them with the keys written out.

How it is kept: by hand, with no other tool. Add a sample by writing its `input` and the `json`
the spec requires; the test prints the first sample that differs.
