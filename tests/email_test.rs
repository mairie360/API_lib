#[cfg(test)]
mod tests {

    use mairie360_api_lib::email::{
        mock_client::MockEmailClient,
        resend::{interface::EmailService, templates::AppTemplate},
    };
    use resend_rs::types::EmailId;

    // #[tokio::test]
    // async fn test_user_registration_sends_email() {
    //     // 1. Initialiser le faux client
    //     let mock_client = MockEmailClient::new();

    //     // 2. Simuler l'action métier de votre API qui envoie un mail
    //     let template = AppTemplate::Welcome {
    //         user_name: "TestUser".to_string(),
    //         activation_link: "https://test.com".to_string(),
    //     };

    //     let result = mock_client
    //         .send_template(vec!["test@example.com".to_string()], template)
    //         .await;

    //     // 3. Vérifications (Assertions)
    //     assert!(result.is_ok());
    //     assert_eq!(result.unwrap().id, "fake-email-id-1234");

    //     let sent = mock_client.sent_emails.lock().unwrap();
    //     assert_eq!(sent.len(), 1);
    //     assert_eq!(sent[0].0, vec!["test@example.com".to_string()]);
    // }

    #[tokio::test]
    async fn test_first_connect_password_reset_template() {
        // 1. Instanciation du template avec des données de test
        let template = AppTemplate::FirstConnectPasswordReset {
            target_name: "Jean Dupont".to_string(),
            mairie_name: "Mairie de Nantes".to_string(),
            reset_link: "https://mondomaine.com/auth/reset?token=abc-123".to_string(),
            expires_time: 30,
        };

        let vars = template.clone().into_variables();

        // 2. Vérification de l'alias Resend
        assert_eq!(template.template_alias(), "first-connect");

        // 3. Vérification de la conversion et des clés de variables

        assert_eq!(
            vars.get("target_name").and_then(|v| v.as_str()),
            Some("Jean Dupont")
        );
        assert_eq!(
            vars.get("mairie_name").and_then(|v| v.as_str()),
            Some("Mairie de Nantes")
        );
        assert_eq!(
            vars.get("reset_link").and_then(|v| v.as_str()),
            Some("https://mondomaine.com/auth/reset?token=abc-123")
        );
        assert_eq!(
            vars.get("expires_in_minutes").and_then(|v| v.as_u64()),
            Some(30)
        );

        let mock_client = MockEmailClient::new();

        let result = mock_client
            .send_template(vec!["test@example.com".to_string()], template)
            .await;

        // 3. Vérifications (Assertions)
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, EmailId::new("fake-email-id-1234"));

        let sent = mock_client.sent_emails.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, vec!["test@example.com".to_string()]);
    }
}
