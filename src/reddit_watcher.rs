use {
  futures::{
    stream::{self, StreamExt},
    Stream
  },
  tokio::time::Duration,
  chrono::{DateTime, Utc},
  anyhow::{Result, bail},
  serde::Deserialize,
  regex::Regex,
  std::sync::{Arc, RwLock},
  crate::{util, cli::Settings}
};

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct Submission {
  #[serde(rename = "name")]
  pub id: String,
  pub title: String,
  pub author: String,
  pub subreddit_name_prefixed: String,
  #[serde(skip)]
  pub created_utc: DateTime<Utc>,
  pub permalink: String
}

pub struct RedditWatcher {
  pub subreddit_filter: RwLock<Option<String>>,
  pub title_regex_filter: RwLock<Option<Regex>>,

  reddit_fetch_interval: RwLock<Duration>,
  subreddit_fetch_interval: RwLock<Duration>,

  http_client: reqwest::Client,
}

impl RedditWatcher {
  pub fn new(env_settings: Settings) -> Result<Self> {
    let timeout = Duration::from_secs(10);
    let title_regex_filter = match env_settings.submission_filter_regex {
      Some(regex) => Some(Regex::new(&regex)?),
      None => None
    };

    Ok(Self {
      http_client: reqwest::Client::builder()
        .user_agent("windows:reqwest:v0.11")
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()?,
      subreddit_filter: RwLock::new(env_settings.subreddit),
      title_regex_filter: RwLock::new(title_regex_filter),
      reddit_fetch_interval: RwLock::new(Duration::from_secs_f64(env_settings.reddit_fetch_interval.unwrap_or(10.0))),
      subreddit_fetch_interval: RwLock::new(Duration::from_secs_f64(env_settings.subreddit_fetch_interval.unwrap_or(60.0))),
    })
  }

  pub fn with_subredit_filter(&self, subreddit: Option<String>) {
    *self.subreddit_filter.write().unwrap() = subreddit;
  }

  pub fn with_title_filter(&self, regex: Option<Regex>) {
    *self.title_regex_filter.write().unwrap() = regex;
  }

  pub fn with_reddit_fetch_interval(&self, t: Duration) {
    *self.reddit_fetch_interval.write().unwrap() = t;
  }

  pub fn with_subreddit_fetch_interval(&self, t: Duration) {
    *self.subreddit_fetch_interval.write().unwrap() = t;
  }

  pub async fn stream_submissions(&self) -> impl Stream<Item = Submission> + '_ {
    let last_id = Arc::new(RwLock::new(self.get_last_id().await));

    stream::repeat(())
      .then(move |_| {
        let last_id = last_id.clone();
        async move {
          tokio::time::sleep(
            if self.subreddit_filter.read().unwrap().is_some() {
              *self.subreddit_fetch_interval.read().unwrap()
            } else {
              *self.reddit_fetch_interval.read().unwrap()
            }
          ).await;

          {
            let mut last_id = last_id.write().unwrap();
            let subreddit = self.subreddit_filter.read().unwrap();
            if last_id.0.is_some() && subreddit.is_none() {
              last_id.0 = None // subreddit filter was disabled
            } else if *subreddit != last_id.0 {
              *last_id = self.get_last_id().await; // subreddit filter was enabled or changed
            }
          }

          let submissions = self.get_new(
            last_id.read().unwrap().1.as_deref(),
            self.subreddit_filter.read().unwrap().as_deref(),
            100
          )
            .await
            .unwrap_or(vec![])
            .into_iter()
            .map(|mut s| {
              let created_utc = s["data"]["created_utc"].as_f64()
                .and_then(util::datetime_from_f64)
                .unwrap_or_default();
              let mut s: Submission = serde_json::from_value(s["data"].take()).unwrap();
              s.created_utc = created_utc;
              s
            })
            .collect::<Vec<_>>();
          if submissions.len() >= 100 {
            log::warn!("Update frequency is too low. Some posts may have been missed!")
          }
          submissions.get(0)
            .map(|s| {
              *last_id.write().unwrap() = (self.subreddit_filter.read().unwrap().clone(), Some(s.id.clone()))
            });

          let submissions = submissions.into_iter().rev()
            // filter by title
            .filter(|s| match self.title_regex_filter.read().unwrap().as_ref() {
              Some(regex) => regex.is_match(&s.title),
              None => true
            });

          stream::iter(submissions)
        }
      })
      .flatten()
  }

  async fn get_new(&self, last_id: Option<&str>, subreddit: Option<&str>, limit: u32) -> Result<Vec<serde_json::Value>> {
    if !(1..=100).contains(&limit) {
      bail!("Invalid submissions limit, mus be in range 1..=100");
    }
    log::debug!("last_id = {last_id:?}");

    let url = format!(
      "https://www.reddit.com/{subreddit}new.json?raw_json=1&limit={limit}{before}",
      subreddit = subreddit.map(|s| format!("r/{s}/")).unwrap_or_default(),
      before = last_id.map(|s| format!("&before={s}")).unwrap_or_default()
    );

    Ok(
      self.http_client
        .get(url)
        .send()
        .await
        .map_err(|e| { log::error!("Reddit connection failed: {e:?}"); e })?
        .json::<serde_json::Value>()
        .await?
        ["data"]
        ["children"]
        .as_array()
        .cloned()
        .unwrap_or(vec![])
    )
  }

  async fn get_last_id(&self) -> (Option<String>, Option<String>) {
    let last_id = self.get_new(None, self.subreddit_filter.read().unwrap().as_deref(), 2)
      .await.ok()
      .and_then(|x| x.get(0).cloned())
      .and_then(|mut x| serde_json::from_value::<Submission>(x["data"].take()).ok())
      .map(|x| x.id);

    (self.subreddit_filter.read().unwrap().clone(), last_id)
  }

}