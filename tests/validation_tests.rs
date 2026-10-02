//! Shared request validation (MAIR-426): free text keeps `<`, `>` and `&`.

use actix_web::{http::StatusCode, test as actix_test, web, App, HttpResponse, ResponseError};
use mairie360_api_lib::validation::{
    check_description, check_label, check_opaque, check_optional, Validate, ValidatedJson,
    ValidatedQuery, ValidationError,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Body {
    name: String,
}

impl Validate for Body {
    fn validate(&self) -> Result<(), ValidationError> {
        check_label("name", &self.name, 8)
    }
}

async fn echo_json(body: ValidatedJson<Body>) -> HttpResponse {
    HttpResponse::Ok().body(body.into_inner().name)
}

async fn echo_query(query: ValidatedQuery<Body>) -> HttpResponse {
    HttpResponse::Ok().body(query.into_inner().name)
}

async fn call(request: actix_test::TestRequest) -> (StatusCode, String) {
    let app = actix_test::init_service(
        App::new()
            .route("/json", web::post().to(echo_json))
            .route("/query", web::get().to(echo_query)),
    )
    .await;
    let response = actix_test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = actix_test::read_body(response).await;
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[actix_web::test]
async fn validated_json_passes_a_valid_body_through() {
    let request = actix_test::TestRequest::post()
        .uri("/json")
        .set_json(serde_json::json!({ "name": "a > b" }));
    assert_eq!(call(request).await, (StatusCode::OK, "a > b".to_owned()));
}

#[actix_web::test]
async fn validated_json_answers_400_naming_the_field() {
    let request = actix_test::TestRequest::post()
        .uri("/json")
        .set_json(serde_json::json!({ "name": "much too long" }));
    assert_eq!(
        call(request).await,
        (
            StatusCode::BAD_REQUEST,
            "Invalid `name`: must be at most 8 characters".to_owned()
        )
    );
}

#[actix_web::test]
async fn validated_json_answers_400_on_a_malformed_body() {
    let request = actix_test::TestRequest::post()
        .uri("/json")
        .insert_header(("content-type", "application/json"))
        .set_payload("{");
    assert_eq!(call(request).await.0, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn validated_query_passes_a_valid_query_through() {
    let request = actix_test::TestRequest::get().uri("/query?name=a%3Cb");
    assert_eq!(call(request).await, (StatusCode::OK, "a<b".to_owned()));
}

#[actix_web::test]
async fn validated_query_answers_400_on_invalid_or_missing_values() {
    let (status, body) = call(actix_test::TestRequest::get().uri("/query?name=%20")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, "Invalid `name`: must not be empty");
    assert_eq!(
        call(actix_test::TestRequest::get().uri("/query")).await.0,
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn free_text_keeps_angle_brackets_and_ampersands() {
    assert!(check_label("name", "Voirie <-> Urbanisme & Co", 64).is_ok());
    assert!(check_description("description", "budget > 10 000 € <3", 1000).is_ok());
    assert!(check_description("message", "<script>alert(1)</script>", 1000).is_ok());
}

#[test]
fn label_rejects_blank_long_and_control() {
    assert!(check_label("name", "Service urbanisme", 64).is_ok());
    assert!(check_label("name", "  ", 64).is_err());
    assert!(check_label("name", &"a".repeat(65), 64).is_err());
    assert!(check_label("name", "Service\0", 64).is_err());
}

#[test]
fn label_counts_characters_not_bytes() {
    assert!(check_label("name", &"é".repeat(64), 64).is_ok());
}

#[test]
fn description_allows_line_breaks_only() {
    assert!(check_description("description", "a\nb\r\n\tc", 1000).is_ok());
    assert!(check_description("description", "", 1000).is_ok());
    assert!(check_description("description", "a\0b", 1000).is_err());
    assert!(check_description("description", &"a".repeat(1001), 1000).is_err());
}

#[test]
fn opaque_rejects_nul() {
    assert!(check_opaque("token", "Zm9vYmFy", 512).is_ok());
    assert!(check_opaque("token", "abc\0", 512).is_err());
}

#[test]
fn optional_skips_absent_values() {
    assert!(check_optional(None, |v| check_label("name", v, 64)).is_ok());
    assert!(check_optional(Some(" "), |v| check_label("name", v, 64)).is_err());
}

#[test]
fn a_validation_error_is_a_400() {
    let error = ValidationError::new("title", "must not be empty");
    assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
    assert_eq!(error.to_string(), "Invalid `title`: must not be empty");
}
