# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`mairie360_api_lib` is a Rust library crate shared across all backend APIs of **Mairie 360**, an Epitech student project. It centralizes cross-cutting concerns (DB access, caching, JWT auth, actix-web middlewares, email, env vars) so individual API services don't reimplement them. It is published to a private Cargo registry (`mairie360`) and consumed as a dependency by those services — it is not a standalone server.

## Commands

Custom cargo aliases are defined in `.cargo/config.toml`:

```bash
cargo lint_check     # cargo fmt --all -- --check
cargo lint_fix        # cargo fmt --all
cargo check_code       # cargo clippy --all-targets --all-features -- -D warnings
```

Standard commands:

```bash
cargo build
cargo test                              # runs the full test suite (see "Tests" below — needs Docker)
cargo test --test jwt_tests              # run one test file
cargo test test_get_jwt_secret           # run a single test by name
cargo test -- --test-threads=1           # tests that touch the shared DB use #[serial] but running single-threaded avoids flakines
cargo audit
cargo deny check advisories licenses
```

CI runs via a shared reusable workflow (`mairie360/CICD/.github/workflows/back-lib-cicd.yml`, pinned by tag in `.github/workflows/cicd.yml`) — it is not defined in this repo. `cargo audit` / `cargo deny` are run by that workflow; there is no `deny.toml` checked in here.

### Build prerequisites

`.cargo/config.toml` declares a private registry `mairie360` (index `github.com/mairie360/cargo-index`) and sets `net.git-fetch-with-cli = true`, so a fresh `cargo build` needs working `git` auth to GitHub (SSH key or credential helper). Pulling the `ghcr.io/mairie360/*` images used by the test suite likewise needs `ghcr.io` registry access.

## Tests

Integration tests live in `tests/*.rs` and spin up real Postgres/Redis via `testcontainers` (Docker is required). Most tests share one Postgres container and one seeded dataset via `test_setup::queries_setup::get_shared_db()`, which uses a `tokio::sync::OnceCell` so the container/migrations/seed run once per test binary. Migrations are applied by running the `ghcr.io/mairie360/liquibase-migrations` image against the `ghcr.io/mairie360/database` container (see `src/test_setup/db_setup.rs`) — pulling these images requires registry access.

Seeded fixture users (Alice/Bob/Admin/Group Owner) and their IDs are exposed as `OnceCell` statics in `test_setup::queries_setup` (`ALICE_ID`, `BOB_ID`, `ADMIN_ID`, `GROUP_OWNER_ID`) — tests read these after `get_shared_db()` has run.

Tests that mutate process-global env vars (`JWT_SECRET`, `JWT_TIMEOUT`, `DB_*`, `REDIS_URL`, ...) use a `once_cell::sync::Lazy` `INIT` block per test file, forced via a local `setup()` — follow this pattern for new tests in the same file rather than setting env vars ad hoc, since these tests run in the same process and can race.

The `test-utils` feature (enabled in `[dev-dependencies]` on the crate itself) gates test-only code that must also be usable by *downstream* consumers of this library, notably `email::mock_client::MockEmailClient` (an `EmailService` mock) and everything under `test_setup` — keep new test helpers behind `#[cfg(any(test, feature = "test-utils"))]` if other API repos should be able to use them.

## Architecture

The crate is organized as independent modules under `src/`, each declared in `lib.rs`. One function/type per file, re-exported from the module's `mod.rs` (mirror this when adding files — see `jwt_manager` and `env_manager`). Test-only code is gated per-item with `#[cfg(any(test, feature = "test-utils"))]` (e.g. `email::mock_client`), not at the module level.

