// This module acts as the central hub for the `installers` crate,
// publicly exposing submodules that handle the installation logic
// for various types of tools and packages.
//
// It serves as an organizational unit, making it clear which installation
// methods are supported by `setup-devbox` and allowing for easy extension
// with new installer types in the future.

// `pub(crate) mod <module_name>;` makes the module and its contents
// accessible within the current crate (`setup-devbox`), but not to external crates
// that might depend on `setup-devbox`. This is standard practice for internal
// utility modules.

/// Declares the `GitHub` module, responsible for installing tools
/// distributed as GitHub releases. This includes logic for fetching
/// release assets, downloading, extracting, and placing binaries.
pub(crate) mod github;

/// Declares the `brew` module, which handles the installation of tools
/// via the Homebrew package manager (for macOS and Linux).
/// It wraps calls to the `brew` command-line utility.
pub(crate) mod brew;

/// Declares the `go` module, intended for installing Go-based tools.
/// This typically involves using `go install` or similar Go toolchain commands.
pub(crate) mod go;

/// Declares the `cargo` module, dedicated to installing Rust crates
/// using the `cargo install` command. It integrates Rust toolchain
/// installations into the `devbox` ecosystem.
pub(crate) mod cargo;

/// Declares the `fonts` module, specifically designed for installing
/// font files from various sources (currently GitHub releases) onto
/// the user's system, especially on macOS.
pub(crate) mod fonts;

/// Declares the `shellrc` module, for managing or injecting
/// configurations into shell startup files (e.g., `.bashrc`, `.zshrc`)
/// after a tool has been installed, to ensure it's in the PATH or
/// has necessary environment variables set.
pub(crate) mod shellrc;

/// Declares the `rustup` module, which would handle the installation
/// and management of the Rust toolchain itself, using the `rustup` installer.
/// This is distinct from `cargo` which installs Rust *applications*.
pub(crate) mod rustup;

/// Declares the `pip` module, for installing Python packages (tools)
/// using the `pip` package installer. It would manage Python dependencies
/// and script installations.
pub(crate) mod pip;
/// Declares the `url` module, which handles the installation of tools
/// by directly downloading files from a specified URL. This is used for
/// binaries or installers not managed by other package managers or GitHub releases.
pub(crate) mod url;