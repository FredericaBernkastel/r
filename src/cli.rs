use clap::Parser;

#[derive(Parser, Clone, Debug, Default)]
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

  /// Interval to fetch new posts. Default: 10s
  pub reddit_fetch_interval: Option<f64>,

  /// Interval to fetch new posts, if subreddit filter enabled. Default: 60s
  pub subreddit_fetch_interval: Option<f64>,

  /// Send a email at least once per interval. Default: 10 minutes
  pub email_send_interval: Option<f64>,

  /// Maximum number of submissions per letter. Default: 200
  pub email_max_submissions_per_letter: Option<usize>,

  /// Only send a letter if above or equal this threshold. Default: 1
  pub email_min_submissions_per_letter: Option<usize>
}