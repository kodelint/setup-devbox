use crate::libs::uninstallers::{ItemToBeRemoved, RemovalResult, RemovalSummary};
use crate::schemas::path_resolver::PathResolver;
use crate::schemas::state_file::{DevBoxState, FontState, ToolState};
use crate::{log_debug, log_error, log_info, log_warn};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

// ============================================================================
//                           REMOVAL ORCHESTRATOR
// ============================================================================

/// Orchestrates the complete removal process for tools and fonts.
///
/// The orchestrator coordinates multiple steps:
/// 1. Finding the item in the state file
/// 2. Running the appropriate uninstaller
/// 3. Cleaning up configuration files
/// 4. Updating the state file
/// 5. Removing the item from configuration YAML
///
/// ## Design Philosophy
///
/// The orchestrator follows the Orchestrator pattern, acting as a high-level
/// coordinator that delegates specific tasks to specialized components:
/// - ToolUninstaller implementations handle installation-specific removal
/// - ConfigurationCleaner handles YAML file manipulation
/// - The orchestrator focuses on workflow and error handling
pub struct RemovalOrchestrator<'a> {
    /// Mutable reference to the application state
    state: &'a mut DevBoxState,

    /// Configuration file manager
    cleaner: crate::libs::uninstallers::ConfigurationCleaner,
}

impl<'a> RemovalOrchestrator<'a> {
    /// Creates a new RemovalOrchestrator instance.
    ///
    /// # Arguments
    ///
    /// * `state` - Mutable reference to the DevBoxState
    /// * `paths` - Reference to the PathResolver (used for ConfigurationCleaner initialization)
    ///
    /// # Returns
    ///
    /// * `Ok(RemovalOrchestrator)` if initialization succeeded
    /// * `Err(String)` if the ConfigurationCleaner could not be initialized
    pub fn new(state: &'a mut DevBoxState, paths: &PathResolver) -> Result<Self, String> {
        let cleaner = crate::libs::uninstallers::ConfigurationCleaner::new(paths)?;

        Ok(Self { state, cleaner })
    }

    /// Selects the appropriate uninstaller based on the installation method.
    ///
    /// # Arguments
    ///
    /// * `installer` - Installation method string (e.g., "cargo", "pip", "github")
    ///
    /// # Returns
    ///
    /// * `Some(Box<dyn ToolUninstaller>)` - Appropriate uninstaller for the method
    /// * `None` - If the installation method is not supported
    ///
    /// # Supported Installation Methods
    ///
    /// - **github/url**: Binary files downloaded from GitHub releases or URLs
    /// - **cargo**: Rust packages installed via cargo
    /// - **rustup**: Rust toolchains managed by rustup
    /// - **go**: Go packages installed via go install
    /// - **pip**: Python packages installed via pip3
    /// - **uv**: Python tools installed via uv
    /// - **brew**: Packages installed via Homebrew
    fn get_uninstaller(
        &self,
        installer: &str,
    ) -> Option<Box<dyn crate::libs::uninstallers::ToolUninstaller>> {
        match installer.to_lowercase().as_str() {
            "github" | "url" => Some(Box::new(crate::libs::uninstallers::BinaryUninstaller)),
            "cargo" => Some(Box::new(crate::libs::uninstallers::CargoUninstaller)),
            "rustup" => Some(Box::new(crate::libs::uninstallers::RustupUninstaller)),
            "go" => Some(Box::new(crate::libs::uninstallers::GoUninstaller)),
            "pip" => Some(Box::new(crate::libs::uninstallers::PipUninstaller)),
            "uv" => Some(Box::new(crate::libs::uninstallers::UvUninstaller)),
            "brew" => Some(Box::new(crate::libs::uninstallers::BrewUninstaller)),
            _ => None,
        }
    }

