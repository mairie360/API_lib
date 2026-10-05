//! The refusals the audit asks every API to test (MAIR-419), written with the
//! `test_setup::http_setup` helpers the APIs use for their own endpoint tests.

use actix_web::{http::StatusCode, middleware::from_fn, test, web, App, HttpResponse};
use mairie360_api_lib::security::{
    access_guard_middleware, AccessCheckConfig, AdminMiddleware, AdminUser, AuthenticatedUser,
    JwtMiddleware,
};
use mairie360_api_lib::test_setup::http_setup::{
    authorization, authorization_for, status_of, test_app_state, SeededUser,
};

async fn whoami(user: AuthenticatedUser) -> HttpResponse {
    HttpResponse::Ok().body(user.id.to_string())
}

async fn admin_only(admin: AdminUser) -> HttpResponse {
    HttpResponse::Ok().body(admin.id.to_string())
}

async fn granted() -> HttpResponse {
    HttpResponse::Ok().body("granted")
}

/// Same wiring as the APIs: `JwtMiddleware` on `/api`, `AdminMiddleware` on the versioned scope,
/// an ACL-checked route on a user resource.
macro_rules! api {
    () => {
        test::init_service(
            App::new().app_data(test_app_state().await).service(
                web::scope("/api")
                    .wrap(JwtMiddleware)
                    .route("/v1/me", web::get().to(whoami))
                    .service(
                        web::scope("/v1/admin")
                            .wrap(AdminMiddleware)
                            .route("/users", web::get().to(admin_only)),
                    )
                    .service(
                        web::resource("/v1/users/{user_id}/data")
                            .app_data(AccessCheckConfig {
                                resource_name: "users",
                                action: "read",
                                id_param_pattern: Some("user_id"),
                            })
                            .wrap(from_fn(access_guard_middleware))
                            .route(web::get().to(granted)),
                    ),
            ),
        )
        .await
    };
}

#[actix_web::test]
async fn seeded_users_resolve_to_their_ids() {
    let app = api!();
    for user in [SeededUser::Alice, SeededUser::Admin, SeededUser::GroupOwner] {
        let id = user.id().await;
        let req = test::TestRequest::get()
            .uri("/api/v1/me")
            .insert_header(authorization(id))
            .to_request();
        let body = test::call_and_read_body(&app, req).await;
        assert_eq!(body, id.to_string(), "{user:?}");
    }
}

#[actix_web::test]
async fn a_request_without_token_is_refused() {
    let app = api!();
    let req = test::TestRequest::get().uri("/api/v1/me").to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn a_non_admin_is_refused_on_admin_routes_encoded_or_not() {
    let app = api!();
    for uri in ["/api/v1/admin/users", "/api/v1/%61dmin/users"] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(authorization_for(SeededUser::GroupOwner).await)
            .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::FORBIDDEN, "{uri}");

        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(authorization_for(SeededUser::Admin).await)
            .to_request();
        assert_eq!(status_of(&app, req).await, StatusCode::OK, "{uri}");
    }
}

#[actix_web::test]
async fn an_archived_account_is_refused() {
    let app = api!();
    let req = test::TestRequest::get()
        .uri("/api/v1/me")
        .insert_header(authorization_for(SeededUser::Bob).await)
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn another_user_cannot_reach_someone_elses_resource() {
    let app = api!();
    let alice = SeededUser::Alice.id().await;
    let uri = format!("/api/v1/users/{alice}/data");

    let req = test::TestRequest::get()
        .uri(&uri)
        .insert_header(authorization(alice))
        .to_request();
    assert_eq!(status_of(&app, req).await, StatusCode::OK, "owner");

    let req = test::TestRequest::get()
        .uri(&uri)
        .insert_header(authorization_for(SeededUser::GroupOwner).await)
        .to_request();
    assert_eq!(
        status_of(&app, req).await,
        StatusCode::FORBIDDEN,
        "another user"
    );
}
