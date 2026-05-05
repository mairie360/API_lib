use mairie360_api_lib::security::JwtMiddleware;

use mairie360_api_lib::{pool::AppState, test_setup::queries_setup::get_shared_db};

use std::env;

static INIT: once_cell::sync::Lazy<()> = once_cell::sync::Lazy::new(|| {
    // This code runs ONCE before any test
    env::set_var("JWT_SECRET", "b\"secret\"");
    env::set_var("JWT_TIMEOUT", "3600");
    println!("Global setup done");
});

fn setup() {
    // Force INIT to run
    once_cell::sync::Lazy::force(&INIT);
}

#[cfg(test)]
mod auth_middleware {

    use super::*;
    use actix_web::{http::StatusCode, test, web, App, HttpResponse};
    use mairie360_api_lib::jwt_manager::generate_jwt;

    // Route de test simple protégée par le middleware
    async fn index() -> HttpResponse {
        HttpResponse::Ok().body("Welcome!")
    }

    #[tokio::test]
    async fn test_middleware_bypass_public_routes() {
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .wrap(JwtMiddleware)
                .route("/auth/login", web::get().to(index)),
        )
        .await;

        // Test sur une route ignorée par le middleware (/auth/...)
        let req = test::TestRequest::get().uri("/auth/login").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_no_token_returns_401() {
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .wrap(JwtMiddleware)
                .route("/protected", web::get().to(index)),
        )
        .await;

        // Requête sans header Authorization
        let req = test::TestRequest::get().uri("/protected").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_middleware_valid_token_success() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        // On récupère l'ID d'Alice depuis ton setup global
        let alice_id = *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
            .get()
            .unwrap();

        let token = generate_jwt(&alice_id.to_string()).unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .wrap(JwtMiddleware)
                .route("/protected", web::get().to(index)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = &test::call_service(&app, req).await;

        let response = resp.response();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "response: {:?}",
            response.body()
        );
    }

    #[tokio::test]
    async fn test_middleware_expired_token_returns_401() {
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        setup();
        env::set_var("JWT_TIMEOUT", "0");
        let token = generate_jwt("2").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));

        let app = test::init_service(
            App::new()
                .app_data(app_state.clone())
                .wrap(JwtMiddleware)
                .route("/protected", web::get().to(index)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // Optionnel: vérifier le corps du message
        let body = test::read_body(resp).await;
        assert!(body.starts_with(b"Unauthorized: JWT token is expired."));
    }

    #[tokio::test]
    async fn test_middleware_missing_db_pool_returns_500() {
        // App sans .app_data(app_state) pour tester la branche d'erreur "DB Pool missing"
        let app = test::init_service(
            App::new()
                .wrap(JwtMiddleware)
                .route("/protected", web::get().to(index)),
        )
        .await;

        let req = test::TestRequest::get().uri("/protected").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[cfg(test)]
mod access_middleware {}
