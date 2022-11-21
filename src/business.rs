use chrono::{Duration, TimeZone, Utc};

// this module contains some business logic

pub const DEFAULT_UP: i64 = 6;
pub const DEFAULT_DOWN: i64 = 2;
pub const GRAPH_MAX_SIZE: usize = 100;

pub fn is_assignable_karma_expired(timestamp: i64) -> bool {
    let now = Utc::now();
    let then = Utc.timestamp(timestamp, 0);

    let midnight = (then + Duration::days(1)).date().and_hms(0, 0, 0);
    now.gt(&midnight)
}
