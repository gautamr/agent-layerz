//! The adapter trait (spec §8.0 **Rendering.**).

use crate::context::Context;
use crate::descriptor::Descriptor;
use crate::model::Model;
use crate::output::Output;

/// An adapter: a pure function from the model and a context to its output. It never reads or
/// writes a file, starts a process, or reads the environment or the clock, so the same model and
/// context give the same output.
///
/// The trait is public, not sealed. The `alz` binary chooses the adapters it registers, and
/// refuses one built for another major version of the model, or for a later minor version than
/// the core's ([`ModelVersion::runs`](crate::ModelVersion::runs)).
pub trait Adapter {
    /// The adapter's static data.
    fn descriptor(&self) -> &Descriptor;

    /// The files, diagnostics and coverage report for one target.
    fn render(&self, model: &Model, context: &Context) -> Output;
}