    /// Removes a tool from the system.
    ///
    /// This method handles the complete removal process:
    /// 1. Locates the tool in state (by name or alias)
    /// 2. Executes the appropriate uninstaller
    /// 3. Cleans up configuration files
    /// 4. Removes the tool from state
    /// 5. Removes the tool from configuration YAML
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name or alias of the tool to remove
    ///
    /// # Returns
    ///
    /// A RemovalResult indicating success, not found, or failure
    ///
    /// # Alias Handling
    ///
    /// Tools can be referenced by either their original name or their renamed alias.
    /// For example, if "ripgrep" was renamed to "rg", both names will work:
    /// - `remove_tool("ripgrep")` - removes using original name
    /// - `remove_tool("rg")` - finds and removes ripgrep by its alias
    pub fn remove_tool(&mut self, tool_name: &str) -> RemovalResult {
        log_info!("[SDB::Remove::Tool] Removing: {}", tool_name.cyan());

        // Step 1: Find the tool by name or alias
        let original_key = self.find_tool_key(tool_name);

        let key = match original_key {
            Some(k) => k,
            None => {
                log_warn!(
                    "[SDB::Remove::Tool] Not found in state: {}",
                    tool_name.yellow()
                );
                return RemovalResult::NotFound;
            }
        };

        // Step 2: Retrieve the tool's state information
        // We can unwrap safely here since we just verified the key exists
        let tool_state = self.state.tools.get(&key).unwrap().clone();

        let uninstall_item = ItemToBeRemoved {
            item_name: key.clone(),
            items_renamed_to: tool_state.renamed_to.clone(),
            item_path: tool_state.install_path.clone(),
            item_source: tool_state.install_method.clone(),
            item_version: tool_state.version.clone(),
        };

        log_debug!(
            "[SDB::Remove::Tool] Original key: {}, Method: {}, Path: {}",
            key.cyan(),
            uninstall_item.item_source.cyan(),
            uninstall_item.item_path.cyan()
        );

        // Step 3: Execute the uninstallation
        if let Err(e) = self.execute_tool_uninstallation(&uninstall_item) {
            log_error!("[SDB::Remove::Tool] Uninstallation failed: {}", e.red());
            return RemovalResult::Failed(e);
        }

        // Step 4: Clean up configuration files
        if let Err(e) = self.remove_tool_configurations(&tool_state, &key) {
            log_warn!(
                "[SDB::Remove::Config] Config cleanup warning: {}",
                e.yellow()
            );
        }

        // Step 5: Remove from state
        self.state.tools.remove(&key);
        log_debug!("[SDB::Remove] Removed from state: {}", key);

        // Step 6: Remove from configuration YAML
        if let Err(e) = self
            .cleaner
            .remove_list_item("tools.yaml", "tools:", "name:", &key)
        {
            log_warn!("[SDB::Remove] YAML cleanup warning: {}", e.yellow());
        }

        RemovalResult::Removed
    }

    /// Locates a tool in the state by name or alias.
    ///
    /// Tools can be stored under their original name in the state file,
    /// but may have been renamed using the `renamed_to` field. This method
    /// checks both possibilities.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name or alias to search for
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The original key in the state file
    /// * `None` - If the tool was not found by name or alias
    ///
    /// # Search Strategy
    ///
    /// 1. First, check if `tool_name` exists as a key in state.tools
    /// 2. If not found, search all tools for a matching `renamed_to` value
    /// 3. Return the original key (not the alias) so we can access the state
    fn find_tool_key(&self, tool_name: &str) -> Option<String> {
        // Direct lookup by key
        if self.state.tools.contains_key(tool_name) {
            return Some(tool_name.to_string());
        }

        // Search by renamed_to field (alias lookup)
        self.state
            .tools
            .iter()
            .find(|(_, state)| {
                state
                    .renamed_to
                    .as_ref()
                    .map_or(false, |alias| alias == tool_name)
            })
            .map(|(key, _)| {
                log_info!(
                    "[SDB::Remove::Tool] Found '{}' as alias for '{}'",
                    tool_name.cyan(),
                    key.cyan()
                );
                key.clone()
            })
    }

    /// Executes the appropriate uninstaller for a tool.
    ///
    /// # Arguments
    ///
    /// * `uninstall_item` - Metadata about the tool to remove
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Uninstallation succeeded or installer type unsupported
    /// * `Err(String)` - Uninstallation failed with specific error
    ///
    /// # Unsupported Installers
    ///
    /// If the installation method is not recognized, this is logged as a warning
    /// but not treated as an error. This allows the rest of the cleanup process
    /// to continue (removing from state, cleaning configs, etc.)
    fn execute_tool_uninstallation(&self, uninstall_item: &ItemToBeRemoved) -> Result<(), String> {
        // Normalize the source type to match our uninstaller keys
        let installer_name =
            ToolState::normalize_source_type(&uninstall_item.item_source.to_lowercase());

        log_debug!(
            "[SDB::Remove::Tool] Using uninstaller for: {}",
            installer_name.cyan()
        );

        if let Some(uninstaller) = self.get_uninstaller(&installer_name) {
            uninstaller.uninstall(uninstall_item)
        } else {
            log_warn!(
                "[SDB::Remove::Tool] Unsupported installer '{}', skipping binary removal",
                installer_name.yellow()
            );
            Ok(())
        }
    }

