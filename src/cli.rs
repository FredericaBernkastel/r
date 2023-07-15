use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = "Example: cargo +nightly run -- -s \"AskReddit\" -r \"what's\"")]
pub struct Settings {
  /// Subreddit to watch
  #[arg(short = 's', long)]
  pub subreddit: String,

  /// Filter submissions by title based on this regex
  #[arg(short = 'r', long = "filter_regex")]
  pub submission_filter_regex: Option<String>,

  /// Email address to notify about new submissions
  #[arg(long)]
  pub notify_email: Option<String>,
}