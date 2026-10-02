//! Bounded list parameters (MAIR-425).

use actix_web::{test as actix_test, web, App, HttpResponse};
use mairie360_api_lib::database::db_interface::QueryParam;
use mairie360_api_lib::database::SqlId;
use mairie360_api_lib::pagination::{
    CursorPage, CursorParams, Page, PageParams, DEFAULT_PAGE_SIZE, MAX_OFFSET, MAX_PAGE_SIZE,
};

#[test]
fn page_params_are_defaulted_and_clamped() {
    let default = PageParams::default();
    assert_eq!((default.limit(), default.offset()), (DEFAULT_PAGE_SIZE, 0));

    let huge = PageParams {
        limit: Some(1_000_000),
        offset: Some(u32::MAX),
    };
    assert_eq!((huge.limit(), huge.offset()), (MAX_PAGE_SIZE, MAX_OFFSET));

    let zero = PageParams {
        limit: Some(0),
        offset: Some(0),
    };
    assert_eq!(zero.limit(), 1, "an empty page is never asked to Postgres");
}

#[test]
fn the_view_fetches_one_row_more_than_the_page() {
    let page = PageParams {
        limit: Some(10),
        offset: Some(20),
    };
    assert_eq!(page.sql_limit(), QueryParam::I64(11));
    assert_eq!(page.sql_offset(), QueryParam::I64(20));
}

#[test]
fn the_extra_row_becomes_has_more() {
    let params = PageParams {
        limit: Some(3),
        offset: Some(6),
    };
    let full = Page::from_overfetch(vec![1, 2, 3, 4], &params);
    assert_eq!(full.items, vec![1, 2, 3]);
    assert!(full.has_more);
    assert_eq!((full.limit, full.offset), (3, 6));

    let last = Page::from_overfetch(vec![1, 2], &params);
    assert_eq!(last.items, vec![1, 2]);
    assert!(!last.has_more);
}

#[test]
fn the_page_serializes_with_its_bounds() {
    let page = Page::from_overfetch(vec!["a"], &PageParams::default());
    assert_eq!(
        serde_json::to_value(&page).unwrap(),
        serde_json::json!({ "items": ["a"], "limit": 50, "offset": 0, "has_more": false })
    );
}

#[test]
fn the_cursor_page_points_to_the_last_row_served() {
    let params = CursorParams {
        before: None,
        limit: Some(2),
    };
    assert_eq!(params.sql_before(), QueryParam::OptionI32(None));
    assert_eq!(params.sql_limit(), QueryParam::I64(3));

    // Newest first: ids 30, 20, 10 fetched for a page of 2.
    let page = CursorPage::from_overfetch(vec![30, 20, 10], &params, |id| *id);
    assert_eq!(page.items, vec![30, 20]);
    assert_eq!(page.next_before, Some(20));

    let next = CursorParams {
        before: SqlId::new(20),
        limit: Some(2),
    };
    assert_eq!(next.sql_before(), QueryParam::OptionI32(Some(20)));
    let last = CursorPage::from_overfetch(vec![10], &next, |id| *id);
    assert_eq!(last.next_before, None, "no older page");
}

async fn list(page: web::Query<PageParams>, cursor: web::Query<CursorParams>) -> HttpResponse {
    HttpResponse::Ok().body(format!(
        "{} {} {:?}",
        page.limit(),
        page.offset(),
        cursor.before
    ))
}

#[actix_web::test]
async fn params_are_read_from_the_query_string() {
    let app = actix_test::init_service(App::new().route("/items", web::get().to(list))).await;

    let req = actix_test::TestRequest::get()
        .uri("/items?limit=500&offset=40&before=9")
        .to_request();
    let body = actix_test::call_and_read_body(&app, req).await;
    assert_eq!(body, "100 40 Some(SqlId(9))");

    let req = actix_test::TestRequest::get()
        .uri("/items?limit=-1")
        .to_request();
    let status = actix_test::call_service(&app, req).await.status();
    assert_eq!(status, actix_web::http::StatusCode::BAD_REQUEST);
}
