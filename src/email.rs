use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize)]
struct SMTPSettings {
  smtp_host: String,
  smtp_user: String,
  smtp_password: String
}

pub struct Mailer {
  //smtp_settings: SMTPSettings,
  mailer: SmtpTransport
}

impl Mailer {
  pub fn new() -> Result<Self> {
    let settings: SMTPSettings = toml::from_str(
      &std::fs::read_to_string("smtp_config.toml").context("Unable to read ./smtp_config.toml")?
    )?;
    let creds = Credentials::new(settings.smtp_user.clone(), settings.smtp_password.clone());
    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&settings.smtp_host)?
      .credentials(creds)
      .build();
    Ok(Self { mailer })
  }

  pub fn send(&self, message: &Message) -> Result<()> {
    // Send the email
    self.mailer.send(message)?;
    Ok(())
  }
}
