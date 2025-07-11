use colored::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Macros for logging
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => (eprintln!("{} {}", "[INFO]".bright_cyan(), format!($($arg)*)));
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => (eprintln!("{} {}", "[WARN]".yellow(), format!($($arg)*)));
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => (eprintln!("{} {}", "[ERROR]".bright_red(), format!($($arg)*)));
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::logger::is_debug_enabled() {
           eprintln!("{} {}", "[DEBUG]".dimmed(), format!($($arg)*));
        }
    };
}

// Atomic flag to track debug mode globally
static DEBUG_ENABLED: OnceLock<AtomicBool> = OnceLock::new();

/// Initialize logger with debug flag
pub fn init(debug: bool) {
    DEBUG_ENABLED
        .get_or_init(|| AtomicBool::new(debug))
        .store(debug, Ordering::Relaxed);

    if debug {
        log_debug!("Logger initialized in DEBUG mode");
    } else {
        log_info!("Logger initialized in INFO mode");
    }
}

/// Accessor used by macros to check if debug is enabled
pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED
        .get()
        .map(|f| f.load(Ordering::Relaxed))
        .unwrap_or(false)
}
