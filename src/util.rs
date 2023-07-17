use {
  chrono::{DateTime, Utc, NaiveDateTime}
};

pub fn datetime_from_f64(t: f64) -> Option<DateTime<Utc>> {
  NaiveDateTime::from_timestamp_opt(t as i64, 0)
    .map(|d| DateTime::<Utc>::from_utc(d, Utc))
}