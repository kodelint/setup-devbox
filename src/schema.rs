// src/schema.rs
// This file is essentially the blueprint for all the different configuration files
// and the internal state that our `setup-devbox` application will use.
// Think of it as defining the "language" for how we describe tools, settings, fonts, and more!

// We're bringing in 'serde' traits here.
// 'Deserialize' means we can take data from external files (like YAML or JSON)
// and turn it into these Rust structures.
// 'Serialize' means we can take our Rust structures and write them out to files.
use serde::{Deserialize, Serialize};
// 'HashMap' is a super useful data structure from Rust's standard library.
// It allows us to store data as key-value pairs, which is perfect for
// things like looking up tools by their name, or organizing settings by OS.
use std::collections::HashMap;

// GitHub API Response Schemas
// These two structs are specifically designed to match the JSON response we get back
// when querying the GitHub API for release information.

/// This struct represents a single downloadable file (an "asset") attached to a GitHub release.
/// When we ask GitHub for the details of a software release, it often lists multiple files
/// that you can download (like different versions for Windows, macOS, Linux, etc.).
#[derive(Debug, Deserialize)] // We derive 'Debug' to easily print its contents for debugging,
// and 'Deserialize' to parse it from GitHub's JSON.
pub struct ReleaseAsset {
    // The actual name of the file, e.g., "my-tool-macos-amd64.tar.gz".
    pub(crate) name: String,
    // This is the direct URL you'd use in your browser (or with an HTTP client!)
    // to download this specific file. Super handy!
    pub(crate) browser_download_url: String,
}

/// This struct captures the overall details of a single GitHub release.
/// A release often contains a collection of assets (the actual downloadable files).
#[derive(Debug, Deserialize)] // Again, 'Debug' for introspection and 'Deserialize' for JSON parsing.
pub struct Release {
    // This `Vec` (which is Rust's way of saying "vector" or dynamic array)
    // will hold a list of all the `ReleaseAsset`s available for this particular release.
    // It's like a shopping list of all the files bundled with this version.
    pub(crate) assets: Vec<ReleaseAsset>,
}

// User Configuration File Schemas (e.g., for YAML files)
// These structs define the structure for the configuration files that our users will write
// to tell `setup-devbox` what tools, settings, and fonts they want managed.

/// Configuration schema for `tools.yaml`.
/// This is the top-level structure for the file where users list all the software tools
/// they want `setup-devbox` to manage and install for them.
#[derive(Debug, Serialize, Deserialize)] // We need both 'Serialize' and 'Deserialize' because
// we'll read these from a file and potentially write them too (though less common for config).
pub struct ToolConfig {
    // This `tools` field will hold a list of individual `ToolEntry` structs.
    // Each `ToolEntry` describes one specific tool the user wants.
    pub tools: Vec<ToolEntry>,
}

/// Represents a single tool entry as defined by the user in the `tools.yaml` file.
/// This is where the user specifies *which* tool, *what version*, and *how* to get it.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolEntry {
    // The human-readable name of the tool (e.g., "terraform", "kubectl").
    pub name: String,
    // The desired version of the tool. It's an `Option<String>` because the user
    // might not specify a version, in which case we'd probably try to get the latest.
    pub version: Option<String>,
    // This crucial field tells `setup-devbox` *where* to get the tool from.
    // Examples: "github", "brew" (for Homebrew), "go" (for Go binaries), "cargo" (for Rust crates).
    pub source: String,
    // If the `source` is "GitHub", this `Option<String>` holds the GitHub repository
    // in the "owner/repo_name" format (e.g., "hashicorp/terraform").
    pub repo: Option<String>,
    // Sometimes, a specific release on GitHub isn't directly the version number,
    // but rather a tag name. This field allows specifying a particular tag to download from.
    pub tag: Option<String>,
    // If the downloaded executable has a generic name (e.g., "cli") but the user
    // wants it to be called something specific (e.g., "mycli"), this field allows renaming.
    pub rename_to: Option<String>,
}

/// Configuration schema for `shellac.yaml`.
/// This file is all about customizing the user's shell environment,
/// including shell-specific configurations and command aliases.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    // This block describes the core shell configuration, like the shell type and raw commands.
    pub shellrc: ShellRc,
    // This `Vec` will contain a list of custom command aliases the user wants to set up.
    pub aliases: Vec<AliasEntry>,
}

/// Represents the `shellrc` block within `shellac.yaml`.
/// This section is for fundamental shell settings that might be different for Bash, Zsh, etc.
#[derive(Debug, Serialize, Deserialize)]
pub struct ShellRc {
    // The type of shell being configured (e.g., "bash", "zsh", "fish").
    pub shell: String,
    // A list of raw command lines that will be directly appended or sourced into the shell's
    // configuration file (like `.bashrc` or `.zshrc`). This allows for highly custom setups.
    pub raw_configs: Vec<String>,
}

/// Represents a single command alias entry in `shellac.yaml`.
/// Aliases are shortcuts for longer commands, making life easier in the terminal.
#[derive(Debug, Serialize, Deserialize)]
pub struct AliasEntry {
    // The short name for the alias (e.g., "gco").
    pub name: String,
    // The full command that the alias expands to (e.g., "git checkout").
    pub value: String,
}

/// Configuration schema for `fonts.yaml`.
/// This file lets users specify custom fonts they want `setup-devbox` to manage and install.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontConfig {
    // This `Vec` will hold a list of individual `FontEntry` structs, each describing one font.
    pub fonts: Vec<FontEntry>,
}

