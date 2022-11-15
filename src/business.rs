use chrono::{DateTime, Duration, NaiveDateTime, Utc};

// this module contains some business logic

pub const DEFAULT_UP: i64 = 6;
pub const DEFAULT_DOWN: i64 = 2;

pub fn is_assignable_karma_expired(timestamp: i64) -> bool {
    let now = Utc::now();
    let then = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc);

    let midnight = (then + Duration::days(1)).date().and_hms(0, 0, 0);
    now.gt(&midnight)
}
