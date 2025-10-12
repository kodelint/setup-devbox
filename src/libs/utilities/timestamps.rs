use chrono::{DateTime, Duration, Utc};
use std::str::FromStr;

/// Converts a Chrono `Duration` object into a human-readable string representation.
///
/// This function formats time durations for display purposes, selecting the most
/// appropriate time unit (days, hours, or minutes) based on the duration's magnitude.
/// It's particularly useful for user-facing messages, logs, and configuration displays
/// where raw duration values would be less intuitive.
///
/// # Arguments
/// * `duration` - A reference to a Chrono `Duration` object to be formatted
///
/// # Returns
/// A `String` containing the formatted duration in the most appropriate time unit:
/// - Days for durations ≥ 1 day
/// - Hours for durations ≥ 1 hour but less than 1 day
/// - Minutes for durations ≥ 1 minute but less than 1 hour
/// - "0 minutes" for durations less than 1 minute
///
/// # Unit Selection Logic
/// The function uses a hierarchical approach to determine the best unit:
/// 1. **Days**: If the duration contains any complete days (≥ 86400 seconds)
/// 2. **Hours**: If no days but contains complete hours (≥ 3600 seconds)
/// 3. **Minutes**: If no hours but contains complete minutes (≥ 60 seconds)
/// 4. **Fallback**: "0 minutes" for sub-minute durations
pub fn format_duration(duration: &Duration) -> String {
    // Check if the duration contains any complete days
    // Using num_days() which returns the total number of whole days in the duration
    if duration.num_days() > 0 {
        format!("{} days", duration.num_days())
    }
    // If no days, check for complete hours
    // num_hours() returns total whole hours, including those that might be part of days
    else if duration.num_hours() > 0 {
        format!("{} hours", duration.num_hours())
    }
    // If no hours, check for complete minutes
    // num_minutes() returns total whole minutes in the duration
    else if duration.num_minutes() > 0 {
        format!("{} minutes", duration.num_minutes())
    } else {
        // Fallback for durations less than 1 minute
        // This ensures we always return a meaningful string, even for very short durations
        "0 minutes".to_string()
    }
}

/// Returns the current timestamp in RFC 3339 format (ISO 8601).
///
/// This function provides a standardized, human-readable timestamp string
/// that includes timezone information. The RFC 3339 format is ideal for
/// serialization and storage as it's both machine-parsable and human-readable.
///
/// # Returns
/// A `String` containing the current UTC timestamp in RFC 3339 format.
/// Example: "2023-12-07T10:30:45.123456789+00:00"
///
/// # Examples
/// ```
/// let timestamp = current_timestamp();
/// println!("Current time: {}", timestamp); // e.g., "2023-12-07T10:30:45.123456789+00:00"
/// ```
pub fn current_timestamp() -> String {
    use chrono::Utc;
    // Get the current UTC datetime and format it according to RFC 3339
    // This includes fractional seconds and timezone offset
    Utc::now().to_rfc3339()
}

/// Parses a human-readable duration string into a Chrono `Duration` object.
///
/// This function converts natural language duration specifications into
/// a precise time duration that can be used for time calculations and comparisons.
///
/// # Arguments
/// * `duration_str` - A string slice containing the duration specification.
///   Expected format: "`<amount>` `<unit>`" (e.g., "7 days", "1 hour", "30 minutes")
///
/// # Returns
/// * `Some(Duration)` - If parsing was successful
/// * `None` - If the input format is invalid or contains unsupported units
///
/// # Supported Units
/// - "day" or "days" (converted to `Duration::days`)
/// - "hour" or "hours" (converted to `Duration::hours`)
/// - "minute" or "minutes" (converted to `Duration::minutes`)
///
/// # Examples
/// ```
/// let duration = parse_duration("7 days");
/// assert!(duration.is_some());
///
/// let invalid = parse_duration("soon");
/// assert!(invalid.is_none());
/// ```
pub fn parse_duration(duration_str: &str) -> Option<Duration> {
    // Split the input string by whitespace to separate amount from unit
    let parts: Vec<&str> = duration_str.split_whitespace().collect();

    // Validate the expected format: exactly two parts (amount and unit)
    if parts.len() != 2 {
        return None; // Invalid format - wrong number of components
    }

    // Parse the numeric amount from the first part
    let amount = i64::from_str(parts[0]).ok()?; // Returns None if parsing fails

    // Normalize the unit to lowercase for case-insensitive matching
    let unit = parts[1].to_lowercase();

    // Match the unit string to appropriate Duration constructor
    match unit.as_str() {
        "day" | "days" => Some(Duration::days(amount)),
        "hour" | "hours" => Some(Duration::hours(amount)),
        "minute" | "minutes" => Some(Duration::minutes(amount)),
        _ => None, // Unsupported time unit
    }
}

