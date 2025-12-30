use thiserror::Error;

#[derive(Error, Debug)]
pub enum InstallerError {
    #[error("Platform detection failed")]
    PlatformDetectionFailed,

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
