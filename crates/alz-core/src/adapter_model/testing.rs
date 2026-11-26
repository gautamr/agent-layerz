//! The pipeline of the Claude adapter's tests: stacks built from text, merged by the core, given to
//! the adapter as the model of the contract. The tests of the adapter state their input as the
//! files of a layer, as the acceptance fixtures do, and reach the merge only through this module,
//! so the adapter's own code never names a merge type.

use alz_adapter::Context;

use crate::adapter_model::model_for;
use crate::stack::Workspace;
use crate::target::claude::{ClaudeFiles, ClaudeSettingsFiles};

pub(crate) use crate::merge::Merged;
pub(crate) use crate::merge::testing::{
    layer_executable, layer_from_yaml, merged as merged_stack, merged_with,
};

/// The Claude files of a merged stack: the core's model of it for Claude, then the adapter.
pub(crate) fn files_of(merged: &Merged) -> ClaudeFiles {
    ClaudeFiles::of(&model_for(merged, crate::target::claude()))
}

/// The Claude settings, hook scripts and `.mcp.json` of a merged stack under a workspace.
pub(crate) fn settings_files_of(merged: &Merged, workspace: &Workspace) -> ClaudeSettingsFiles {
    let context = Context::new(
        workspace
            .add_dirs()
            .iter()
            .map(|dir| dir.as_str().to_owned())
            .collect(),
    );
    ClaudeSettingsFiles::of(&model_for(merged, crate::target::claude()), &context)
}