/// Determines if a given RFC 3339 timestamp is older than a specified duration.
///
/// This function is essential for implementing update policies where tools
/// with version "latest" should only be updated after a certain time has elapsed
/// since their last installation/update.
///
/// # Arguments
/// * `timestamp` - An RFC 3339 formatted timestamp string to check
/// * `duration` - Reference to a Chrono `Duration` representing the threshold
///
/// # Returns
/// * `true` - If the timestamp is older than the specified duration OR
///   if the timestamp cannot be parsed (error-safe default)
/// * `false` - If the timestamp is newer than or equal to the duration threshold
///
/// # Error Handling
/// If the timestamp cannot be parsed (invalid format), the function returns `true`
/// as a safety measure, assuming the tool should be updated to establish a proper
/// timestamp record.
///
/// # Examples
/// ```
/// let old_timestamp = "2023-01-01T00:00:00Z";
/// let new_timestamp = current_timestamp();
/// let one_day = Duration::days(1);
///
/// assert!(is_timestamp_older_than(old_timestamp, &one_day));
/// assert!(!is_timestamp_older_than(new_timestamp, &one_day));
/// ```
pub fn is_timestamp_older_than(timestamp: &str, duration: &Duration) -> bool {
    // Attempt to parse the RFC 3339 timestamp string into a DateTime object
    if let Ok(parsed_time) = DateTime::parse_from_rfc3339(timestamp) {
        // Convert the parsed time to UTC timezone for consistent comparison
        let time_utc = parsed_time.with_timezone(&Utc);
        let now = Utc::now();

        // Calculate the time elapsed since the timestamp and compare with threshold
        now - time_utc > *duration
    } else {
        // If timestamp parsing fails, adopt a conservative approach:
        // assume the timestamp is old and requires update.
        // This ensures tools with corrupted or missing timestamp data get updated
        // to establish a proper timestamp record.
        true
    }
}

/// Converts an RFC 3339 timestamp into a human-readable relative time string.
///
/// This function provides user-friendly time descriptions like "2 days ago"
/// or "3 hours ago" which are more intuitive for users than raw timestamps.
///
/// # Arguments
/// * `timestamp` - An RFC 3339 formatted timestamp string
///
/// # Returns
/// * `Some(String)` - Human-readable relative time description if parsing succeeds
/// * `None` - If the timestamp cannot be parsed
///
/// # Time Ranges
/// - More than 1 day: "X days ago"
/// - More than 1 hour: "X hours ago"
/// - More than 1 minute: "X minutes ago"
/// - Less than 1 minute: "just now"
///
/// # Examples
/// ```
/// let recent_time = current_timestamp();
/// println!("{}", time_since(&recent_time).unwrap()); // "just now" or "2 minutes ago"
///
/// let old_time = "2023-01-01T00:00:00Z";
/// println!("{}", time_since(old_time).unwrap()); // "340 days ago"
/// ```
pub fn time_since(timestamp: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| {
            // Calculate the duration between now and the provided timestamp
            let duration = Utc::now().signed_duration_since(dt.with_timezone(&Utc));

            // Select the most appropriate time unit based on the duration magnitude
            if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} minutes ago", duration.num_minutes())
            } else {
                // For durations less than a minute, use "just now"
                "just now".to_string()
            }
        })
        .ok() // Convert Result to Option, discarding any parsing errors
}
