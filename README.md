# mairie360-api-lib

**mairie360-api-lib** est une librairie Rust faisant partie du projet **Mairie 360**, un projet étudiant de l'école Epitech.

## À propos de Mairie 360

Mairie 360 a pour objectif de proposer un **outil performant de gestion de mairie**, centralisant les besoins administratifs et fonctionnels des municipalités. Le projet est entièrement développé par des étudiants dans le cadre de l'école Epitech.

## Objectif de cette librairie

Ce dépôt contient les **fonctionnalités communes** à toutes les API du projet. Il est destiné à :

- Centraliser le code partagé entre les différentes API.
- Faciliter la maintenance et l'évolution des fonctionnalités communes.
- Offrir une base solide pour développer de nouvelles API intégrées au projet Mairie 360.

## Technologie

- Cette librairie, comme l'ensemble des API du projet, est codée en **Rust**.
- Elle respecte les bonnes pratiques Rust pour la sécurité et la performance.

## Licence et usage

- La librairie est **publique**.
- Elle n’a **aucune restriction d’usage** à ce jour.
- Vous êtes libres de l’utiliser, la modifier et la redistribuer.

## Installation

Ajoutez cette dépendance dans votre `Cargo.toml` :

```toml
[dependencies]
mairie360-api-lib = "0.1.0"

## Request logging without personal data (`request_log`, MAIR-290)

A log describes a request by its type and context, never by the value it received.

- APIs that log with actix's `Logger`: `App::new().wrap(mairie360_api_lib::request_log::request_logger())`
  instead of `Logger::default()` (no query string, no `Referer`).
- APIs that log with `tracing-actix-web` (feature `tracing-actix`):

```toml
mairie360_api_lib = { version = "3.1", features = ["tracing-actix"] }
tracing-actix-web = { version = "0.7.25", default-features = false }
```

```rust
use mairie360_api_lib::request_log::{hide_query, restore_query, RedactedRootSpanBuilder};

App::new()
    .wrap(middleware::from_fn(restore_query))
    .wrap(TracingLogger::<RedactedRootSpanBuilder>::new())
    .wrap(middleware::from_fn(hide_query))
```

## Usage telemetry without identifiers (`usage`, feature `usage`, MAIR-501)

What is used, how much and by how many agents, never who does what. Enable the feature and wire
the ledger, the middleware and the endpoint the instance's OpenTelemetry Collector scrapes:

```rust
use mairie360_api_lib::usage::{usage_metrics, usage_middleware, UsageLedger, USAGE_METRICS_PATH};

let ledger = web::Data::new(UsageLedger::new("core-api"));
App::new()
    .app_data(ledger.clone())
    .wrap(actix_web::middleware::from_fn(usage_middleware))
    .route(USAGE_METRICS_PATH, web::get().to(usage_metrics));
```

- Counts per service, route template (never the path with its values), method, status and period
  (one hour): actions, distinct users, summed latency.
- Distinct users come from a hash of the user id with a salt drawn for each period, kept in
  memory only and dropped with the hashes when the period closes: no id, hash or salt is ever
  logged, stored or exported.
- Only closed periods are served, and a count under the threshold `k` (5 by default) is not: the
  small operations are summed into one `other` entry, dropped too when it is under `k`.
- `ALLOWED_SPAN_ATTRIBUTES` is the list of span attributes a trace may carry; the collector
  applies the same list (Devops/Deploiment).
