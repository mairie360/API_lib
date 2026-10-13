use super::mailer::MailerError;

/// Local part of every address the APIs send from. Nobody reads that mailbox: an e-mail sent by
/// an API never invites a reply.
pub const NO_REPLY_LOCAL_PART: &str = "no-reply";

/// Builds the sender of the APIs' e-mails: `no-reply@<domain>`, preceded by `display_name`
/// when one is given (`Mairie 360 <no-reply@mairie360-eip.fr>`).
///
/// Only the domain is configurable, so an instance cannot send from another mailbox by mistake.
/// The domain must be verified in the Resend dashboard.
///
/// # Errors
///
/// [`MailerError::Config`] when `domain` is empty, contains `@`, whitespace, `<` or `>`, has no
/// dot, or when `display_name` contains a control character, `<` or `>`.
pub fn no_reply_sender(domain: &str, display_name: Option<&str>) -> Result<String, MailerError> {
    let domain = domain.trim();
    let invalid_domain = domain.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain
            .chars()
            .any(|c| c == '@' || c == '<' || c == '>' || c.is_whitespace() || c.is_control());
    if invalid_domain {
        return Err(MailerError::Config(format!(
            "invalid e-mail domain {domain:?}: expected a bare domain such as mairie360-eip.fr"
        )));
    }
    let address = format!("{NO_REPLY_LOCAL_PART}@{}", domain.to_ascii_lowercase());

    match display_name.map(str::trim).filter(|name| !name.is_empty()) {
        None => Ok(address),
        Some(name) if name.chars().any(|c| c == '<' || c == '>' || c.is_control()) => Err(
            MailerError::Config(format!("invalid sender display name {name:?}")),
        ),
        Some(name) => Ok(format!("{name} <{address}>")),
    }
}
