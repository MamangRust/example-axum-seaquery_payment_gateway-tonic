use chrono::{DateTime, Utc};

pub fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    if value.is_empty() {
        None
    } else {
        DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    }
}
