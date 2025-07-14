// This is the main module file for the `utils` directory.
// It declares the submodules within the `utils` directory and
// re-exports their public items, making them accessible from `crate::utils::*`.

// Declare the `path_helpers` module.
pub mod path_helpers;
// Declare the `compression` module.
pub mod compression;
pub mod platform;
pub mod assets;
pub mod binary;
