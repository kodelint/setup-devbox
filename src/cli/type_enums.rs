use std::fmt;
use std::str::FromStr;

pub use crate::schemas::tools_enums::SourceType;

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
