use mairie360_api_lib::database::db_interface::{ApiRequestDto, Database, QueryParam};
use mairie360_api_lib::error::ApiLibError;
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::{
    queries_setup::get_shared_db, redis_setup::start_redis_container,
};
use serde::Deserialize;
use serial_test::serial;

/// Une vue de test standard (sans TTL)
#[derive(Debug, Clone, Deserialize)]
struct CachedUserExistsView {
    user_id: i32,
    params: Vec<QueryParam>,
}

impl CachedUserExistsView {
    fn new(user_id: i32) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id)],
        }
    }
}

impl ApiRequestDto for CachedUserExistsView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM public.users WHERE id = $1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }

    fn cache_key(&self) -> Option<String> {
        Some(format!("test:user:exists:{}", self.user_id))
    }
}

/// Une vue de test dédiée pour valider l'expiration par TTL court (1 seconde)
#[derive(Debug, Clone, Deserialize)]
struct CachedUserExistsWithShortTtlView {
    user_id: i32,
    params: Vec<QueryParam>,
}

impl CachedUserExistsWithShortTtlView {
    fn new(user_id: i32) -> Self {
        Self {
            user_id,
            params: vec![QueryParam::I32(user_id)],
        }
    }
}

impl ApiRequestDto for CachedUserExistsWithShortTtlView {
    fn query_sql(&self) -> &'static str {
        "SELECT EXISTS(SELECT 1 FROM public.users WHERE id = $1)"
    }

    fn query_params(&self) -> &[QueryParam] {
        &self.params
    }

    fn cache_key(&self) -> Option<String> {
        Some(format!("test:user:short_ttl:{}", self.user_id))
    }

    // TTL très court de 1 seconde pour le test d'expiration
    fn cache_ttl(&self) -> Option<u64> {
        Some(1)
    }
}

#[cfg(test)]
mod smart_database_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_smart_db_cache_aside_flow() {
        let (_db_container, db_host) = get_shared_db().await;
        let (_redis_node, redis_config) = start_redis_container().await;

        let db = Database::new(db_host.as_str()).await;
        let redis = Redis::new(&redis_config.url);
        let smart_db = SmartDatabase::new(db, redis.clone());

        let view = CachedUserExistsView::new(1);
        let cache_key = "test:user:exists:1";

        let _ = redis.delete(cache_key).await;
        assert!(!redis.key_exist(cache_key).await.unwrap());

        let first_result: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert!(first_result);

        assert!(
            redis.key_exist(cache_key).await.unwrap(),
            "La clé doit être présente dans Redis après un fetch réussi"
        );

