#[cfg(any(test, feature = "test-utils"))]
use crate::email::resend::{interface::EmailService, templates::AppTemplate};
#[cfg(any(test, feature = "test-utils"))]
use resend_rs::types::{CreateEmailResponse, EmailId};

#[cfg(any(test, feature = "test-utils"))]
pub struct MockEmailClient {
    // Vous pouvez stocker les e-mails "envoyés" pour les inspecter dans vos tests
    pub sent_emails: std::sync::Mutex<Vec<(Vec<String>, AppTemplate)>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockEmailClient {
    pub fn new() -> Self {
        Self {
            sent_emails: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl EmailService for MockEmailClient {
    async fn send_template(
        &self,
        to: Vec<String>,
        template: AppTemplate,
    ) -> Result<CreateEmailResponse, resend_rs::Error> {
        // On enregistre l'appel pour l'Assertion du test
        self.sent_emails.lock().unwrap().push((to, template));

        let email_id: EmailId = EmailId::new("fake-email-id-1234");

        // On simule une réponse réussie de Resend
        Ok(CreateEmailResponse { id: email_id })
    }
}