    /// Removes configuration files associated with a tool.
    ///
    /// When tools are installed with the configuration manager enabled,
    /// their config files are tracked in the state. This method removes
    /// all tracked configuration files.
    ///
    /// # Arguments
    ///
    /// * `tool_state` - State information for the tool
    /// * `tool_name` - Name of the tool (for logging)
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All configs removed or config manager not enabled
    /// * `Err(String)` - Failed to remove one or more config files
    ///
    /// # Path Expansion
    ///
    /// Configuration paths may contain shell variables like `~` for home directory.
    /// These are expanded before attempting removal using the `shellexpand` crate.
    fn remove_tool_configurations(
        &self,
        tool_state: &ToolState,
        tool_name: &str,
    ) -> Result<(), String> {
        // Check if configuration manager was enabled for this tool
        let config_state = match tool_state.get_configuration_manager() {
            Some(state) if state.enabled => state,
            _ => {
                log_debug!(
                    "[SDB::Remove::Config] No config manager for '{}'",
                    tool_name
                );
                return Ok(());
            }
        };

        log_info!(
            "[SDB::Remove::Config] Cleaning configs for '{}'",
            tool_name.cyan()
        );

        // Remove each tracked configuration file
        for dest_path_str in &config_state.tools_configuration_paths {
            // Expand shell variables like ~ in the path
            let dest_path = PathBuf::from(shellexpand::tilde(dest_path_str).to_string());

            if dest_path.exists() {
                fs::remove_file(&dest_path).map_err(|e| {
                    format!("Failed to remove config {}: {}", dest_path.display(), e)
                })?;
                log_info!(
                    "[SDB::Remove::Config] Deleted config: {}",
                    dest_path.display().to_string().cyan()
                );
            } else {
                log_debug!(
                    "[SDB::Remove::Config] Config not found: {}",
                    dest_path.display()
                );
            }
        }

        Ok(())
    }

    /// Removes a font from the system.
    ///
    /// This method handles the complete font removal process:
    /// 1. Locates the font in state
    /// 2. Removes all font files from the fonts directory
    /// 3. Removes the font from state
    /// 4. Removes the font from configuration YAML
    ///
    /// # Arguments
    ///
    /// * `font_name` - Name of the font to remove
    ///
    /// # Returns
    ///
    /// A RemovalResult indicating success, not found, or failure
    ///
    /// # Font File Handling
    ///
    /// Fonts are typically installed as multiple .ttf files (regular, bold, italic, etc.)
    /// all containing the font name. This method removes all matching font files.
    pub fn remove_font(&mut self, font_name: &str) -> RemovalResult {
        log_info!("[SDB::Remove::Font] Removing: {}", font_name.cyan());

        // Step 1: Check if font exists in state
        let font_state = match self.state.fonts.get(font_name) {
            Some(state) => state.clone(),
            None => {
                log_warn!(
                    "[SDB::Remove::Font] Not found in state: {}",
                    font_name.yellow()
                );
                return RemovalResult::NotFound;
            }
        };

        // Step 2: Remove font files from the file system
        if let Err(e) = self.remove_font_files(&font_state) {
            log_error!("[SDB::Remove::Font] Failed to remove files: {}", e.red());
            return RemovalResult::Failed(e);
        }

        // Step 3: Remove from state
        self.state.fonts.remove(font_name);
        log_debug!("[SDB::Remove::Font] Removed from state: {}", font_name);

        // Step 4: Remove from configuration YAML
        if let Err(e) = self
            .cleaner
            .remove_list_item("fonts.yaml", "fonts:", "name:", font_name)
        {
            log_warn!("[SDB::Remove::Font] YAML cleanup warning: {}", e.yellow());
        }

        RemovalResult::Removed
    }

