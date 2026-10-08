//! No value received or read in the errors and logs of the lib (MAIR-290): a log describes an
//! error by its type and context, never by the value. The database cases provoke real Postgres
//! errors whose message or `DETAIL` carries the marker values, then check the `Display`, the
//! `Debug` and the log line of the resulting error.

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

use actix_web::ResponseError;
use mairie360_api_lib::database::db_interface::{ApiRequestDto, Database, QueryParam};
use mairie360_api_lib::database::error::DbError;
use mairie360_api_lib::database::query_views::{
    DoesUserExistByEmailQueryView, GetUserIdByEmailQueryView, IsSessionTokenValidQueryView,
};
use mairie360_api_lib::error::{describe_json_error, log_error, ApiLibError};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serde::Deserialize;
use serial_test::serial;

const EMAIL: &str = "gdpr.markerqzkfjwxa@example.com";
const PHONE: &str = "6123498765";
const TOKEN: &str = "refresh-token-markerqzkfjwxa";

fn assert_clean(text: &str, what: &str) {
    for value in [EMAIL, "markerqzkfjwxa", PHONE, TOKEN, "203.0.113.7"] {
        assert!(!text.contains(value), "{what} leaks {value:?}: {text}");
    }
}

/// Everything `f` logs through `tracing`, at every level.
fn captured_logs(f: impl FnOnce()) -> String {
    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);
    impl Write for Buffer {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let bytes = buffer.0.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

#[derive(Debug, Clone, Deserialize)]
struct RawQuery {
    #[serde(skip)]
    sql: &'static str,
    params: Vec<QueryParam>,
}

impl ApiRequestDto for RawQuery {
    fn query_sql(&self) -> &'static str {
        self.sql
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }
}

const fn query(sql: &'static str, params: Vec<QueryParam>) -> RawQuery {
    RawQuery { sql, params }
}

fn text(value: &str) -> QueryParam {
    QueryParam::Text(value.to_string())
}

/// Display, Debug, the log of its HTTP answer and the log of `log_error`.
fn assert_error_clean(error: DbError) {
    assert_clean(&error.to_string(), "Display");
    assert_clean(&format!("{error:?}"), "Debug");
    let logs = captured_logs(|| {
        let _ = error.error_response();
    });
    assert_clean(&logs, "error_response log");
    let logs = captured_logs(|| {
        let _ = log_error("test", ApiLibError::Database(error));
    });
    assert_clean(&logs, "log_error");
}

const INSERT_USER: &str =
    "INSERT INTO users (first_name, last_name, email, password, phone_number) VALUES ($1, $2, $3, $4, $5)";

#[tokio::test]
#[serial]
async fn a_check_violation_names_the_constraint_not_the_failing_row() {
    let (_container, db_host) = get_shared_db().await;
    let db = Database::new(db_host.as_str()).await;
    let mut tx = db.begin().await.unwrap();
    // The DETAIL of a check violation is "Failing row contains (…)": names, e-mail, phone and
    // here the plaintext password that chk_users_password_hashed refuses.
    let error = tx
        .execute(&query(
            INSERT_USER,
            vec![
                text("Markerqzkfjwxa"),
                text("Tracer"),
                text(EMAIL),
                text(TOKEN),
                text(PHONE),
            ],
        ))
        .await
        .unwrap_err();
    let DbError::Sqlx(sqlx::Error::Database(db_error)) = &error else {
        panic!("expected a Postgres error, got {error:?}");
    };
    assert_eq!(db_error.code().as_deref(), Some("23514"));
    let described = error.to_string();
    assert!(described.contains("SQLSTATE 23514"), "{described}");
    assert!(
        described.contains("constraint chk_users_password_hashed"),
        "{described}"
    );
    assert!(described.contains("table users"), "{described}");
    assert_error_clean(error);
}

