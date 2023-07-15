use {
  futures::{Stream, StreamExt},
  roux::{Subreddit, submission::SubmissionData},
  tokio::time::Duration,
  tokio_retry::strategy::{jitter, ExponentialBackoff},
  chrono::{DateTime, Utc, NaiveDateTime}
};

pub struct SubredditWatcher {
  subreddit: Subreddit,
}

impl SubredditWatcher {
  pub fn new(subreddit: impl AsRef<str>) -> Self {
    Self { subreddit: Subreddit::new(subreddit.as_ref()) }
  }

  pub fn stream_submissions(&self) -> impl Stream<Item = (SubmissionData, DateTime<Utc>)> {
    let retry_strategy = ExponentialBackoff::from_millis(5)
      .factor(100)
      .map(jitter) // add jitter to delays
      .take(3); // limit to 3 retries

    // Abort fetching new items after 10s
    let timeout = Duration::from_secs(10);
    let now = Utc::now();

    let (submissions_stream, _) = roux_stream::stream_submissions(
      &self.subreddit,
      Duration::from_secs(60),
      retry_strategy.clone(),
      Some(timeout.clone()),
    );

   submissions_stream
    .filter_map(|s| async move { s.ok() })
    .filter_map(move |submission| async move {
      match NaiveDateTime::from_timestamp_opt(submission.created_utc as i64, 0)
        .map(|d| DateTime::<Utc>::from_utc(d, Utc)) {
        Some(created_utc) if created_utc > now => Some((submission, created_utc)),
        _ => None
      }
    })
  }
}