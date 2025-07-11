// This file is the voice of our `setup-devbox` application.
// Think of it as our app's dedicated communication expert, handling all the messages
// it wants to tell you, whether it's good news (INFO), a gentle heads-up (WARN),
// something went wrong (ERROR), or detailed behind-the-scenes chatter (DEBUG).
// It makes sure our messages are not just informative but also a little bit
// friendly and easy to read in your terminal, using colors to guide your eyes!

// We're bringing in the `colored` crate here. This is our little magic wand
// for making terminal output vibrant and readable. It allows us to add
// beautiful colors and styling (like bold or dimmed text) to our log messages.
use colored::*;
// `AtomicBool` and `Ordering` are like super-reliable, high-speed messengers
// that let us safely share and update a simple `true`/`false` flag across
// different parts of our program at the same time, without any confusion.
// In our case, it's used for the global `DEBUG_ENABLED` flag.
use std::sync::atomic::{AtomicBool, Ordering};
// `OnceLock` is a clever mechanism that guarantees a value is initialized
// exactly once, and then safely provides access to that same value forever after.
// It's perfect for our `DEBUG_ENABLED` flag, ensuring it's set up correctly
// the very first time `init` is called and never accidentally re-initialized.
use std::sync::OnceLock;

/// These are our super handy logging macros!
/// Think of macros as intelligent templates that generate code for us.
/// Instead of writing `eprintln!` and color logic repeatedly, we just call
/// `log_info!`, `log_warn!`, etc., and the macro expands into the full,
/// properly formatted logging code.
///
/// `#[macro_export]` makes these macros available *everywhere* in our crate
/// (and even to other crates that depend on ours), so we can easily log
/// from any part of the `setup-devbox` application.

// `log_info!` is for general messages that tell you what's happening.
// It's like our app cheerfully announcing its progress!
// We use `bright_cyan` for a friendly, informative blue-green touch.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => (eprintln!("{} {}", "[INFO]".bright_cyan(), format!($($arg)*)));
}

// `log_warn!` is for situations that aren't critical errors but might
// be worth your attention. It's like a gentle tap on the shoulder from our app.
// The `yellow` color is perfect for a subtle warning.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => (eprintln!("{} {}", "[WARN]".yellow(), format!($($arg)*)));
}

// `log_error!` is reserved for when something has gone wrong and needs
// your immediate attention. This is our app waving a big red flag!
// `bright_red` ensures it stands out powerfully in the terminal.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => (eprintln!("{} {}", "[ERROR]".bright_red(), format!($($arg)*)));
}

// `log_debug!` is for super detailed messages that are mostly helpful
// when we're trying to figure out *why* something isn't working or
// just to trace the app's internal logic. It's like our app talking to itself,
// but you can listen in if you want!
// It only prints if debug mode is explicitly enabled (using the `is_debug_enabled()` check).
// The `dimmed` color makes it less intrusive, as these logs can be very verbose.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        // This 'if' check is the gatekeeper! Debug messages only appear if the
        // global `DEBUG_ENABLED` flag is set to true.
        if $crate::logger::is_debug_enabled() {
           eprintln!("{} {}", "[DEBUG]".dimmed(), format!($($arg)*));
        }
    };
}

// This is our global switch for debug mode. It's an `AtomicBool` wrapped in a `OnceLock`.
// `OnceLock` makes sure it's set up exactly once, and `AtomicBool` allows us to
// check and change its value safely from anywhere in our program, even if multiple
// parts of the program try to access it at the same time (which is common in Rust!).
static DEBUG_ENABLED: OnceLock<AtomicBool> = OnceLock::new();

/// This function is your "on/off" switch for debug logging.
/// You call `logger::init(true)` to turn on debug messages,
/// or `logger::init(false)` to keep them off (defaulting to info/warn/error only).
///
/// It's designed to be called once, usually at the very beginning of your application's startup.
///
/// # Arguments
/// * `debug`: A simple `bool` that dictates whether debug messages should be printed (`true`) or not (`false`).
pub fn init(debug: bool) {
    // Here's the magic of `OnceLock`:
    // `get_or_init` tries to get the value. If it's already been set, it returns the existing one.
    // If it hasn't been set *yet*, it calls our provided closure (`|| AtomicBool::new(debug)`)
    // to create a new `AtomicBool` with the initial `debug` value you pass in, and then
    // stores it for future use.
    DEBUG_ENABLED
        .get_or_init(|| AtomicBool::new(debug))
        // `.store(debug, Ordering::Relaxed)` then updates the `AtomicBool` with the
        // current `debug` value. `Ordering::Relaxed` means we don't need super strict
        // memory synchronization for this simple flag, keeping it fast.
        .store(debug, Ordering::Relaxed);

    // Just a friendly message to confirm the logger's status!
    if debug {
        log_debug!("Logger initialized in DEBUG mode");
    } else {
        log_info!("Logger initialized in INFO mode");
    }
}

/// This is a tiny helper function, primarily used by our `log_debug!` macro.
/// It simply checks the global `DEBUG_ENABLED` flag to see if debug logging is currently active.
///
/// # Returns
/// * `true` if debug logging is enabled globally, `false` otherwise.
pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED
        // `get()` attempts to retrieve the `AtomicBool` from our `OnceLock`.
        // It returns an `Option<&AtomicBool>`, which will be `None` if `init` hasn't been called yet.
        .get()
        // `.map(|f| f.load(Ordering::Relaxed))` will execute `f.load(Ordering::Relaxed)`
        // ONLY if `get()` returned `Some(f)`. It loads the current boolean value from the `AtomicBool`.
        .map(|f| f.load(Ordering::Relaxed))
        // `.unwrap_or(false)` is our fallback: if `init` was never called (meaning `get()` returned `None`),
        // we default to assuming debug is *not* enabled, which is a safe default.
        .unwrap_or(false)
}