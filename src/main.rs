use {
  anyhow::Result,
  futures::{StreamExt},
  subreddit_watcher::SubredditWatcher,
  clap::Parser,
  maud::html
};

mod cli;
mod subreddit_watcher;
mod email;

#[tokio::main]
async fn main() -> Result<()> {
  let settings = cli::Settings::parse();
  let settings = &settings;

  let filter_regex = match settings.submission_filter_regex.as_ref() {
    Some(regex_str) => Some(regex::Regex::new(&regex_str)?),
    None => None
  };
  let filter_regex = filter_regex.as_ref();

  let mailer = match settings.notify_email {
    Some(_) => Some(email::Mailer::new()?),
    None => None
  };
  let mailer = mailer.as_ref();

  SubredditWatcher::new(&settings.subreddit)
    .stream_submissions()
    // filter based on submission title
    .filter_map(|(submission, created_utc)| async move {
      match filter_regex {
        Some(filter_regex) => filter_regex
          .is_match(&submission.title)
          .then(||(submission, created_utc)),
        None => Some((submission, created_utc))
      }
    })
    .for_each(|(submission, created_utc)| async move {
      println!(
        "[{}] r/{} by {}: {}",
        created_utc, submission.subreddit, submission.author, submission.title
      );

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
    .await;
  Ok(())
}
