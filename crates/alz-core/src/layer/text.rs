//! The text fields of `layer.yaml` that the spec leaves free: the version, the description, the
//! slot names and the leak-guard patterns (§5.1). Each is YAML text and nothing more; the type
//! keeps one field from being used for another.

macro_rules! free_text {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The text as written in the file.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(text: &str) -> Self {
                Self(text.to_owned())
            }
        }
    };
}

free_text! {
    /// A layer's `version`, as written (`"1.4.0"`): the spec calls it semver and states no rule.
    LayerVersion
}

free_text! {
    /// A layer's `description`, as written.
    Description
}

free_text! {
    /// The name of a skill that a layer provides or requires (`provides_slots`, `requires_slots`).
    SlotName
}

free_text! {
    /// A `forbid_patterns` entry, kept as text. Compiling it belongs to the leak guard (§9.2),
    /// which reads a layer that is already parsed.
    ForbidPatternText
}

#[cfg(test)]
mod tests {
    use super::{Description, ForbidPatternText, LayerVersion, SlotName};

    #[test]
    fn each_field_keeps_its_text_as_written() {
        assert_eq!(LayerVersion::from("1.10").as_str(), "1.10");
        assert_eq!(Description::from("a  b").as_str(), "a  b");
        assert_eq!(SlotName::from("arch-standards").as_str(), "arch-standards");
        assert_eq!(
            ForbidPatternText::from("(?i)\\b(acme)\\b").as_str(),
            "(?i)\\b(acme)\\b"
        );
    }
}
