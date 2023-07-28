#![allow(dead_code)]

mod reddit_watcher;
mod email;
mod cli;
mod util;

pub use {
  reddit_watcher::{RedditWatcher, Submission},
  email::Mailer,
  cli::Settings
};