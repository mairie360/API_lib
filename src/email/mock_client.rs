// The deprecated `MockEmailClient` still implements the deprecated `EmailService`.
#![allow(deprecated)]

#[cfg(any(test, feature = "test-utils"))]
use crate::email::resend::{interface::EmailService, templates::AppTemplate};
#[cfg(any(test, feature = "test-utils"))]
use resend_rs::types::{CreateEmailResponse, EmailId};

#[cfg(any(test, feature = "test-utils"))]
#[deprecated(since = "3.1.0", note = "use `email::mock_client::MockMailer`")]
pub struct MockEmailClient {
    // Vous pouvez stocker les e-mails "envoyés" pour les inspecter dans vos tests
    pub sent_emails: std::sync::Mutex<Vec<(Vec<String>, AppTemplate)>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for MockEmailClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl MockEmailClient {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sent_emails: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl EmailService for MockEmailClient {
    fn send_template(
        &self,
        to: Vec<String>,
        template: AppTemplate,
    ) -> impl std::future::Future<Output = Result<CreateEmailResponse, resend_rs::Error>> {
        // On enregistre l'appel pour l'Assertion du test
        self.sent_emails.lock().unwrap().push((to, template));

        let email_id: EmailId = EmailId::new("fake-email-id-1234");

        // On simule une réponse réussie de Resend, disponible immédiatement
        std::future::ready(Ok(CreateEmailResponse { id: email_id }))
    }
}

/// An e-mail recorded by [`MockMailer`].
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentEmail {
    /// Recipient.
    pub to: String,
    /// Alias of the template.
    pub alias: String,
    /// Variables of the template.
    pub variables: std::collections::HashMap<String, serde_json::Value>,
}

/// [`TemplateMailer`](crate::email::TemplateMailer) that records every e-mail instead of
/// sending it, so the APIs' tests can assert on the template and its variables.
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Default)]
pub struct MockMailer {
    sent: std::sync::Mutex<Vec<SentEmail>>,
    fail: bool,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockMailer {
    /// A mailer that accepts every e-mail.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A mailer that refuses every e-mail with a [`MailerError::Config`](crate::email::MailerError),
    /// to test the paths where sending fails.
    #[must_use]
    pub const fn failing() -> Self {
        Self {
            sent: std::sync::Mutex::new(Vec::new()),
            fail: true,
        }
    }

    /// The e-mails sent so far, oldest first.
    ///
    /// # Panics
    ///
    /// Panics if a previous test thread panicked while holding the lock.
    #[must_use]
    pub fn sent(&self) -> Vec<SentEmail> {
        self.sent.lock().expect("MockMailer lock poisoned").clone()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl crate::email::TemplateMailer for MockMailer {
    fn send<T: crate::email::EmailTemplate + Sync>(
        &self,
        to: &str,
        template: &T,
    ) -> impl std::future::Future<Output = Result<EmailId, crate::email::MailerError>> {
        let result = if self.fail {
            Err(crate::email::MailerError::Config(
                "MockMailer::failing refuses every e-mail".to_string(),
            ))
        } else {
            self.sent
                .lock()
                .expect("MockMailer lock poisoned")
                .push(SentEmail {
                    to: to.to_string(),
                    alias: template.alias().to_string(),
                    variables: template.variables(),
                });
            Ok(EmailId::new("mock-email-id"))
        };
        std::future::ready(result)
    }
}
