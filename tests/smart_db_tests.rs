use mairie360_api_lib::database::db_interface::{ApiRequestDto, Database, QueryParam};
use mairie360_api_lib::redis::redis_interface::Redis;
use mairie360_api_lib::smart_db::SmartDatabase;
use mairie360_api_lib::test_setup::{
    queries_setup::get_shared_db, redis_setup::start_redis_container,
};
use serde::Deserialize;
use serial_test::serial;

/// Une vue de test personnalisée pour valider le cache sur un scalaire (ex: existence d'un utilisateur)
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

    // On active dynamiquement le cache pour cette vue
    fn cache_key(&self) -> Option<String> {
        let key = format!("test:user:exists:{}", self.user_id);
        Some(key)
    }
}

#[cfg(test)]
mod smart_database_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_smart_db_cache_aside_flow() {
        // 1. Initialisation des deux conteneurs (Postgres + Redis)
        let (_db_container, db_host) = get_shared_db().await;
        let (_redis_node, redis_config) = start_redis_container().await;

        let db = Database::new(db_host.as_str()).await;
        let redis = Redis::new(&redis_config.url);
        let smart_db = SmartDatabase::new(db, redis.clone());

        let view = CachedUserExistsView::new(1);
        let cache_key = "test:user:exists:1";

        // S'assurer que le cache est vide au départ
        let _ = redis.delete(cache_key).await;
        assert!(!redis.key_exist(cache_key).await.unwrap());

        // 2. Premier appel : Cache Miss -> Doit interroger la DB et remplir le cache
        let first_result: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert!(first_result);

        // 3. Vérification que Redis a bien stocké la valeur
        assert!(
            redis.key_exist(cache_key).await.unwrap(),
            "La clé doit être présente dans Redis après un fetch réussi"
        );

        // 4. Deuxième appel : Cache Hit -> Récupère directement depuis Redis
        let second_result: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert_eq!(first_result, second_result);
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

        // 1. On peuple le cache manuellement ou via un fetch
        let _: bool = smart_db.fetch_scalar(&view).await.unwrap();
        assert!(redis.key_exist(cache_key).await.unwrap());

        // 2. Simulation d'une action d'écriture (ex: update ou delete fictif sur le user)
        // On crée une vue d'écriture simple qui partage la même clé de cache pour l'invalidation
        #[derive(Debug, Clone, Deserialize)]
        struct DummyUpdateView {
            params: Vec<QueryParam>,
        }
        impl ApiRequestDto for DummyUpdateView {
            fn query_sql(&self) -> &'static str {
                "SELECT 1"
            } // Requête neutre pour l'exemple
            fn query_params(&self) -> &[QueryParam] {
                &self.params
            }
            fn cache_key(&self) -> Option<String> {
                Some("test:user:exists:1".to_string())
            }
        }

        let update_view = DummyUpdateView { params: vec![] };

        // 3. L'appel à execute doit invalider (supprimer) la clé du cache
        let execute_res = smart_db.execute(update_view).await;
        assert!(execute_res.is_ok());

        // 4. Vérification que la clé a bien été supprimée de Redis
        let exists_after = redis.key_exist(cache_key).await.unwrap();
        assert!(
            !exists_after,
            "Le cache doit être invalidé (supprimé) après une exécution d'écriture"
        );
    }
}
