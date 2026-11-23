//! The text of a file as the leak guard reads it (spec §9.2).
//!
//! A file is read lossily, so one that is not UTF-8 is checked like any other. A file that starts
//! with a UTF-16 byte order mark is decoded as UTF-16, and every NUL is removed, so that a token
//! written in UTF-16 (or with NULs between its characters) is found by the same patterns.

/// The text of `bytes`: decoded as UTF-16 after a byte order mark, as UTF-8 otherwise, with the
/// mark and every NUL left out and every undecodable unit replaced by U+FFFD.
pub(super) fn decode(bytes: &[u8]) -> String {
    let decoded = match bytes {
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    };
    decoded.replace('\0', "")
}

/// `bytes` as UTF-16 units of the given byte order; a last odd byte is dropped.
fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let (pairs, _odd_last_byte) = bytes.as_chunks::<2>();
    let units = pairs.iter().copied().map(unit);
    char::decode_utf16(units)
        .map(|decoded| decoded.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::decode;

    fn utf16_le(text: &str) -> Vec<u8> {
        [0xFF, 0xFE]
            .into_iter()
            .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
            .collect()
    }

    fn utf16_be(text: &str) -> Vec<u8> {
        [0xFE, 0xFF]
            .into_iter()
            .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
            .collect()
    }

    #[test]
    fn utf8_is_read_as_it_is() {
        assert_eq!(decode("héllo\n".as_bytes()), "héllo\n");
    }

    #[test]
    fn a_utf16_file_with_a_mark_is_decoded_in_either_byte_order() {
        assert_eq!(decode(&utf16_le("key: ä€😀\n")), "key: ä€😀\n");
        assert_eq!(decode(&utf16_be("key: ä€😀\n")), "key: ä€😀\n");
    }

    #[test]
    fn nuls_are_removed_with_or_without_a_mark() {
        assert_eq!(decode(b"a\0b\0c\0"), "abc");
        assert_eq!(decode(&utf16_le("a\0b")), "ab");
    }

    #[test]
    fn a_broken_utf16_file_is_decoded_lossily_and_never_fails() {
        let mut bytes = utf16_le("ok");
        bytes.extend([0x00, 0xD8, 0x41]);
        assert_eq!(decode(&bytes), "ok\u{FFFD}");
        assert_eq!(decode(&[0xFF, 0xFE]), "");
    }
}
