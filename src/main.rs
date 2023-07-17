use {
  anyhow::Result,
  futures::{StreamExt},
  reddit_watcher::RedditWatcher,
  clap::Parser,
  maud::html
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
  let settings = &settings;

  let filter_regex = match settings.submission_filter_regex.as_ref() {
    Some(regex_str) => Some(regex::Regex::new(regex_str)?),
    None => None
  };

  let mailer = match settings.notify_email {
    Some(_) => Some(email::Mailer::new()?),
    None => None
  };
  let mailer = mailer.as_ref();

  RedditWatcher::new()?
    .with_subredit_filter(settings.subreddit.clone())
    .with_title_filter(filter_regex)
    .stream_submissions()
    .await
    .for_each(|submission| async move {
      log::info!(
        "[{}] {} {}: \"{}\" by {}",
        submission.created_utc,
        submission.id,
        submission.subreddit_name_prefixed,
        submission.title,
        submission.author
      );
    })
    .await;
  /*
      if let Some(mailer) = mailer {
        let subject = format!("r/{}: {}", settings.subreddit, submission.title);
        let content = html! {
          head {
            style type="text/css" {
              "body { font-family: monospace; }"
            }
          }
          p {
            "title: " (submission.title) br;
            "link: https://reddit.com" (submission.permalink) br;
            "created: " (created_utc.to_string()) br;
            "by: " (submission.author)
          }
        };
        let message = lettre::Message::builder()
          .from(settings.notify_email.as_ref().unwrap().parse().unwrap())
          .to(settings.notify_email.as_ref().unwrap().parse().unwrap())
          .subject(subject)
          .header(lettre::message::header::ContentType::TEXT_HTML)
          .body(content.into_string()).unwrap();
        // Send the email
        match mailer.send(&message) {
          Ok(_) => println!("Email sent successfully!"),
          Err(e) => eprintln!("Could not send email: {e:?}"),
        };
      }
    })
    .await;*/
  Ok(())
}
