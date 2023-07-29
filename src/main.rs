#![allow(dead_code)]

use std::sync::OnceLock;
use {
  anyhow::Result,
  futures::{StreamExt},
  reddit_watcher::RedditWatcher,
  clap::Parser,
  std::{
    sync::Arc
  }
};

mod cli;
mod reddit_watcher;
mod email;
mod util;

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

  let mailer = Arc::new(OnceLock::new());

  let mailer_thr = match settings.notify_email {
    Some(_) => {
      mailer.set(email::Mailer::new(settings.clone())?).ok().unwrap();
      Some(email::Mailer::start_thread(mailer.clone()))
    },
    None => None
  };

  let reddit_watcher = RedditWatcher::new(settings.clone())?;
  reddit_watcher.with_subredit_filter(settings.subreddit.clone());
  reddit_watcher.with_title_filter(filter_regex);
  reddit_watcher
    .stream_submissions()
    .await
    .for_each({
      let mailer = mailer.get();
      move |submission| async move {
        log::info!(
          "[{}] {} {}: \"{}\" by {}",
          submission.created_utc,
          submission.id,
          submission.subreddit_name_prefixed,
          submission.title,
          submission.author
        );

        mailer.map(|m| {
          m.add_submission_to_queue(submission);
        });
      }
    })
    .await;

  mailer_thr
    .map(|thr| thr.join().unwrap());
  Ok(())
}
