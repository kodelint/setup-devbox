//! # Tool Installation Orchestrator
//!
//! This module serves as the central orchestrator for the tool installation pipeline.
//! It is responsible for determining the correct action for each tool (install, update,
//! or skip), invoking the appropriate installer, and managing the tool's state and
//! configuration. It integrates with various other modules to perform tasks such as
//! state management, path resolution, and command execution.
//!
//! ## Core Responsibilities
//!
//! - **Result Processing**: Collects and categorizes installation results for reporting
//!
//! ## Installation Pipeline
//!
//! The orchestrator follows a structured pipeline for each tool:
//! 1. **Reporting**: Categorize and display results to the user

use crate::schemas::tools::{InstallationSummary, ToolProcessingResult};
use crate::{log_error, log_info};
use colored::Colorize;

// ============================================================================
// INSTALLATION SUMMARY IMPLEMENTATION
// ============================================================================

impl InstallationSummary {
    /// Creates a new `InstallationSummary` from the raw `ToolProcessingResult`s.
    ///
    /// ## Parameters
    /// - `results`: Vector of tool names and their processing results
    ///
    /// ## Returns
    /// `InstallationSummary` with categorized results
    pub(crate) fn from_processing_results(results: Vec<(String, ToolProcessingResult)>) -> Self {
        let mut summary = Self {
            installed_tools: Vec::new(),
            updated_tools: Vec::new(),
            configuration_updated_tools: Vec::new(),
            skipped_tools: Vec::new(),
            configuration_skipped_tools: Vec::new(),
            failed_tools: Vec::new(),
        };

        // Categorize each result into the appropriate vector.
        for (tool_name, result) in results {
            match result {
                ToolProcessingResult::Installed => summary.installed_tools.push(tool_name),
                ToolProcessingResult::Updated => summary.updated_tools.push(tool_name),
                ToolProcessingResult::ConfigurationUpdated => {
                    summary.configuration_updated_tools.push(tool_name)
                }
                ToolProcessingResult::Skipped(reason) => {
                    summary.skipped_tools.push((tool_name, reason))
                }
                ToolProcessingResult::ConfigurationSkipped(reason) => summary
                    .configuration_skipped_tools
                    .push((tool_name, reason)),
                ToolProcessingResult::Failed(reason) => {
                    summary.failed_tools.push((tool_name, reason))
                }
            }
        }
        summary
    }

    /// Checks if any state-changing operations (install, update, config update) occurred.
    /// This is used to decide whether the state file needs to be saved.
    ///
    /// ## Returns
    /// `true` if any state changes occurred, `false` otherwise
    pub(crate) fn has_state_changes(&self) -> bool {
        !self.installed_tools.is_empty()
            || !self.updated_tools.is_empty()
            || !self.configuration_updated_tools.is_empty()
    }

    /// Prints the complete summary to the console.
    pub(crate) fn display_summary(&self) {
        self.display_skipped_tools();
        self.display_configuration_skipped_tools();
        self.display_failed_tools();
        self.display_success_summary();
    }

    /// Prints a formatted list of skipped tools.
    fn display_skipped_tools(&self) {
        if self.skipped_tools.is_empty() {
            return;
        }

        println!();
        println!(
            "{} Skipped tools (already up to date) {}",
            "============".blue(),
            "=============".blue()
        );

        for tools_chunk in self.skipped_tools.chunks(3) {
            let tools_line = tools_chunk
                .iter()
                .map(|(name, _)| name.bright_yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("[Skipped Install] {tools_line}");
        }
        println!("{}", "=".repeat(61).blue());
    }

    /// Prints a formatted list of tools where configuration syncing was skipped.
    fn display_configuration_skipped_tools(&self) {
        if self.configuration_skipped_tools.is_empty() {
            return;
        }

        println!();
        println!(
            "{} Skipped Configuration Sync (already up to date) {}",
            "======".blue(),
            "======".blue()
        );

        for tools_chunk in self.configuration_skipped_tools.chunks(3) {
            let tools_line = tools_chunk
                .iter()
                .map(|(name, _)| name.bright_yellow().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("[Skipped Configuration] {tools_line}");
        }
        println!("{}\n", "=".repeat(61).blue());
    }

    /// Prints a formatted list of failed tool installations.
    fn display_failed_tools(&self) {
        if self.failed_tools.is_empty() {
            return;
        }

        println!();
        log_error!("[SDB::Tools] Failed installations:");
        for (tool_name, failure_reason) in &self.failed_tools {
            log_error!("  {} - {}", tool_name.red().bold(), failure_reason.red());
        }
    }

    /// Prints a final summary of successfully processed tools.
    fn display_success_summary(&self) {
        let total_processed = self.installed_tools.len()
            + self.updated_tools.len()
            + self.configuration_updated_tools.len();

        if total_processed > 0 {
            log_info!(
                "[SDB::Tools] Successfully processed {} tool(s)",
                total_processed
            );
        }
    }
}
