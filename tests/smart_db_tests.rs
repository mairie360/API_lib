use mairie360_api_lib::database::db_interface::{ApiRequestDto, Database, QueryParam};
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
        let (_db_container, db_host) = get_shared_db().await;
        let (_redis_node, redis_config) = start_redis_container().await;

        let db = Database::new(db_host.as_str()).await;
        let redis = Redis::new(&redis_config.url);
        let smart_db = SmartDatabase::new(db, redis.clone());

        let view = CachedUserExistsView::new(1);
        let cache_key = "test:user:exists:1";

        let _: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert!(redis.key_exist(cache_key).await.unwrap());

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
