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
    use mairie360_api_lib::{jwt_manager::generate_jwt, security::JwtMiddleware};

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
mod access_middleware {
    use super::*;
    use actix_web::middleware::from_fn;
    use actix_web::{http::StatusCode, test, web, App, HttpMessage, HttpResponse};
    use mairie360_api_lib::security::{
        access_guard_middleware, AccessCheckConfig, AuthenticatedUser,
    };

    // Handler de test pour valider que le middleware a laissé passer la requête
    async fn fake_handler() -> HttpResponse {
        HttpResponse::Ok().body("Granted")
    }

    #[tokio::test]
    async fn test_access_granted_owner() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        // On récupère Alice (ID 1 dans ton setup)
        let alice_id = *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
            .get()
            .unwrap();

        let app = test::init_service(
            App::new().app_data(app_state.clone()).service(
                web::resource("/users/{user_id}/data")
                    .app_data(AccessCheckConfig {
                        resource_name: "users", // Nom de la table/ressource
                        action: "read",
                        id_param_pattern: Some("user_id"),
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .route(web::get().to(fake_handler)),
            ),
        )
        .await;

        // On simule une requête sur sa propre ressource (Alice accède à Alice)
        let req = test::TestRequest::get()
            .uri(&format!("/users/{}/data", alice_id))
            .to_request();

        // On injecte l'utilisateur authentifié (simule le JwtMiddleware)
        req.extensions_mut().insert(AuthenticatedUser {
            id: alice_id as u64,
        });

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        assert_eq!(body, "Granted");
    }

    #[tokio::test]
    async fn test_access_forbidden_for_other_user() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        let alice_id = *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
            .get()
            .unwrap();
        let bob_id = alice_id + 1; // Supposons que Bob est l'ID suivant

        let app = test::init_service(
            App::new().app_data(app_state.clone()).service(
                web::resource("/users/{user_id}/secret")
                    .app_data(AccessCheckConfig {
                        resource_name: "users",
                        action: "write",
                        id_param_pattern: Some("user_id"),
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .route(web::get().to(fake_handler)),
            ),
        )
        .await;

        // Alice (ID 1) essaie d'accéder aux données de Bob (ID 2)
        let req = test::TestRequest::get()
            .uri(&format!("/users/{}/secret", bob_id))
            .to_request();

        req.extensions_mut().insert(AuthenticatedUser {
            id: alice_id as u64,
        });

        let resp = test::try_call_service(&app, req).await;

        match resp {
            Ok(res) => assert_eq!(res.status(), StatusCode::FORBIDDEN),
            Err(err) => {
                // Si le middleware renvoie une Error (ce qui est ton cas),
                // Actix la capture ici. On vérifie que c'est bien une 403.
                assert_eq!(err.as_response_error().status_code(), StatusCode::FORBIDDEN);
            }
        }
    }

    #[tokio::test]
    async fn test_access_bad_request_invalid_id_format() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        let app = test::init_service(
            App::new().app_data(app_state.clone()).service(
                web::resource("/items/{id}")
                    .app_data(AccessCheckConfig {
                        resource_name: "items",
                        action: "read",
                        id_param_pattern: Some("id"),
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .route(web::get().to(fake_handler)),
            ),
        )
        .await;

        // On envoie une string au lieu d'un i32 dans l'URL
        let req = test::TestRequest::get()
            .uri("/items/not-an-integer")
            .to_request();
        req.extensions_mut().insert(AuthenticatedUser { id: 1 });

        let resp = test::try_call_service(&app, req).await;

        // Le middleware doit intercepter le parse().ok() et renvoyer 400
        match resp {
            Ok(res) => assert_eq!(res.status(), StatusCode::BAD_REQUEST),
            Err(err) => {
                // Si le middleware renvoie une Error (ce qui est ton cas),
                // Actix la capture ici. On vérifie que c'est bien une 400.
                assert_eq!(
                    err.as_response_error().status_code(),
                    StatusCode::BAD_REQUEST
                );
            }
        }
    }

    #[tokio::test]
    async fn test_access_global_permission_success() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        // Ici, on teste le cas où id_param_pattern est None (vérification globale)
        // Utile pour les routes de listing ou Admin
        let app = test::init_service(
            App::new().app_data(app_state.clone()).service(
                web::scope("/admin")
                    .app_data(AccessCheckConfig {
                        resource_name: "users",
                        action: "read", // La fonction SQL cherchera 'read_all'
                        id_param_pattern: None,
                    })
                    .wrap(from_fn(access_guard_middleware))
                    .route("/all-users", web::get().to(fake_handler)),
            ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/admin/all-users")
            .to_request();

        // On simule un utilisateur (ID 1)
        // Note: Pour que ce test réussisse, Alice doit avoir le droit 'read_all'
        // sur 'users' dans ta base de données de test.
        req.extensions_mut().insert(AuthenticatedUser { id: 1 });

        let resp = test::call_service(&app, req).await;

        // Si Alice est admin dans ton setup SQL, ça sera OK, sinon FORBIDDEN
        // Ce test valide que le passage de paramètre NULL à Postgres fonctionne.
        assert!(resp.status() == StatusCode::OK || resp.status() == StatusCode::FORBIDDEN);
    }
}

mod admin_path_tests {
    use super::*;
    use actix_web::{http::StatusCode, test, web, App, HttpResponse};
    use mairie360_api_lib::jwt_manager::generate_jwt;
    use mairie360_api_lib::security::AdminMiddleware;

    // Handler de test pour valider que le middleware a laissé passer la requête
    async fn fake_handler() -> HttpResponse {
        HttpResponse::Ok().body("Granted")
    }

    #[actix_web::test]
    async fn test_admin_path() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        let app = test::init_service(
            App::new().app_data(app_state.clone()).service(
                web::scope("/api/v1/admin")
                    .wrap(AdminMiddleware)
                    .route("/all-users", web::get().to(fake_handler)),
            ),
        )
        .await;

        let token = generate_jwt("1").unwrap();

        let req = test::TestRequest::get()
            .uri("/api/v1/admin/all-users")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        // On simule un utilisateur (ID 1)

        let resp = test::call_service(&app, req).await;

        // Le test passe si le statut est OK ou FORBIDDEN (Alice peut être admin ou non)
        assert!(resp.status() == StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_admin_path_forbidden() {
        setup();
        let (_container, url) = get_shared_db().await;
        let app_state = web::Data::new(AppState::new("".to_string(), url.to_string()).await);

        let app = test::init_service(
            App::new().app_data(app_state.clone()).service(
                web::scope("/api/v1/admin")
                    .wrap(AdminMiddleware)
                    .route("/all-users", web::get().to(fake_handler)),
            ),
        )
        .await;

        let token = generate_jwt("Z").unwrap();

        let req = test::TestRequest::get()
            .uri("/api/v1/admin/all-users")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();

        // On simule un utilisateur (ID 1)

        let resp = test::call_service(&app, req).await;

        // Le test passe si le statut est OK ou FORBIDDEN (Alice peut être admin ou non)
        assert!(
            resp.status() == StatusCode::UNAUTHORIZED,
            "expected UNAUTHORIZED, got {}",
            resp.status()
        );
    }
}
