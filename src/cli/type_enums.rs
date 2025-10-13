use std::fmt;
use std::str::FromStr;
/// Defines the set of valid configuration types that can be edited.
/// Each variant corresponds to a specific configuration file.
#[derive(Debug, Clone)]
pub enum ConfigType {
    Tools,    // tools.yaml - Tool definitions and installation specifications
    Fonts,    // fonts.yaml - Font specifications and installation details
    Shell,    // shellrc.yaml - Shell aliases and configuration snippets
    Settings, // settings.yaml - System settings and preferences
}

/// Implementation of string parsing for ConfigType enum.
/// Allows converting string arguments to strongly-typed ConfigType values.
impl FromStr for ConfigType {
    type Err = String;

    /// Parses a string into a ConfigType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(ConfigType)` if the string matches a valid configuration type
    /// * `Err(String)` with error message if no match found
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tools" => Ok(ConfigType::Tools),
            "fonts" => Ok(ConfigType::Fonts),
            "shell" => Ok(ConfigType::Shell),
            "settings" => Ok(ConfigType::Settings),
            _ => {
                let valid_types = ["tools", "fonts", "shell", "settings"].join(", ");
                Err(format!(
                    "Invalid config type '{s}'. Must be one of: {valid_types}",
                ))
            }
        }
    }
}

/// Implementation of display formatting for ConfigType enum.
/// Provides human-readable string representation for each configuration type.
impl fmt::Display for ConfigType {
    /// Formats the ConfigType as a string for display purposes.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigType::Tools => write!(f, "tools"),
            ConfigType::Fonts => write!(f, "fonts"),
            ConfigType::Shell => write!(f, "shell"),
            ConfigType::Settings => write!(f, "settings"),
        }
    }
}

/// Defines the set of valid installation/source methods for tools.
/// Each variant corresponds to a different installation backend or package manager.
#[derive(Debug, Clone)]
pub enum SourceType {
    Brew,   // Homebrew package manager (macOS/Linux)
    Cargo,  // Rust package manager
    Github, // GitHub releases and repositories
    Go,     // Go language tooling
    Rustup, // Rust toolchain manager
    Url,    // Direct URL downloads
    Uv,     // Python package manager
    Pip,    // Python package installer
}

/// Implementation of string parsing for SourceType enum.
/// Allows converting string arguments to strongly-typed SourceType values.
impl FromStr for SourceType {
    type Err = String;

    /// Parses a string into a SourceType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(SourceType)` if the string matches a valid source type
    /// * `Err(String)` with error message if no match found
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "brew" => Ok(SourceType::Brew),
            "cargo" => Ok(SourceType::Cargo),
            "github" => Ok(SourceType::Github),
            "go" => Ok(SourceType::Go),
            "rustup" => Ok(SourceType::Rustup),
            "url" => Ok(SourceType::Url),
            "uv" => Ok(SourceType::Uv),
            "pip" => Ok(SourceType::Pip),
            _ => {
                let valid_types = [
                    "brew", "cargo", "github", "go", "rustup", "url", "uv", "pip",
                ]
                .join(", ");
                Err(format!(
                    "Invalid source type '{s}'. Must be one of: {valid_types}"
                ))
            }
        }
    }
}

/// Implementation of display formatting for SourceType enum.
/// Provides human-readable string representation for each source type.
impl fmt::Display for SourceType {
    /// Formats the SourceType as a string for display purposes.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SourceType::Brew => write!(f, "brew"),
            SourceType::Cargo => write!(f, "cargo"),
            SourceType::Github => write!(f, "github"),
            SourceType::Go => write!(f, "go"),
            SourceType::Rustup => write!(f, "rustup"),
            SourceType::Url => write!(f, "url"),
            SourceType::Uv => write!(f, "uv"),
            SourceType::Pip => write!(f, "pip"),
        }
    }
}

/// Defines the set of valid configuration value types for system settings.
/// Used for type-safe serialization and validation of setting values.
#[derive(Debug, Clone)]
pub enum ValueType {
    Bool,   // Boolean values (true/false)
    String, // String values
    Int,    // Integer values
    Float,  // Floating-point values
}

/// Implementation of string parsing for ValueType enum.
/// Allows converting string arguments to strongly-typed ValueType values.
impl FromStr for ValueType {
    type Err = String;

    /// Parses a string into a ValueType enum variant.
    ///
    /// # Arguments
    /// * `s` - The string to parse (case-insensitive)
    ///
    /// # Returns
    /// * `Ok(ValueType)` if the string matches a valid value type
    /// * `Err(String)` with error message if no match found
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bool" => Ok(ValueType::Bool),
            "string" => Ok(ValueType::String),
            "int" => Ok(ValueType::Int),
            "float" => Ok(ValueType::Float),
            _ => {
                let valid_types = ["bool", "string", "int", "float"].join(", ");
                Err(format!(
                    "Invalid value type '{s}'. Must be one of: {valid_types}"
                ))
            }
        }
    }
}

/// Implementation of display formatting for ValueType enum.
/// Provides human-readable string representation for each value type.
impl fmt::Display for ValueType {
    /// Formats the ValueType as a string for display purposes.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueType::Bool => write!(f, "bool"),
            ValueType::String => write!(f, "string"),
            ValueType::Int => write!(f, "int"),
            ValueType::Float => write!(f, "float"),
        }
    }
}
