use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

use actix_web::{web, App, HttpResponse, HttpServer};
use mairie360_api_lib::email::{
    mock_client::{MockMailer, SentEmail},
    no_reply_sender, EmailTemplate, MailerError, ResendMailer, TemplateMailer,
};
use serde_json::{json, Value};

/// Requests received by the fake Resend server.
static RECEIVED: LazyLock<Arc<Mutex<Vec<Value>>>> = LazyLock::new(Arc::default);

/// Fake Resend API on a random local port, reached through `RESEND_BASE_URL` (read by
/// `resend_rs` when the client is built). Started once for the whole binary.
static FAKE_RESEND: LazyLock<String> = LazyLock::new(|| {
    let received = Arc::clone(&RECEIVED);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        actix_web::rt::System::new().block_on(async move {
            let server = HttpServer::new(move || {
                let received = Arc::clone(&received);
                App::new().route(
                    "/emails",
                    web::post().to(move |body: web::Json<Value>| {
                        let received = Arc::clone(&received);
                        async move {
                            received.lock().unwrap().push(body.into_inner());
                            HttpResponse::Ok().json(json!({ "id": "fake-resend-id" }))
                        }
                    }),
                )
            })
            .workers(1)
            .bind(("127.0.0.1", 0))
            .expect("Failed to bind the fake Resend server");
            let port = server.addrs()[0].port();
            tx.send(format!("http://127.0.0.1:{port}")).unwrap();
            server.run().await.unwrap();
        });
    });
    let url = rx.recv().expect("fake Resend server did not start");
    std::env::set_var("RESEND_BASE_URL", &url);
    url
});

struct PasswordReset {
    first_name: String,
    reset_url: String,
}

impl EmailTemplate for PasswordReset {
    fn alias(&self) -> &'static str {
        "account-password-reset"
    }

    fn variables(&self) -> HashMap<String, Value> {
        HashMap::from([
            ("USER_FIRST_NAME".to_string(), json!(self.first_name)),
            ("RESET_URL".to_string(), json!(self.reset_url)),
            ("EXPIRE_MINUTES".to_string(), json!(15)),
        ])
    }
}

fn reset_template() -> PasswordReset {
    PasswordReset {
        first_name: "Jeanne".to_string(),
        reset_url: "https://login.mairie360.fr/reset-password?token=abc".to_string(),
    }
}

#[test]
fn no_reply_sender_builds_the_address_from_the_domain() {
    assert_eq!(
        no_reply_sender("mairie360-eip.fr", None).unwrap(),
        "no-reply@mairie360-eip.fr"
    );
    assert_eq!(
        no_reply_sender(" Mairie360-EIP.fr ", Some("Mairie 360")).unwrap(),
        "Mairie 360 <no-reply@mairie360-eip.fr>"
    );
    assert_eq!(
        no_reply_sender("mairie360-eip.fr", Some("  ")).unwrap(),
        "no-reply@mairie360-eip.fr"
    );
}

#[test]
fn no_reply_sender_refuses_anything_but_a_bare_domain() {
    for domain in [
        "",
        "localhost",
        "admin@mairie360-eip.fr",
        "mairie360-eip.fr>",
        "mairie 360.fr",
        ".mairie360-eip.fr",
        "mairie360-eip.fr.",
    ] {
        assert!(
            matches!(no_reply_sender(domain, None), Err(MailerError::Config(_))),
            "{domain:?} should be refused"
        );
    }
    assert!(matches!(
        no_reply_sender("mairie360-eip.fr", Some("Evil <admin@x.fr>")),
        Err(MailerError::Config(_))
    ));
}

#[test]
fn resend_mailer_needs_a_key_and_a_valid_domain() {
    assert!(matches!(
        ResendMailer::new(" ", "mairie360-eip.fr", None),
        Err(MailerError::Config(_))
    ));
    assert!(matches!(
        ResendMailer::new("re_test", "admin@mairie360-eip.fr", None),
        Err(MailerError::Config(_))
    ));
    let mailer = ResendMailer::new("re_test", "mairie360-eip.fr", Some("Mairie 360")).unwrap();
    assert_eq!(mailer.from(), "Mairie 360 <no-reply@mairie360-eip.fr>");
}

