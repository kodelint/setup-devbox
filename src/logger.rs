use colored::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Macros for logging
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => (println!("{} {}", "[INFO]".on_green(), format!($($arg)*)));
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => (println!("{} {}", "[WARN]".yellow(), format!($($arg)*)));
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => (println!("{} {}", "[ERROR]".red(), format!($($arg)*)));
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if $crate::logger::is_debug_enabled() {
            println!("{} {}", "[DEBUG]".blue(), format!($($arg)*));
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
