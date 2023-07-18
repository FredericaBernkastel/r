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
  crate::util
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

#[derive(Default)]
pub struct RedditWatcher {
  subreddit_filter: Option<String>,
  title_regex_filter: Option<Regex>,
  http_client: reqwest::Client
}

impl RedditWatcher {
  pub fn new() -> Result<Self> {
    let timeout = Duration::from_secs(10);
    Ok(Self {
      http_client: reqwest::Client::builder()
        .user_agent("windows:reqwest:v0.11")
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()?,
      ..Default::default()
    })
  }

  pub fn with_subredit_filter(mut self, subreddit: Option<String>) -> Self {
    self.subreddit_filter = subreddit;
    self
  }

  pub fn with_title_filter(mut self, regex: Option<Regex>) -> Self {
    self.title_regex_filter = regex;
    self
  }

  pub async fn stream_submissions(&self) -> impl Stream<Item = Submission> + '_ {
    let last_id = self.get_new(None, self.subreddit_filter.as_deref(), 2)
      .await.ok()
      .and_then(|x| x.get(0).cloned())
      .and_then(|mut x| serde_json::from_value::<Submission>(x["data"].take()).ok())
      .map(|x| x.id);
    let last_id = Arc::new(RwLock::new(last_id));

    stream::repeat(())
      .then(move |_| {
        let last_id = last_id.clone();
        async move {
          tokio::time::sleep(
            if self.subreddit_filter.is_some() {
              crate::SUBREDDIT_FETCH_INTERVAL
            } else {
              crate::REDDIT_FETCH_INTERVAL
            }
          ).await;
          let submissions = self.get_new(
            last_id.read().unwrap().as_deref(),
            self.subreddit_filter.as_deref(),
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
              *last_id.write().unwrap() = Some(s.id.clone())
            });

          let submissions = submissions.into_iter().rev()
            // filter by title
            .filter(|s| match &self.title_regex_filter {
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
}