# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

`mairie360_api_lib` is a Rust library crate shared across all backend APIs of **Mairie 360**, an Epitech student project. It centralizes cross-cutting concerns (DB access, caching, JWT auth, actix-web middlewares, email, env vars) so individual API services don't reimplement them. It is published on crates.io and consumed as a dependency by those services — it is not a standalone server.

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

Integration tests live in `tests/*.rs` and spin up real Postgres/Redis via `testcontainers` (Docker is required). Most tests share one Postgres container and one seeded dataset via `test_setup::queries_setup::get_shared_db()`, which uses a `tokio::sync::OnceCell` so the container/migrations/seed run once per test binary. Migrations are applied by running the `ghcr.io/mairie360/liquibase-migrations` image against the `ghcr.io/mairie360/database` container (see `src/test_setup/db_setup.rs`) — pulling these images requires registry access. Postgres is published on a random host port (never the host network / `5432`, so an unrelated local Postgres cannot be picked up instead), and the shared container is removed by an `atexit` hook since the `static` holding it is never dropped.

Seeded fixture users (Alice/Bob/Admin/Group Owner) and their IDs are exposed as `OnceCell` statics in `test_setup::queries_setup` (`ALICE_ID`, `BOB_ID`, `ADMIN_ID`, `GROUP_OWNER_ID`) — tests read these after `get_shared_db()` has run. Their e-mails (`alice@example.com`, `bob@example.com` — archived, `admin@test.com`, `owner@test.com`) are what the Keycloak tests put in their tokens.

`test_setup::keycloak_setup::KeycloakMock` is a fake realm (JWKS endpoint on a random local port, two throwaway RSA keys in `src/test_setup/fixtures/`, `sign`/`access_token_claims` helpers) for testing Keycloak tokens without Docker or a Keycloak instance; `tests/keycloak_tests.rs` runs entirely on it. It is `pub` so the APIs' own tests can use it. Configuration tests use `temp_env` and live in that file only, because `AppState::new` reads the `KEYCLOAK_*` variables from the process environment.

Tests that mutate process-global env vars (`JWT_SECRET`, `JWT_TIMEOUT`, `DB_*`, `REDIS_URL`, ...) use a `once_cell::sync::Lazy` `INIT` block per test file, forced via a local `setup()` — follow this pattern for new tests in the same file rather than setting env vars ad hoc, since these tests run in the same process and can race.

The `test-utils` feature (enabled in `[dev-dependencies]` on the crate itself) gates `email::mock_client::MockEmailClient` (an `EmailService` mock) for *downstream* consumers of this library; `test_setup` (DB/Redis containers, seeded fixtures, Keycloak mock) is unconditionally `pub` so the API repos can use it without the feature.

## Architecture

The crate is organized as independent modules under `src/`, each declared in `lib.rs`. One function/type per file, re-exported from the module's `mod.rs` (mirror this when adding files — see `jwt_manager` and `env_manager`). Test-only code is gated per-item with `#[cfg(any(test, feature = "test-utils"))]` (e.g. `email::mock_client`), not at the module level.

