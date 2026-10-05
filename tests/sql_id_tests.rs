//! `SqlId` and `try_id_to_sql` (MAIR-422): ids Postgres cannot hold are refused, never wrapped
//! to another row.

use actix_web::{http::StatusCode, test as actix_test, web, App, HttpResponse, ResponseError};
use mairie360_api_lib::database::db_interface::QueryParam;
use mairie360_api_lib::database::{try_id_to_sql, InvalidId, SqlId};
use serde::Deserialize;

const I32_MAX: u64 = 2_147_483_647;

#[test]
fn accepts_one_to_i32_max_only() {
    assert_eq!(SqlId::try_from(1_u64).map(SqlId::get), Ok(1));
    assert_eq!(
        SqlId::try_from(I32_MAX).map(SqlId::get),
        Ok(i32::MAX),
        "i32::MAX is the last valid id"
    );
    for id in [0, I32_MAX + 1, (1_u64 << 32) + 1, u64::MAX] {
        assert_eq!(SqlId::try_from(id), Err(InvalidId), "{id}");
    }
    assert_eq!(SqlId::try_from(-1_i64), Err(InvalidId));
    assert_eq!(SqlId::try_from(0_i32), Err(InvalidId));
}

#[test]
fn parses_plain_decimal_digits_only() {
    assert_eq!("42".parse::<SqlId>().map(SqlId::get), Ok(42));
    for text in [
        "",
        "0",
        "-1",
        "+1",
        " 1",
        "1 ",
        "1.0",
        "0x10",
        "abc",
        "2147483648",
        "4294967297",
    ] {
        assert_eq!(text.parse::<SqlId>(), Err(InvalidId), "{text:?}");
    }
}

#[test]
fn converts_without_cast() {
    let id = SqlId::try_from(7_u64).unwrap();
    assert_eq!(QueryParam::from(id), QueryParam::I32(7));
    assert_eq!(i32::from(id), 7);
    assert_eq!(u64::from(id), 7);
    assert_eq!(id.to_string(), "7");
    assert_eq!(serde_json::to_string(&id).unwrap(), "7");
}

#[test]
fn try_id_to_sql_refuses_instead_of_saturating() {
    assert_eq!(try_id_to_sql(5), Ok(5));
    assert_eq!(try_id_to_sql(I32_MAX), Ok(i32::MAX));
    assert_eq!(try_id_to_sql((1_u64 << 32) + 1), Err(InvalidId));
    assert_eq!(try_id_to_sql(0), Err(InvalidId));
    assert_eq!(InvalidId.status_code(), StatusCode::BAD_REQUEST);
}

#[test]
fn deserializes_from_json_with_the_same_bounds() {
    #[derive(Deserialize)]
    struct Body {
        id: SqlId,
    }
    let body: Body = serde_json::from_str(r#"{"id": 12}"#).unwrap();
    assert_eq!(body.id.get(), 12);
    for json in [
        r#"{"id": 0}"#,
        r#"{"id": -3}"#,
        r#"{"id": 4294967297}"#,
        r#"{"id": 1.5}"#,
        r#"{"id": "12"}"#,
    ] {
        assert!(serde_json::from_str::<Body>(json).is_err(), "{json}");
    }
}

async fn echo(path: web::Path<(SqlId, SqlId)>) -> HttpResponse {
    let (project, task) = path.into_inner();
    HttpResponse::Ok().body(format!("{project}/{task}"))
}

#[derive(Deserialize)]
struct Filter {
    owner: SqlId,
}

async fn filter(query: web::Query<Filter>) -> HttpResponse {
    HttpResponse::Ok().body(query.owner.to_string())
}

#[actix_web::test]
async fn path_ids_out_of_range_are_not_found_and_query_ids_bad_requests() {
    let app = actix_test::init_service(
        App::new()
            .route("/projects/{project}/tasks/{task}", web::get().to(echo))
            .route("/tasks", web::get().to(filter)),
    )
    .await;

    let req = actix_test::TestRequest::get()
        .uri("/projects/3/tasks/2147483647")
        .to_request();
    assert_eq!(
        actix_test::call_and_read_body(&app, req).await,
        "3/2147483647"
    );

    for uri in [
        "/projects/4294967297/tasks/1",
        "/projects/1/tasks/0",
        "/projects/-1/tasks/1",
        "/projects/1/tasks/abc",
    ] {
        let req = actix_test::TestRequest::get().uri(uri).to_request();
        let status = actix_test::call_service(&app, req).await.status();
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
    }

    let req = actix_test::TestRequest::get()
        .uri("/tasks?owner=9")
        .to_request();
    assert_eq!(actix_test::call_and_read_body(&app, req).await, "9");
    let req = actix_test::TestRequest::get()
        .uri("/tasks?owner=4294967297")
        .to_request();
    assert_eq!(
        actix_test::call_service(&app, req).await.status(),
        StatusCode::BAD_REQUEST
    );
}
