//! A layer's own description, `layer.yaml` of a layer (spec §5.1), as validated types.

mod kind;
mod meta;
mod name;
mod scan;
mod text;
mod visibility;

pub use kind::{Kind, KindError, KindLabel};
pub use meta::LayerMeta;
pub use name::{LayerName, LayerNameError};
pub use scan::{LayerRefusal, LayerScan, Limits, NameProblem, Purpose, ReadLayer, scan_verdict};
pub use text::{Description, ForbidPatternText, LayerVersion, SlotName};
pub use visibility::{Visibility, VisibilityError};