- **`database`** — low-level Postgres access (`db_interface::Database`, built on `sqlx`). Callers implement the `ApiRequestDto` trait on their own DTOs (`query_sql()`, `query_params()`, optional `cache_key()`/`cache_ttl()`) instead of writing ad hoc queries; `Database` binds `QueryParam` enum values positionally and expects the SQL to return JSON-compatible rows (results are deserialized via `serde_json::from_value`, not `sqlx::FromRow`). `database::query_views` holds concrete `ApiRequestDto` implementations used internally by this crate (`IsAdminQueryView`, `HasAccessQueryView`, `DoesUserExistByIdQueryView`, `DoesUserExistByEmailQueryView`, `IsSessionTokenValidQueryView`) — they follow the `query_sql`/`query_params`/`Display` pattern and are the model to copy when adding a new internal query.
- **`redis`** — `redis_interface::Redis` wraps `deadpool-redis`. Has plain (`get`/`set`/`delete`/`expire`) and `secure_*` variants; the `secure_*` variants are idempotent no-ops when the key already exists/doesn't exist (e.g. `secure_set` skips if the key is already present, `secure_delete`/`secure_expire` skip if absent).
- **`smart_db`** — `SmartDatabase` composes `Database` + `Redis` into a cache-aside layer: on `fetch_one`/`fetch_all`/`fetch_scalar` it checks Redis first (via the DTO's `cache_key()`), falls back to Postgres on miss, then repopulates Redis (applying `cache_ttl()` if set). Redis failures are swallowed everywhere except the required Postgres read/write, which errors normally. `execute()` invalidates the DTO's `cache_key()` after a successful write. This is the primary interface API services use for DB access, not `Database` directly.
- **`jwt_manager`** — one function per file (`generate_jwt`, `decode_jwt`, `check_jwt_validity`, `get_user_id_from_jwt`, `get_role_from_jwt`, `get_timeout_from_jwt`, `get_jwt_from_request`, `get_jwt_secret`, `get_jwt_timeout`). Secret/timeout come from the `JWT_SECRET`/`JWT_TIMEOUT` env vars. `check_jwt_validity` does decode + expiry check + a live `DoesUserExistByIdQueryView` lookup through `SmartDatabase` — it's the one function that hits the DB, the others are pure decode/encode.
- **`keycloak`** — validation of the access tokens issued by the instance's Keycloak realm (SSO, MAIR-140). `KeycloakConfig::from_env` reads `KEYCLOAK_REALM_URL` + `KEYCLOAK_CLIENT_ID` (both required to enable it), `KEYCLOAK_CLIENT_SECRET`, `KEYCLOAK_ISSUER` (public `iss` when the API reaches Keycloak through an internal URL, defaults to the realm URL) and `KEYCLOAK_AUDIENCE` (comma-separated, defaults to the client id). `KeycloakTokenVerifier::verify` checks an asymmetric signature against the realm JWKS (cached, refetched once on an unknown `kid`), `iss`, `exp` (30 s leeway), the `typ` claim (`Bearer` only: ID/refresh tokens are refused), the audience (`aud` **or** `azp`, since Keycloak only lists a client in `aud` with an audience mapper) and a verified e-mail, and returns a `KeycloakIdentity` (`sub`, e-mail, realm roles). `jwt_manager::authenticate_token` is the entry point used by the middlewares: it routes on the token's `alg` header — HMAC = historical `JWT_SECRET` token (same checks as `check_jwt_validity`), anything else = Keycloak token, whose e-mail is then matched case-insensitively to an active account by `GetUserIdByEmailQueryView` (archived accounts → `UnknownUser`). Without a verifier (Keycloak not configured), a Keycloak-shaped token is simply `InvalidToken`, so the password login keeps working during the transition.
- **`security`** — actix-web middlewares built on `jwt_manager` + `database::query_views`, all expecting `web::Data<AppState>` in app data:
  - `JwtMiddleware` — authenticates the bearer token (`authenticate_token`: historical JWT or Keycloak access token) on every request except `/`, `/swagger-ui*`, `/api-docs*`, and any path containing `/auth`; on success inserts `AuthenticatedUser` into request extensions.
  - `AdminMiddleware` — same token check, but only applies to paths matching `/api/v\d+/admin` (via a `lazy_static` regex) and additionally requires `IsAdminQueryView` to return true.
  - `access_guard_middleware` + `AccessCheckConfig` — a `Next`-based (not `Transform`-based, unlike the two above) middleware for fine-grained per-route access checks against `HasAccessQueryView` / the `check_access` Postgres function. `AccessCheckConfig` (resource name, action, optional URL id-param name) must be attached as route `app_data`. `1` = allow, `-1` = 404, anything else = 403.
  - `AuthenticatedUser` — an actix `FromRequest` extractor that reads the value the middlewares above put in request extensions; it errors if used on a route not wrapped by one of them.
- **`state::AppState`** — constructed once at service startup via `AppState::new(redis_url, pg_url)`, owns the `SmartDatabase` and the optional `KeycloakTokenVerifier` (built from `KeycloakConfig::from_env()`; `AppState::with_keycloak` takes an explicit config instead, e.g. in tests) and is injected as `web::Data<AppState>`; all middlewares pull both from it.
- **`email`** — `EmailService` trait (`send_template`) with a real `resend::interface::ResendClient` implementation (Resend API, template-based) and a `mock_client::MockEmailClient` for tests/downstream test-utils. Templates are the `resend::templates::AppTemplate` enum — add new transactional emails there (alias + variables), not as free-form strings.
- **`error::ApiLibError`** — the crate-wide error enum aggregating `DbError`, `RedisError`, `JWTCheckError`, `KeycloakError`, `PasswordError`, Resend errors, and JSON errors; implements actix-web's `ResponseError` so it can be returned directly from handlers. Each submodule also has its own `ResponseError`-implementing error type (`database::error::DbError`, `redis::error::RedisError`, `jwt_manager::error::JWTCheckError`) with per-variant HTTP status mapping and `eprintln!`-based logging on the "real" failure branches (mirror this pattern — log genuine failures, stay silent on expected ones like `NotFound`/`ExpiredToken` — when adding new error variants).
- **`env_manager`** — `get_env_var` (returns `Option<String>`) vs `get_critical_env_var` (panics if unset); prefer `get_critical_env_var` for required startup config, matching existing usage.

Much of the existing code (comments, error strings) is still in French; new or rewritten code is written in English (see `../CLAUDE.md`), and the historical `JWTCheckError` messages are kept as-is because Core_API quotes them in its OpenAPI examples.
