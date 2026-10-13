use std::collections::HashMap;

/// A transactional e-mail stored as a template in the Resend dashboard.
///
/// The lib holds no business e-mail: each API implements this trait for the e-mails it sends
/// and hands them to a [`TemplateMailer`](super::mailer::TemplateMailer). The subject, the
/// HTML and the text come from the template, so an API never writes an e-mail body by hand.
pub trait EmailTemplate {
    /// Alias of the template in the Resend dashboard (e.g. `account-password-reset`). The
    /// template must be published: Resend refuses to send a draft.
    fn alias(&self) -> &'static str;

    /// Values of the template variables, keyed exactly as the template names them
    /// (`{{{USER_FIRST_NAME}}}` → `USER_FIRST_NAME`). A variable the template declares with a
    /// fallback may be left out.
    fn variables(&self) -> HashMap<String, serde_json::Value>;
}