#[tokio::test]
#[serial]
async fn a_unique_violation_carries_the_constraint_name() {
    let (_container, db_host) = get_shared_db().await;
    let db = Database::new(db_host.as_str()).await;
    let mut tx = db.begin().await.unwrap();
    let insert = query(
        INSERT_USER,
        vec![
            text("Markerqzkfjwxa"),
            text("Tracer"),
            text(EMAIL),
            text(mairie360_api_lib::test_setup::queries_setup::seed_password_hash()),
            text(PHONE),
        ],
    );
    tx.execute(&insert).await.unwrap();
    let error = tx.execute(&insert).await.unwrap_err();
    // The DETAIL would be "Key (email)=(gdpr.marker…) already exists". The constraint is
    // uq_users_email_lower since Database v1.7.0, uq_users_email before.
    assert!(
        matches!(&error, DbError::UniqueViolation(constraint) if constraint.starts_with("uq_users_email")),
        "{error:?}"
    );
    assert_error_clean(error);
}

#[tokio::test]
#[serial]
async fn an_invalid_input_is_not_quoted() {
    let (_container, db_host) = get_shared_db().await;
    let db = Database::new(db_host.as_str()).await;
    // 22P02's message itself quotes the input: invalid input syntax for type integer: "…".
    let error = db
        .fetch_one::<i32, _>(&query("SELECT to_jsonb($1::int)", vec![text(EMAIL)]))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("SQLSTATE 22P02"), "{error}");
    assert_error_clean(error);
}

#[tokio::test]
#[serial]
async fn a_row_that_does_not_match_the_dto_is_not_quoted() {
    let (_container, db_host) = get_shared_db().await;
    let db = Database::new(db_host.as_str()).await;
    let error = db
        .fetch_one::<i32, _>(&query("SELECT to_jsonb($1::text)", vec![text(EMAIL)]))
        .await
        .unwrap_err();
    assert!(
        matches!(&error, DbError::MappingError(msg) if msg.starts_with("invalid type: string, expected i32")),
        "{error:?}"
    );
    assert_error_clean(error);
}

#[test]
fn json_errors_keep_the_type_and_drop_the_value() {
    #[derive(Debug, Deserialize)]
    enum Kind {
        #[allow(dead_code)]
        Direct,
    }

    let cases = [
        (
            serde_json::from_str::<i32>(&format!("\"{EMAIL}\"")).unwrap_err(),
            "invalid type: string, expected i32 at line 1 column 33",
        ),
        (
            serde_json::from_str::<bool>(PHONE).unwrap_err(),
            "invalid type: integer, expected a boolean at line 1 column 10",
        ),
        (
            serde_json::from_str::<u8>(PHONE).unwrap_err(),
            "invalid value: integer, expected u8 at line 1 column 10",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(describe_json_error(&error), expected);
        let lib_error = ApiLibError::from(error);
        assert_clean(&lib_error.to_string(), "Display");
        assert_clean(&format!("{lib_error:?}"), "Debug");
    }

    let error = serde_json::from_str::<Kind>(&format!("\"{EMAIL}\"")).unwrap_err();
    assert!(describe_json_error(&error).starts_with("unknown variant, expected `Direct`"));
}

#[test]
fn query_views_never_show_their_values() {
    let by_email = GetUserIdByEmailQueryView::new(EMAIL);
    assert_eq!(by_email.to_string(), "GetUserIdByEmailQueryView");
    assert_clean(&format!("{by_email:?}"), "Debug");

    let exists = DoesUserExistByEmailQueryView::new(EMAIL.to_string());
    assert_clean(&exists.to_string(), "Display");
    assert_clean(&format!("{exists:?}"), "Debug");

    let session = IsSessionTokenValidQueryView::new(
        42,
        TOKEN.to_string(),
        IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
    );
    assert_eq!(
        session.to_string(),
        "IsSessionTokenValidQueryView: user_id = 42"
    );
    let debug = format!("{session:?}");
    assert_clean(&debug, "Debug");
    assert!(debug.contains("I32(42)"), "{debug}");
}

#[test]
fn query_params_hide_text_and_addresses_only() {
    assert_eq!(format!("{:?}", text(EMAIL)), "Text(<31 chars>)");
    assert_eq!(
        format!(
            "{:?}",
            QueryParam::IpAddr(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)))
        ),
        "IpAddr(<hidden>)"
    );
    assert_eq!(format!("{:?}", QueryParam::I32(7)), "I32(7)");
    assert_eq!(
        format!("{:?}", QueryParam::OptionI32(None)),
        "OptionI32(None)"
    );
}
