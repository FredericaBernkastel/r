use std::sync::OnceLock;
use {
  lettre::{
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport
  },
  serde::Deserialize,
  anyhow::{Context, Result},
  maud::html,
  std::{
    collections::VecDeque,
    time::Duration,
    sync::{Arc, RwLock}
  },
  crate::{
    cli::Settings,
    reddit_watcher::Submission
  }
};

#[derive(Debug, Clone, Deserialize)]
struct SMTPSettings {
  smtp_host: String,
  smtp_user: String,
  smtp_password: String
}

pub struct Mailer {
  //smtp_settings: SMTPSettings,
  mailer: SmtpTransport,
  submission_queue: Arc<RwLock<VecDeque<Submission>>>,

  pub env_settings: Arc<RwLock<Settings>>,
}

impl Mailer {
  pub fn new(env_settings: Settings) -> Result<Self> {
    let settings: SMTPSettings = toml::from_str(
      &std::fs::read_to_string("smtp_config.toml").context("Unable to read ./smtp_config.toml")?
    )?;
    let creds = Credentials::new(settings.smtp_user.clone(), settings.smtp_password.clone());
    let mailer = SmtpTransport::relay(&settings.smtp_host)?
      .credentials(creds)
      .build();
    Ok(Self {
      mailer,
      submission_queue: Arc::new(RwLock::new(
        VecDeque::with_capacity(env_settings.email_max_submissions_per_letter.unwrap_or(200))
      )),
      env_settings: Arc::new(RwLock::new(env_settings))
    })
  }

  fn compose_message(submissions: impl Iterator<Item = Submission>, settings: &Settings) -> Message {
    let subject = format!(
      "{subreddit}: new posts{regex}",
      subreddit = settings.subreddit.as_deref().map(|r| format!("/r/{r}")).unwrap_or("reddit".to_string()),
      regex = settings.submission_filter_regex.as_deref().map(|r| format!(" containing \"{r}\"")).unwrap_or_default()
    );
    let content = html! {
      head {
        style type="text/css" {r"
          body { font-family: monospace; }
          p { margin: 0; }
          a { text-decoration: none; }
        "}
      }
      @for submission in submissions {
        p {
          "[" (submission.created_utc) "] "
          (submission.subreddit_name_prefixed) ": \""
          a href={"https://reddit.com" (submission.permalink)} { (submission.title) } "\" "
          "by " (submission.author);
        }
      }
    };
    Message::builder()
      .from(settings.notify_email.as_ref().unwrap().parse().unwrap())
      .to(settings.notify_email.as_ref().unwrap().parse().unwrap())
      .subject(subject)
      .header(lettre::message::header::ContentType::TEXT_HTML)
      .body(content.into_string()).unwrap()
  }

  pub fn start_thread(this: Arc<OnceLock<Self>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
      let mut last_sent = chrono::Utc::now();
      loop {
        std::thread::sleep(Duration::from_secs(1));
        let Some(this) = this.get() else { continue };
        //log::debug!("email queue size: {}", this.submission_queue.read().unwrap().len());

        let now = chrono::Utc::now();

        let env_settings = this.env_settings.read().unwrap();
        if env_settings.notify_email.is_none() {
          continue;
        }

        let mut submission_queue = this.submission_queue.write().unwrap();
        let over_quota = submission_queue.len() > env_settings.email_max_submissions_per_letter.unwrap_or(200);
        let under_quota = submission_queue.len() < env_settings.email_min_submissions_per_letter.unwrap_or(1);
        let over_interval = now - last_sent > chrono::Duration::from_std(
          Duration::from_secs_f64(env_settings.email_send_interval.unwrap_or(10.0) * 60.0)
        ).unwrap();
        if (over_quota || over_interval) && !under_quota {
          let to_send_count = 0..env_settings.email_max_submissions_per_letter.unwrap_or(200).min(submission_queue.len());
          let to_send = submission_queue.drain(to_send_count.clone()).collect::<Vec<_>>();
          last_sent = now;
          // drop the lock as soon as possible
          drop(submission_queue);
          drop(env_settings);
          let message = Self::compose_message(to_send.into_iter(), &this.env_settings.read().unwrap());
          // Send the email
          match this.mailer.send(&message) {
            Ok(_) => log::debug!("[{now}] {} items sent in email", to_send_count.len()),
            Err(e) => log::error!("[{now}] Could not send email: {e:?}"),
          };
        }
      }
    })
  }

  pub fn add_submission_to_queue(&self, submission: Submission) {
    self.submission_queue.write().unwrap().push_back(submission);
  }
}
