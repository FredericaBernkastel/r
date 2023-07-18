use {
  anyhow::Result,
  futures::{StreamExt},
  reddit_watcher::RedditWatcher,
  clap::Parser,
  std::time::Duration,
  std::{
    sync::Arc
  }
};

mod cli;
mod reddit_watcher;
mod email;
mod util;

// fetch new posts every 10s
static REDDIT_FETCH_INTERVAL: Duration = Duration::from_secs(10);
// if watching subreddit, fetch new posts every 1 minute
static SUBREDDIT_FETCH_INTERVAL: Duration = Duration::from_secs(60);
// send a email at least once per 10 minutes
static EMAIL_SEND_INTERVAL: Duration = Duration::from_secs(10 * 60);
// at most 200 posts per letter
static EMAIL_MAX_SUBMISSIONS_PER_LETTER: usize = 200;
// do not send a letter, if less than 1 post is present in queue
static EMAIL_MIN_SUBMISSIONS_PER_LETTER: usize = 1;

#[tokio::main]
async fn main() -> Result<()> {
  // Configure log level with RUST_LOG environment variable
  // https://docs.rs/env_logger/0.10.0/env_logger/#enabling-logging
  env_logger::Builder::from_env(
    env_logger::Env::default()
      .default_filter_or("debug")
  ) .format_timestamp(None)
    .format_module_path(false)
    .format_target(false)
    .init();

  let settings = cli::Settings::parse();

  let filter_regex = match settings.submission_filter_regex.as_ref() {
    Some(regex_str) => Some(regex::Regex::new(regex_str)?),
    None => None
  };

  let (mailer, mailer_thr) = match settings.notify_email {
    Some(_) => {
      let mailer = Arc::new(email::Mailer::new()?);
      let thr = mailer.clone().start_thread(&settings);
      (Some(mailer), Some(thr))
    },
    None => (None, None)
  };

  RedditWatcher::new()?
    .with_subredit_filter(settings.subreddit.clone())
    .with_title_filter(filter_regex)
    .stream_submissions()
    .await
    .for_each({
      let mailer = mailer.as_deref();
      move |submission| async move {
        log::info!(
          "[{}] {} {}: \"{}\" by {}",
          submission.created_utc,
          submission.id,
          submission.subreddit_name_prefixed,
          submission.title,
          submission.author
        );

        if let Some(mailer) = mailer {
          mailer.add_submission_to_queue(submission);
        }
      }
    })
    .await;

  mailer_thr
    .map(|thr| thr.join().unwrap());
  Ok(())
}
