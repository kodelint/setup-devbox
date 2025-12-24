use crate::schemas::state_file::ToolState;
use crate::schemas::tools_types::ToolEntry;

pub trait Installer {
    fn install(&self, tool: &ToolEntry) -> Option<ToolState>;
}