    /// Removes all font files associated with a font.
    ///
    /// # Arguments
    ///
    /// * `font_state` - State information for the font
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All font files removed (or none found)
    /// * `Err(String)` - Failed to access fonts directory or remove files
    ///
    /// # Implementation Details
    ///
    /// Font files are stored in a system fonts directory (typically ~/Library/Fonts
    /// on macOS). This method:
    /// 1. Locates the fonts directory
    /// 2. Searches for all .ttf files containing the font name
    /// 3. Removes each matching file
    ///
    /// If no font files are found, a warning is logged but this is not an error.
    fn remove_font_files(&self, font_state: &FontState) -> Result<(), String> {
        let fonts_dir = PathResolver::get_font_installation_dir()
            .map_err(|e| format!("Failed to get fonts directory: {}", e))?;

        log_debug!("[SDB::Remove::Font] Searching in: {}", fonts_dir.display());

        let mut removed_count = 0;

        // Read all files in the fonts directory
        let entries = fs::read_dir(&fonts_dir).map_err(|e| {
            format!(
                "Failed to read fonts directory {}: {}",
                fonts_dir.display(),
                e
            )
        })?;

        // Find and remove all font files matching this font name
        for entry in entries {
            let entry = entry.map_err(|e| {
                format!(
                    "Failed to read directory entry in {}: {}",
                    fonts_dir.display(),
                    e
                )
            })?;

            let path = entry.path();

            // Check if this is a font file for our font
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Match files containing the font name and ending with .ttf
                    if file_name.contains(&font_state.name) && file_name.ends_with(".ttf") {
                        fs::remove_file(&path).map_err(|e| {
                            format!("Failed to remove font file {}: {}", path.display(), e)
                        })?;
                        log_info!(
                            "[SDB::Remove::Font] Deleted: {}",
                            path.display().to_string().cyan()
                        );
                        removed_count += 1;
                    }
                }
            }
        }

        if removed_count == 0 {
            log_warn!(
                "[SDB::Remove::Font] No font files found for: {}",
                font_state.name.yellow()
            );
        } else {
            log_info!(
                "[SDB::Remove::Font] Removed {} file(s) for: {}",
                removed_count.to_string().cyan(),
                font_state.name.cyan()
            );
        }

        Ok(())
    }
}

// ============================================================================
//                                 REMOVAL SUMMARY
// ============================================================================

impl RemovalSummary {
    /// Displays a formatted summary of all removal operations.
    ///
    /// The summary includes:
    /// - Successfully removed items (with checkmarks)
    /// - Items that were not found (with warning icons)
    /// - Items that failed to remove (with error icons and reasons)
    ///
    /// # Output Format
    ///
    /// ```text
    /// ===================
    ///   Removal Summary
    /// ===================
    /// ✓ tool1
    /// ✓ tool2
    /// Successfully removed tool(s): 2
    ///
    /// ⚠ unknown-tool
    /// Items not found: 1
    ///
    /// ✗ failed-tool - Permission denied
    /// Failed removals: 1
    /// ===================
    /// ```
    pub fn display(&self) {
        println!("\n{}", "=".repeat(19).bright_blue());
        println!("  {}", "Removal Summary".bright_yellow().bold());
        println!("{}", "=".repeat(19).bright_blue());

        // Display successfully removed tools
        if !self.removed_tools.is_empty() {
            for tool in &self.removed_tools {
                println!("  {} {}", "✓".green(), tool.green());
            }
            println!(
                "Successfully removed tool(s): {}\n",
                self.removed_tools.len().to_string().green()
            );
        }

        // Display successfully removed fonts
        if !self.removed_fonts.is_empty() {
            for font in &self.removed_fonts {
                println!("  {} {}", "✓".green(), font.green());
            }
            println!(
                "Successfully removed font(s): {}\n",
                self.removed_fonts.len().to_string().green()
            );
        }

        // Display items that were not found
        if !self.not_found_items.is_empty() {
            for item in &self.not_found_items {
                println!("  {} {}", "⚠".yellow(), item.yellow());
            }
            println!(
                "Items not found: {}\n",
                self.not_found_items.len().to_string().yellow()
            );
        }

        // Display items that failed to remove
        if !self.failed_removals.is_empty() {
            for (item, reason) in &self.failed_removals {
                println!("  {} {} - {}", "✗".red(), item.red(), reason.red());
            }
            println!(
                "Failed removals: {}\n",
                self.failed_removals.len().to_string().red()
            );
        }

        println!("{}\n", "=".repeat(19).bright_blue());
    }
}