- **`database`** — low-level Postgres access (`db_interface::Database`, built on `sqlx`). Callers implement the `ApiRequestDto` trait on their own DTOs (`query_sql()`, `query_params()`, optional `cache_key()`/`cache_ttl()`) instead of writing ad hoc queries; `Database` binds `QueryParam` enum values positionally and expects the SQL to return JSON-compatible rows (results are deserialized via `serde_json::from_value`, not `sqlx::FromRow`). `database::query_views` holds concrete `ApiRequestDto` implementations used internally by this crate (`IsAdminQueryView`, `HasAccessQueryView`, `DoesUserExistByIdQueryView`, `DoesUserExistByEmailQueryView`, `IsSessionTokenValidQueryView`) — they follow the `query_sql`/`query_params`/`Display` pattern and are the model to copy when adding a new internal query.
- **`redis`** — `redis_interface::Redis` wraps `deadpool-redis`. Has plain (`get`/`set`/`delete`/`expire`) and `secure_*` variants; the `secure_*` variants are idempotent no-ops when the key already exists/doesn't exist (e.g. `secure_set` skips if the key is already present, `secure_delete`/`secure_expire` skip if absent).
- **`smart_db`** — `SmartDatabase` composes `Database` + `Redis` into a cache-aside layer: on `fetch_one`/`fetch_all`/`fetch_scalar` it checks Redis first (via the DTO's `cache_key()`), falls back to Postgres on miss, then repopulates Redis (applying `cache_ttl()` if set). Redis failures are swallowed everywhere except the required Postgres read/write, which errors normally. `execute()` invalidates the DTO's `cache_key()` after a successful write. This is the primary interface API services use for DB access, not `Database` directly.
- **`jwt_manager`** — one function per file (`generate_jwt`, `decode_jwt`, `check_jwt_validity`, `get_user_id_from_jwt`, `get_role_from_jwt`, `get_timeout_from_jwt`, `get_jwt_from_request`, `get_jwt_secret`, `get_jwt_timeout`). Secret/timeout come from the `JWT_SECRET`/`JWT_TIMEOUT` env vars. `check_jwt_validity` does decode + expiry check + a live `DoesUserExistByIdQueryView` lookup through `SmartDatabase` — it's the one function that hits the DB, the others are pure decode/encode.
- **`security`** — actix-web middlewares built on `jwt_manager` + `database::query_views`, all expecting `web::Data<AppState>` in app data:
  - `JwtMiddleware` — validates the JWT on every request except `/`, `/swagger-ui*`, `/api-docs*`, and any path containing `/auth`; on success inserts `AuthenticatedUser` into request extensions.
  - `AdminMiddleware` — same JWT check, but only applies to paths matching `/api/v\d+/admin` (via a `lazy_static` regex) and additionally requires `IsAdminQueryView` to return true.
  - `access_guard_middleware` + `AccessCheckConfig` — a `Next`-based (not `Transform`-based, unlike the two above) middleware for fine-grained per-route access checks against `HasAccessQueryView` / the `check_access` Postgres function. `AccessCheckConfig` (resource name, action, optional URL id-param name) must be attached as route `app_data`. `1` = allow, `-1` = 404, anything else = 403.
  - `AuthenticatedUser` — an actix `FromRequest` extractor that reads the value the middlewares above put in request extensions; it errors if used on a route not wrapped by one of them.
- **`state::AppState`** — constructed once at service startup via `AppState::new(redis_url, pg_url)`, owns the `SmartDatabase` and is injected as `web::Data<AppState>`; all middlewares pull the `SmartDatabase` from it.
- **`email`** — `EmailService` trait (`send_template`) with a real `resend::interface::ResendClient` implementation (Resend API, template-based) and a `mock_client::MockEmailClient` for tests/downstream test-utils. Templates are the `resend::templates::AppTemplate` enum — add new transactional emails there (alias + variables), not as free-form strings.
- **`error::ApiLibError`** — the crate-wide error enum aggregating `DbError`, `RedisError`, `JWTCheckError`, Resend errors, and JSON errors; implements actix-web's `ResponseError` so it can be returned directly from handlers. Each submodule also has its own `ResponseError`-implementing error type (`database::error::DbError`, `redis::error::RedisError`, `jwt_manager::error::JWTCheckError`) with per-variant HTTP status mapping and `eprintln!`-based logging on the "real" failure branches (mirror this pattern — log genuine failures, stay silent on expected ones like `NotFound`/`ExpiredToken` — when adding new error variants).
- **`env_manager`** — `get_env_var` (returns `Option<String>`) vs `get_critical_env_var` (panics if unset); prefer `get_critical_env_var` for required startup config, matching existing usage.

Comments and error strings throughout the codebase are written in French — match this when touching existing files.
