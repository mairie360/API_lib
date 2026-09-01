use std::future::Future;

use resend_rs::{
    types::{CreateEmailBaseOptions, CreateEmailResponse, EmailTemplate},
    Resend,
};

use crate::email::resend::templates::AppTemplate;

pub trait EmailService {
    fn send_template(
        &self,
        to: Vec<String>,
        template: AppTemplate,
    ) -> impl Future<Output = Result<CreateEmailResponse, resend_rs::Error>>;
}

#[derive(Clone)]
pub struct ResendClient {
    client: Resend,
    from: String,
}

impl ResendClient {
    // Le constructeur redevient synchrone puisqu'on ne fait plus d'appel API au démarrage
    pub fn new(api_key: &str, from: &str) -> Self {
        let client = Resend::new(api_key);
        Self {
            client,
            from: from.into(),
        }
    }
}

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
