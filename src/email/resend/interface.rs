// Deprecated since 3.1.0 along with `AppTemplate`, kept until the next major.
#![allow(deprecated)]

use std::future::Future;

use resend_rs::{
    types::{CreateEmailBaseOptions, CreateEmailResponse, EmailTemplate},
    Resend,
};

use crate::email::resend::templates::AppTemplate;

#[deprecated(since = "3.1.0", note = "use `email::TemplateMailer`")]
pub trait EmailService {
    fn send_template(
        &self,
        to: Vec<String>,
        template: AppTemplate,
    ) -> impl Future<Output = Result<CreateEmailResponse, resend_rs::Error>>;
}

#[deprecated(
    since = "3.1.0",
    note = "use `email::ResendMailer`, which always sends from `no-reply@<domain>`"
)]
#[derive(Clone)]
pub struct ResendClient {
    client: Resend,
    from: String,
}

#[allow(deprecated)]
impl ResendClient {
    // Le constructeur redevient synchrone puisqu'on ne fait plus d'appel API au démarrage
    #[must_use]
    pub fn new(api_key: &str, from: &str) -> Self {
        let client = Resend::new(api_key);
        Self {
            client,
            from: from.into(),
        }
    }
}

#[allow(deprecated)]
impl EmailService for ResendClient {
    async fn send_template(
        &self,
        to: Vec<String>,
        template: AppTemplate,
    ) -> Result<CreateEmailResponse, resend_rs::Error> {
        let alias = template.template_alias();
        let variables = template.into_variables();

        let email_options = CreateEmailBaseOptions::new(&self.from, to, "");
        let resend_template = EmailTemplate::new(alias).with_variables(variables);
        let email_options = email_options.with_template(resend_template);

        let response = self.client.emails.send(email_options).await?;
        Ok(response)
    }
}