#[test]
fn resend_mailer_from_env_requires_the_key_and_the_domain() {
    temp_env::with_vars(
        [
            ("RESEND_API_KEY", None::<&str>),
            ("EMAIL_DOMAIN", Some("mairie360-eip.fr")),
        ],
        || {
            assert!(matches!(
                ResendMailer::from_env(),
                Err(MailerError::Config(_))
            ));
        },
    );
    temp_env::with_vars(
        [
            ("RESEND_API_KEY", Some("re_test")),
            ("EMAIL_DOMAIN", None::<&str>),
        ],
        || {
            assert!(matches!(
                ResendMailer::from_env(),
                Err(MailerError::Config(_))
            ));
        },
    );
    temp_env::with_vars(
        [
            ("RESEND_API_KEY", Some("re_test")),
            ("EMAIL_DOMAIN", Some("mairie360-eip.fr")),
            ("EMAIL_SENDER_NAME", Some("Mairie 360")),
        ],
        || {
            assert_eq!(
                ResendMailer::from_env().unwrap().from(),
                "Mairie 360 <no-reply@mairie360-eip.fr>"
            );
        },
    );
}

#[tokio::test]
async fn resend_mailer_sends_the_template_from_no_reply() {
    LazyLock::force(&FAKE_RESEND);
    let mailer = ResendMailer::new("re_test", "mairie360-eip.fr", Some("Mairie 360")).unwrap();

    let id = mailer
        .send("jeanne@example.com", &reset_template())
        .await
        .unwrap();

    assert_eq!(id.to_string(), "fake-resend-id");
    let received = RECEIVED.lock().unwrap().clone();
    let body = received.last().expect("no request reached the fake Resend");
    assert_eq!(body["from"], "Mairie 360 <no-reply@mairie360-eip.fr>");
    assert_eq!(body["to"], json!(["jeanne@example.com"]));
    assert_eq!(body["template"]["id"], "account-password-reset");
    assert_eq!(body["template"]["variables"]["USER_FIRST_NAME"], "Jeanne");
    assert_eq!(body["template"]["variables"]["EXPIRE_MINUTES"], 15);
    assert!(body.get("reply_to").is_none(), "no Reply-To: {body}");
    assert!(body.get("html").is_none() && body.get("text").is_none());
}

#[tokio::test]
async fn mock_mailer_records_the_template() {
    let mailer = MockMailer::new();

    mailer
        .send("jeanne@example.com", &reset_template())
        .await
        .unwrap();

    assert_eq!(
        mailer.sent(),
        vec![SentEmail {
            to: "jeanne@example.com".to_string(),
            alias: "account-password-reset".to_string(),
            variables: reset_template().variables(),
        }]
    );
}

#[tokio::test]
async fn failing_mock_mailer_refuses_and_records_nothing() {
    let mailer = MockMailer::failing();

    let result = mailer.send("jeanne@example.com", &reset_template()).await;

    assert!(matches!(result, Err(MailerError::Config(_))));
    assert_eq!(mailer.sent(), Vec::new());
}

/// The deprecated closed enum keeps working until the next major.
#[allow(deprecated)]
#[tokio::test]
async fn deprecated_app_template_still_works() {
    use mairie360_api_lib::email::{
        mock_client::MockEmailClient,
        resend::{interface::EmailService, templates::AppTemplate},
    };

    let template = AppTemplate::FirstConnectPasswordReset {
        target_name: "Jean Dupont".to_string(),
        mairie_name: "Mairie de Nantes".to_string(),
        reset_link: "https://mondomaine.com/auth/reset?token=abc-123".to_string(),
        expires_time: 30,
    };
    assert_eq!(template.template_alias(), "first-connect");
    assert_eq!(
        template.clone().into_variables().get("target_name"),
        Some(&json!("Jean Dupont"))
    );

    let mock_client = MockEmailClient::new();
    let result = mock_client
        .send_template(vec!["test@example.com".to_string()], template)
        .await;
    assert!(result.is_ok());
    assert_eq!(mock_client.sent_emails.lock().unwrap().len(), 1);
}