/// Represents a single font entry as defined by the user in `fonts.yaml`.
/// Similar to `ToolEntry`, it describes what font, what version, and where to get it.
#[derive(Debug, Serialize, Deserialize)]
pub struct FontEntry {
    // The name of the font (e.g., "FiraCode", "JetBrainsMono").
    pub name: String,
    // The desired version of the font. Often fonts don't have explicit versions like software,
    // but it's good to include for consistency or specific releases.
    pub version: Option<String>,
    // The source from where the font can be obtained (e.g., "github", "nerdfonts").
    pub source: String,
    // If the `source` is "GitHub", this is the repository (e.g., "ryanoasis/nerd-fonts").
    pub repo: Option<String>,
    // A specific tag or release on GitHub to download the font from.
    pub tag: Option<String>,
}

/// Configuration schema for `settings.yaml`.
/// This file allows users to define system-level settings (e.g., macOS defaults)
/// that `setup-devbox` should apply.
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsConfig {
    // This is a `HashMap` where the key is the operating system (e.g., "macos", "linux")
    // and the value is a list of `SettingEntry` structs. This allows users to
    // specify OS-specific settings.
    pub settings: HashMap<String, Vec<SettingEntry>>, // Key: OS name (e.g., "macos"), Value: List of settings for that OS
}

/// Represents a single system setting to be applied, typically for macOS defaults commands.
/// This allows for granular control over system behavior.
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingEntry {
    // The domain of the setting (e.g., "com.apple.finder" for Finder settings).
    pub domain: String,
    // The specific key within that domain (e.g., "AppleShowAllFiles" to show hidden files).
    pub key: String,
    // The value to set for that key (e.g., "true", "false", "Always").
    pub value: String,
    // This is a special field! `#[serde(rename = "type")]` tells Serde to expect a key
    // named "type" in the YAML/JSON, even though our Rust field is called `value_type`.
    // This is often done when the external format uses a keyword that's reserved in Rust.
    // This field specifies the data type of the `value` (e.g., "string", "bool", "integer").
    #[serde(rename = "type")]
    pub value_type: String,
}

// Application State File Schema (state.json)
// These structs define the structure of `setup-devbox`s internal state file (`state.json`).
// This file is crucial for `setup-devbox` to remember what it has installed and configured,
// so it doesn't try to re-install things or re-apply settings unnecessarily.

/// The complete structure of `state.json`, which acts as `setup-devbox`s memory.
/// It records everything `setup-devbox` has done – what tools are installed, settings applied, etc.
/// 'Clone' is derived here because we might need to easily duplicate this state object in memory.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DevBoxState {
    // A `HashMap` to keep track of all installed tools.
    // The key is likely the tool's name, and the value is its `ToolState` details.
    pub tools: HashMap<String, ToolState>,
    // A `HashMap` to record which settings have been applied.
    // The key could be a combination of domain and key, or just the domain, and the value is its `SettingState`.
    pub settings: HashMap<String, SettingState>,
    // A `HashMap` to store information about installed fonts.
    // Key: font name, Value: `FontState` details.
    pub fonts: HashMap<String, FontState>,
}

/// Stores detailed information about each tool that `setup-devbox` has successfully installed.
/// This helps `setup-devbox` know if a tool is already there, where it is, and how it was installed.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolState {
    // The exact version of the tool that was installed.
    pub version: String,
    // The file system path where the tool's executable or main directory is located.
    pub install_path: String,
    // A boolean flag indicating whether this tool was installed by `setup-devbox` itself.
    // This is useful for distinguishing between tools `setup-devbox` manages and those installed manually.
    pub installed_by_devbox: bool,
    // The method used to install the tool (e.g., "github-release", "brew", "go-install").
    pub install_method: String,
    // If the tool's executable was renamed during installation, this stores the new name.
    pub renamed_to: Option<String>,
    // The type of package (e.g., "binary", "go-module", "brew-formula").
    pub package_type: String,
}

/// Records the state of a single system setting that `setup-devbox` has applied.
/// This helps `setup-devbox` avoid re-applying settings that are already in place.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingState {
    // The domain of the setting (e.g., "com.apple.finder").
    pub domain: String,
    // The specific key of the setting (e.g., "AppleShowAllFiles").
    pub key: String,
    // The value that was set for this key.
    pub value: String,
    // Note: The `value_type` from `SettingEntry` isn't stored here.
    // The state just records *what* was set, assuming the type was handled during application.
}

/// Records the state of a single font that `setup-devbox` has installed.
/// This helps `setup-devbox` know which fonts are managed and where they came from.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontState {
    // The name of the font (e.g., "FiraCode Nerd Font").
    pub name: String,
    // The original URL from which the font was downloaded. Useful for re-downloads or verification.
    pub url: String,
    // A list of the actual font files (e.g., .ttf, .otf) that were installed for this font.
    pub files: Vec<String>,
}

//  Main Application Configuration

/// This struct defines the main configuration file for the `setup-devbox` application itself.
/// It tells `setup-devbox` *where* to find the other detailed configuration files (tools, settings, etc.).
/// This allows for a flexible file structure where users can organize their configs.
#[derive(Debug, Serialize, Deserialize)]
pub struct MainConfig {
    // An optional path to the `tools.yaml` file. If not specified, `setup-devbox` might look for a default.
    pub tools: Option<String>,
    // An optional path to the `settings.yaml` file.
    pub settings: Option<String>,
    // An optional path to the `shellac.yaml` file (for shell configs and aliases).
    pub shellrc: Option<String>,
    // An optional path to the `fonts.yaml` file.
    pub fonts: Option<String>,
}