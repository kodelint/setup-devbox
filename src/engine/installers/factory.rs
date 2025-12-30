use std::collections::HashMap;
use std::sync::Arc;

use crate::engine::installers::{
    brew::BrewInstaller, cargo::CargoInstaller, github::GitHubInstaller, go::GoInstaller,
    pip::PipInstaller, rustup::RustupInstaller, traits::Installer, url::UrlInstaller,
    uv::UvInstaller,
};
use crate::schemas::tools_enums::SourceType;

/// Factory for creating/retrieving installers based on SourceType.
///
/// This struct manages the mapping between tool source types and their corresponding
/// installer implementations. It allows the orchestrator to remain decoupled from
/// specific installer logic.
pub struct InstallerFactory {
    installers: HashMap<SourceType, Arc<dyn Installer + Send + Sync>>,
}

impl InstallerFactory {
    /// Creates a new InstallerFactory and registers all available installers.
    pub fn new() -> Self {
        let mut installers: HashMap<SourceType, Arc<dyn Installer + Send + Sync>> = HashMap::new();

        // Register all supported installers
        installers.insert(SourceType::Github, Arc::new(GitHubInstaller));
        installers.insert(SourceType::Brew, Arc::new(BrewInstaller));
        installers.insert(SourceType::Go, Arc::new(GoInstaller));
        installers.insert(SourceType::Cargo, Arc::new(CargoInstaller));
        installers.insert(SourceType::Rustup, Arc::new(RustupInstaller));
        installers.insert(SourceType::Pip, Arc::new(PipInstaller));
        installers.insert(SourceType::Uv, Arc::new(UvInstaller));
        installers.insert(SourceType::Url, Arc::new(UrlInstaller));

        Self { installers }
    }

    /// Retrieves the appropriate installer for the given source type.
    ///
    /// # Arguments
    /// * `source_type` - The source type of the tool (e.g., Github, Brew)
    ///
    /// # Returns
    /// * `Option<Arc<dyn Installer>>` - The installer instance if found, or None
    pub fn get_installer(
        &self,
        source_type: &SourceType,
    ) -> Option<Arc<dyn Installer + Send + Sync>> {
        self.installers.get(source_type).cloned()
    }
}

impl Default for InstallerFactory {
    fn default() -> Self {
        Self::new()
    }
}
