use std::future::Future;

use resend_rs::{
    types::{CreateEmailBaseOptions, EmailId, EmailTemplate as ResendTemplate},
    Resend,
};
use thiserror::Error;

use super::{sender::no_reply_sender, template::EmailTemplate};
use crate::env_manager::get_env_var;

/// Resend API key (`re_…`). Required by [`ResendMailer::from_env`].
pub const RESEND_API_KEY: &str = "RESEND_API_KEY";
/// Domain the e-mails are sent from (`no-reply@<EMAIL_DOMAIN>`). Required by
/// [`ResendMailer::from_env`].
pub const EMAIL_DOMAIN: &str = "EMAIL_DOMAIN";
/// Optional display name of the sender (`Mairie 360` → `Mairie 360 <no-reply@…>`).
pub const EMAIL_SENDER_NAME: &str = "EMAIL_SENDER_NAME";

/// Failure to configure the mailer or to send an e-mail.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MailerError {
    /// Missing or invalid configuration (API key, domain, sender name).
    #[error("e-mail configuration error: {0}")]
    Config(String),
    /// Resend refused the e-mail or could not be reached.
    #[error("Resend error: {0}")]
    Resend(#[from] resend_rs::Error),
}

/// Sends [`EmailTemplate`]s. Implemented by [`ResendMailer`], and by
/// [`MockMailer`](super::mock_client::MockMailer) in tests.
pub trait TemplateMailer {
    /// Sends `template` to `to` and returns the id Resend gave the e-mail.
    fn send<T: EmailTemplate + Sync>(
        &self,
        to: &str,
        template: &T,
    ) -> impl Future<Output = Result<EmailId, MailerError>>;
}

/// [`TemplateMailer`] over the Resend HTTP API. Every e-mail leaves from
/// `no-reply@<domain>` (see [`no_reply_sender`]), with no `Reply-To`.
///
/// The API endpoint is `https://api.resend.com`, or the `RESEND_BASE_URL` environment variable
/// when set: the test stacks point it at a fake Resend server.
#[derive(Clone)]
pub struct ResendMailer {
    client: Resend,
    from: String,
}

impl ResendMailer {
    /// Builds a mailer sending from `no-reply@<domain>`.
    ///
    /// # Errors
    ///
    /// [`MailerError::Config`] when the API key is empty or the domain / display name is
    /// invalid (see [`no_reply_sender`]).
    pub fn new(
        api_key: &str,
        domain: &str,
        display_name: Option<&str>,
    ) -> Result<Self, MailerError> {
        if api_key.trim().is_empty() {
            return Err(MailerError::Config(format!("{RESEND_API_KEY} is empty")));
        }
        Ok(Self {
            client: Resend::new(api_key.trim()),
            from: no_reply_sender(domain, display_name)?,
        })
    }

    /// Builds the mailer from `RESEND_API_KEY`, `EMAIL_DOMAIN` and the optional
    /// `EMAIL_SENDER_NAME`.
    ///
    /// # Errors
    ///
    /// [`MailerError::Config`] when `RESEND_API_KEY` or `EMAIL_DOMAIN` is missing or invalid.
    pub fn from_env() -> Result<Self, MailerError> {
        let api_key = get_env_var(RESEND_API_KEY)
            .ok_or_else(|| MailerError::Config(format!("{RESEND_API_KEY} is not set")))?;
        let domain = get_env_var(EMAIL_DOMAIN)
            .ok_or_else(|| MailerError::Config(format!("{EMAIL_DOMAIN} is not set")))?;
        let display_name = get_env_var(EMAIL_SENDER_NAME);
        Self::new(&api_key, &domain, display_name.as_deref())
    }

    /// The sender every e-mail of this mailer uses.
    #[must_use]
    pub fn from(&self) -> &str {
        &self.from
    }
}

impl TemplateMailer for ResendMailer {
    async fn send<T: EmailTemplate + Sync>(
        &self,
        to: &str,
        template: &T,
    ) -> Result<EmailId, MailerError> {
        let resend_template =
            ResendTemplate::new(template.alias()).with_variables(template.variables());
        // Empty subject: the template's own subject is used.
        let options =
            CreateEmailBaseOptions::new(&self.from, [to], "").with_template(resend_template);
        let response = self.client.emails.send(options).await?;
        Ok(response.id)
    }
}
