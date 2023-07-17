use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
pub struct Settings {
  /// Filter submissions by a single subreddit
  #[arg(short = 's', long)]
  pub subreddit: Option<String>,

  /// Filter submissions by title using regex
  #[arg(short = 'r', long = "filter_regex")]
  pub submission_filter_regex: Option<String>,

  /// Email address to notify about new submissions
  #[arg(long)]
  pub notify_email: Option<String>,
}