pub mod error;
pub mod mailer;
#[cfg(any(test, feature = "test-utils"))]
pub mod mock_client;
pub mod resend;
pub mod sender;
pub mod template;

pub use mailer::{MailerError, ResendMailer, TemplateMailer};
pub use sender::no_reply_sender;
pub use template::EmailTemplate;
