use crate::engine::installers::errors::InstallerError;
use crate::schemas::state_file::ToolState;
use crate::schemas::tools_types::ToolEntry;

/// # `Installer` Trait
///
/// This trait defines the common interface for all installers. Each installer
/// (e.g., `brew`, `github`, `cargo`) must implement this trait to provide a
/// consistent way to install tools and check for their latest versions.
pub trait Installer {
    /// # `install`
    ///
    /// Installs a tool based on the provided `ToolEntry`.
    ///
    /// ## Arguments
    ///
    /// * `tool`: A reference to a `ToolEntry` struct, which contains the tool's
    ///   configuration (name, version, source, etc.).
    ///
    /// ## Returns
    ///
    /// A `Result` which is:
    /// - `Ok(ToolState)`: A `ToolState` struct representing the state of the
    ///   installed tool. This is used for tracking and state management.
    /// - `Err(InstallerError)`: An `InstallerError` if the installation fails for any reason.
    fn install(&self, tool: &ToolEntry) -> Result<ToolState, InstallerError>;

    /// # `get_latest_version`
    ///
    /// Gets the latest available version for a tool.
    ///
    /// ## Arguments
    ///
    /// * `tool`: A reference to a `ToolEntry` struct. The implementation for each
    ///   installer will use the information in this struct (e.g., repo, package name)
    ///   to find the latest version.
    ///
    /// ## Returns
    ///
    /// A `Result` which is:
    /// - `Ok(String)`: A string containing the latest version number.
    /// - `Err(InstallerError)`: An `InstallerError` if it fails to get the latest version.
    fn get_latest_version(&self, tool: &ToolEntry) -> Result<String, InstallerError>;
}