        let second_result: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert_eq!(first_result, second_result);
    }

    #[tokio::test]
    #[serial]
    async fn test_smart_db_cache_ttl_expiration() {
        let (_db_container, db_host) = get_shared_db().await;
        let (_redis_node, redis_config) = start_redis_container().await;

        let db = Database::new(db_host.as_str()).await;
        let redis = Redis::new(&redis_config.url);
        let smart_db = SmartDatabase::new(db, redis.clone());

        let view = CachedUserExistsWithShortTtlView::new(1);
        let cache_key = "test:user:short_ttl:1";

        // Nettoyage préalable
        let _ = redis.delete(cache_key).await;

        // 1. Premier appel : Met en cache avec un TTL de 1 seconde
        let result: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert!(result);

        // 2. Vérification immédiate : la clé doit exister
        assert!(
            redis.key_exist(cache_key).await.unwrap(),
            "La clé doit être en cache juste après le fetch"
        );

        // 3. On attend un peu plus d'une seconde (1200ms) pour laisser le TTL expirer
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        // 4. Vérification après expiration : la clé doit avoir disparu de Redis
        let exists_after = redis.key_exist(cache_key).await.unwrap();
        assert!(
            !exists_after,
            "La clé aurait dû être supprimée automatiquement par Redis après expiration du TTL"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_smart_db_cache_invalidation_on_execute() {
        #[derive(Debug, Clone, Deserialize)]
        struct DummyUpdateView {
            params: Vec<QueryParam>,
        }
        impl ApiRequestDto for DummyUpdateView {
            fn query_sql(&self) -> &'static str {
                "SELECT 1"
            }
            fn query_params(&self) -> &[QueryParam] {
                &self.params
            }
            fn cache_key(&self) -> Option<String> {
                Some("test:user:exists:1".to_string())
            }
        }

        let (_db_container, db_host) = get_shared_db().await;
        let (_redis_node, redis_config) = start_redis_container().await;

        let db = Database::new(db_host.as_str()).await;
        let redis = Redis::new(&redis_config.url);
        let smart_db = SmartDatabase::new(db, redis.clone());

        let view = CachedUserExistsView::new(1);
        let cache_key = "test:user:exists:1";

        let _: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert!(redis.key_exist(cache_key).await.unwrap());

        let update_view = DummyUpdateView { params: vec![] };

        let execute_res = smart_db.execute(update_view).await;
        assert!(execute_res.is_ok());

        let exists_after = redis.key_exist(cache_key).await.unwrap();
        assert!(
            !exists_after,
            "Le cache doit être invalidé (supprimé) après une exécution d'écriture"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_key_actually_expires_after_ttl() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        // 1. On set une clé et on lui applique un TTL de 1 seconde
        redis_interface
            .set("ttl_expiry_key", "temp_value")
            .await
            .unwrap();
        redis_interface.expire("ttl_expiry_key", 1).await.unwrap();

        // 2. Vérification immédiate (la clé est présente)
        assert!(
            redis_interface.key_exist("ttl_expiry_key").await.unwrap(),
            "La clé doit exister immédiatement après le set et l'expire"
        );

        // 3. On attend un peu plus d'une seconde
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

        // 4. Vérification après expiration (la clé doit avoir disparue)
        let exists = redis_interface.key_exist("ttl_expiry_key").await.unwrap();
        assert!(
            !exists,
            "La clé Redis aurait dû être supprimée automatiquement après l'expiration du TTL"
        );
    }
}

/// `SmartDatabase::begin` (MAIR-391): several writes applied together or not at all.
#[cfg(test)]
mod transaction_tests {
    use super::*;

    const CACHE_KEY: &str = "test:tx:values";

    #[derive(Debug, Clone, Deserialize)]
    struct CreateTable;

    impl ApiRequestDto for CreateTable {
        fn query_sql(&self) -> &'static str {
            "CREATE TABLE IF NOT EXISTS smart_tx_test (label TEXT NOT NULL)"
        }
        fn query_params(&self) -> &[QueryParam] {
            &[]
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    struct InsertLabel {
        params: Vec<QueryParam>,
    }

    impl InsertLabel {
        fn new(label: &str) -> Self {
            Self {
                params: vec![QueryParam::Text(label.to_string())],
            }
        }
    }

    impl ApiRequestDto for InsertLabel {
        fn query_sql(&self) -> &'static str {
            "INSERT INTO smart_tx_test (label) VALUES ($1)"
        }
        fn query_params(&self) -> &[QueryParam] {
            &self.params
        }
        fn cache_key(&self) -> Option<String> {
            Some(CACHE_KEY.to_string())
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    struct CountLabel {
        params: Vec<QueryParam>,
    }

    impl CountLabel {
        fn new(label: &str) -> Self {
            Self {
                params: vec![QueryParam::Text(label.to_string())],
            }
        }
    }

    impl ApiRequestDto for CountLabel {
        fn query_sql(&self) -> &'static str {
            "SELECT count(*) FROM smart_tx_test WHERE label = $1"
        }
        fn query_params(&self) -> &[QueryParam] {
            &self.params
        }
    }

    async fn smart_db(redis_url: &str) -> (SmartDatabase, Redis) {
        let (_db_container, db_host) = get_shared_db().await;
        let redis = Redis::new(redis_url);
        let smart_db = SmartDatabase::new(Database::new(db_host.as_str()).await, redis.clone());
        smart_db.execute(CreateTable).await.unwrap();
        (smart_db, redis)
    }

    #[tokio::test]
    #[serial]
    async fn test_commit_applies_every_write_then_invalidates_the_cache() {
        let (_redis_node, redis_config) = start_redis_container().await;
        let (smart_db, redis) = smart_db(&redis_config.url).await;
        redis.set(CACHE_KEY, "stale").await.unwrap();

        let mut tx = smart_db.begin().await.unwrap();
        tx.execute(&InsertLabel::new("commit")).await.unwrap();
        tx.execute(&InsertLabel::new("commit")).await.unwrap();
        let seen_inside: i64 = tx.fetch_scalar(&CountLabel::new("commit")).await.unwrap();
        let seen_outside: i64 = smart_db
            .fetch_scalar(&CountLabel::new("commit"))
            .await
            .unwrap();

        assert_eq!(seen_inside, 2, "the transaction sees its own writes");
        assert_eq!(seen_outside, 0, "nobody else sees them before the commit");
        assert!(
            redis.key_exist(CACHE_KEY).await.unwrap(),
            "the cache is only invalidated on commit"
        );

        tx.commit().await.unwrap();

        let committed: i64 = smart_db
            .fetch_scalar(&CountLabel::new("commit"))
            .await
            .unwrap();
        assert_eq!(committed, 2);
        assert!(!redis.key_exist(CACHE_KEY).await.unwrap());
    }

    #[tokio::test]
    #[serial]
    async fn test_dropped_or_failed_transactions_apply_nothing() {
        let (_redis_node, redis_config) = start_redis_container().await;
        let (smart_db, redis) = smart_db(&redis_config.url).await;
        redis.set(CACHE_KEY, "kept").await.unwrap();

        {
            let mut tx = smart_db.begin().await.unwrap();
            tx.execute(&InsertLabel::new("dropped")).await.unwrap();
            // Dropped without commit, like an early `?` return.
        }
        let mut tx = smart_db.begin().await.unwrap();
        tx.execute(&InsertLabel::new("rolled-back")).await.unwrap();
        tx.rollback().await.unwrap();

        for label in ["dropped", "rolled-back"] {
            let count: i64 = smart_db
                .fetch_scalar(&CountLabel::new(label))
                .await
                .unwrap();
            assert_eq!(count, 0, "{label}");
        }
        assert!(
            redis.key_exist(CACHE_KEY).await.unwrap(),
            "nothing was written, nothing is invalidated"
        );
    }

    /// Error type of an API endpoint: `transaction` only needs it to accept an `ApiLibError`.
    #[derive(Debug, PartialEq, Eq)]
    enum EndpointError {
        Database,
        Refused,
    }

    impl From<ApiLibError> for EndpointError {
        fn from(_: ApiLibError) -> Self {
            Self::Database
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_transaction_helper_commits_on_ok() {
        let (_redis_node, redis_config) = start_redis_container().await;
        let (smart_db, redis) = smart_db(&redis_config.url).await;
        redis.set(CACHE_KEY, "stale").await.unwrap();

        let seen_inside = smart_db
            .transaction(async |tx| {
                tx.execute(&InsertLabel::new("helper-ok")).await?;
                tx.execute(&InsertLabel::new("helper-ok")).await?;
                tx.fetch_scalar::<i64, _>(&CountLabel::new("helper-ok"))
                    .await
            })
            .await
            .unwrap();

        let committed: i64 = smart_db
            .fetch_scalar(&CountLabel::new("helper-ok"))
            .await
            .unwrap();
        assert_eq!(seen_inside, 2, "the closure's value is returned");
        assert_eq!(committed, 2);
        assert!(
            !redis.key_exist(CACHE_KEY).await.unwrap(),
            "invalidated after the commit"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_transaction_helper_rolls_back_on_err() {
        let (_redis_node, redis_config) = start_redis_container().await;
        let (smart_db, redis) = smart_db(&redis_config.url).await;
        redis.set(CACHE_KEY, "kept").await.unwrap();

        // Refused by the endpoint's own logic after a first write.
        let refused: Result<(), EndpointError> = smart_db
            .transaction(async |tx| {
                tx.execute(&InsertLabel::new("helper-refused")).await?;
                Err(EndpointError::Refused)
            })
            .await;
        // Failed in Postgres after a first write (NULL in a NOT NULL column).
        let failed: Result<(), EndpointError> = smart_db
            .transaction(async |tx| {
                tx.execute(&InsertLabel::new("helper-failed")).await?;
                tx.execute(&InsertNull).await?;
                Ok(())
            })
            .await;

        assert_eq!(
            refused,
            Err(EndpointError::Refused),
            "the error is kept as is"
        );
        assert_eq!(failed, Err(EndpointError::Database));
        for label in ["helper-refused", "helper-failed"] {
            let count: i64 = smart_db
                .fetch_scalar(&CountLabel::new(label))
                .await
                .unwrap();
            assert_eq!(count, 0, "{label}");
        }
        assert!(redis.key_exist(CACHE_KEY).await.unwrap());
    }

    #[derive(Debug, Clone, Deserialize)]
    struct InsertNull;

    impl ApiRequestDto for InsertNull {
        fn query_sql(&self) -> &'static str {
            "INSERT INTO smart_tx_test (label) VALUES (NULL)"
        }
        fn query_params(&self) -> &[QueryParam] {
            &[]
        }
    }
}
