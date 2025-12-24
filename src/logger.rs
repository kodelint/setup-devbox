// This file implements the application's logging system using tracing.
// It provides macros for different log levels (INFO, WARN, ERROR, DEBUG)
// and handles conditional output via tracing-subscriber.

pub use tracing::{debug, error, info, warn};
use colored::Colorize;
use tracing::{Event, Subscriber};
use tracing::field::{Field, Visit};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Provides convenient logging macros that forward to tracing.
/// `#[macro_export]` makes these macros globally available within the crate.

// `log_info!` for general application progress and informational messages.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => ($crate::logger::info!($($arg)*));
}

// `log_warn!` for non-critical issues or noteworthy conditions.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => ($crate::logger::warn!($($arg)*));
}

// `log_error!` for critical errors requiring immediate attention.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => ($crate::logger::error!($($arg)*));
}

// `log_debug!` for detailed internal application tracing.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => ($crate::logger::debug!($($arg)*));
}

struct SimpleFormatter;

impl<S, N> FormatEvent<S, N> for SimpleFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let level = *event.metadata().level();
        let level_str = match level {
            tracing::Level::TRACE => "[TRACE]".dimmed(),
            tracing::Level::DEBUG => "[DEBUG]".dimmed(),
            tracing::Level::INFO => "[INFO]".bright_green(),
            tracing::Level::WARN => "[WARN]".bright_yellow(),
            tracing::Level::ERROR => "[ERROR]".bright_red(),
        };

        // Write level
        write!(writer, "{} ", level_str)?;

        // Write message using custom visitor
        let mut visitor = MessageVisitor { writer: &mut writer };
        event.record(&mut visitor);

        writeln!(writer)
    }
}

struct MessageVisitor<'a> {
    // Use dyn Write to avoid double borrow/lifetime issues with specific Writer type
    writer: &'a mut dyn std::fmt::Write,
}

impl<'a> Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // Debug implementation for fmt::Arguments (which is what message is)
            // simply writes the formatted string, preserving ANSI codes.
            let _ = write!(self.writer, "{:?}", value);
        }
    }
}

/// Initializes the logger, setting the global debug mode.
/// This function should be called once at application startup.
///
/// # Arguments
/// * `debug`: If `true`, enables debug logging; otherwise, only info, warn, and error messages are printed.
pub fn init(debug: bool) {
    let filter = if debug {
        tracing_subscriber::filter::LevelFilter::DEBUG
    } else {
        tracing_subscriber::filter::LevelFilter::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(filter)
        .event_format(SimpleFormatter)
        .with_writer(std::io::stderr)
        .init();
}
