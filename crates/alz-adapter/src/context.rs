//! What the stack manifest says that is not a layer item (spec §8.0 **Rendering.**).

/// The context of a rendering: what the stack manifest says that is not a layer item.
///
/// The struct is `#[non_exhaustive]`, so a minor version can add a field and a value is made with
/// [`Context::new`]:
///
/// ```compile_fail
/// use alz_adapter::Context;
///
/// let _ = Context { additional_directories: Vec::new() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Context {
    /// The additional directories: `workspace.hub` first, then `workspace.add_dirs`, as written,
    /// without duplicates (§5.2).
    pub additional_directories: Vec<String>,
}

impl Context {
    /// A context with these additional directories.
    #[must_use]
    pub const fn new(additional_directories: Vec<String>) -> Self {
        Self {
            additional_directories,
        }
    }
}
