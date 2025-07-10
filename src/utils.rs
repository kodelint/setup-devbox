use std::path::PathBuf;

/// Expand tilde (~) in paths to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            return PathBuf::from(path.replacen("~", &home.to_string_lossy(), 1));
        }
    }
    PathBuf::from(path)
}