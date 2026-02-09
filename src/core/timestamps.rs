use chrono::{DateTime, Duration, Utc};

/// Converts a Chrono `Duration` object into a human-readable string representation.
pub fn format_duration(duration: &Duration) -> String {
    if duration.num_days() > 0 {
        format!("{} days", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes", duration.num_minutes())
    } else {
        "0 minutes".to_string()
    }
}

/// Returns the current timestamp in RFC 3339 format (ISO 8601).
pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

/// Determines if a given RFC 3339 timestamp is older than a specified duration.
pub fn is_timestamp_older_than(timestamp: &str, duration: &Duration) -> bool {
    if let Ok(parsed_time) = DateTime::parse_from_rfc3339(timestamp) {
        let time_utc = parsed_time.with_timezone(&Utc);
        let now = Utc::now();
        now - time_utc > *duration
    } else {
        true
    }
}

/// Converts an RFC 3339 timestamp into a human-readable relative time string.
pub fn time_since(timestamp: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| {
            let duration = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
            if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else if duration.num_minutes() > 0 {
                format!("{} minutes ago", duration.num_minutes())
            } else {
                "just now".to_string()
            }
        })
        .ok()
}
