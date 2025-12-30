use crate::engine::installers::errors::InstallerError;
use crate::schemas::state_file::ToolState;
use crate::schemas::tools_types::ToolEntry;

pub trait Installer {
    fn install(&self, tool: &ToolEntry) -> Result<ToolState, InstallerError>;
}
