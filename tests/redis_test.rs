use mairie360_api_lib::{
    redis::redis_interface::Redis, test_setup::redis_setup::start_redis_container,
};

#[cfg(test)]
mod unsecured_redis_tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_set_success() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let response = redis_interface.set("test_key", "test_value").await;

        assert!(response.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_set_failure() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let first_response = redis_interface.set("unique_key", "value1").await;
        assert!(first_response.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_get() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let first_response = redis_interface.set("get_unique_key", "value1").await;
        assert!(first_response.is_ok());

        let second_response = redis_interface.get::<String>("get_unique_key").await;
        assert!(
            second_response.is_ok(),
            "The key should be found after a successful first add, got error: {second_response:?}"
        );

        let value = second_response.unwrap();
        assert_eq!(
            value, "value1",
            "The value retrieved should match the value added, got {value} instead"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_delete() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let _ = redis_interface.set("test_key", "test_value").await;
        let result = redis_interface.delete("test_key").await;
        assert!(
            result.is_ok(),
            "Key should be deleted and return an error on GET"
        );

        let get_result = redis_interface.get::<String>("test_key").await;
        assert!(
            get_result.is_err(),
            "Key should not be found after deletion"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_check_key() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let _ = redis_interface.set("test_key", "test_value").await;
        let result = redis_interface.key_exist("test_key").await;
        assert!(result.is_ok(), "Key should be found");

        let result = redis_interface.key_exist("non_existent_key").await;
        assert!(result.is_ok(), "Non-existent key should a valid result");
        let exists = result.unwrap();
        assert!(!exists, "Non-existent key should not be found");
    }
}

#[cfg(test)]
mod safe_redis_tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_secure_set_success() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let result = redis_interface.secure_set("test_key", "test_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_get() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let _ = redis_interface
            .secure_set("get_secured_key", "test_value")
            .await;

        let result = redis_interface
            .secure_get::<String>("get_secured_key")
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_delete_key() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        let _ = redis_interface
            .secure_set("delete_secured_key", "test_value")
            .await;
        let result = redis_interface.secure_delete("delete_secured_key").await;
        assert!(result.is_ok());

        let result = redis_interface
            .secure_get::<String>("delete_secured_key")
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value.is_none());

        // secure_get renvoie Result<Option<T>, RedisError>
        let result = redis_interface
            .secure_get::<String>("delete_secured_key")
            .await;

        assert!(result.is_ok());
        assert!(
            result.unwrap().is_none(),
            "Key should not be found after deletion"
        );
    }
}

#[cfg(test)]
mod redis_ttl_tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_expire_success() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        // 1. On crée une clé
        let _ = redis_interface.set("expire_key", "expire_value").await;

        // 2. On applique un TTL de 10 secondes
        let result = redis_interface.expire("expire_key", 10).await;
        assert!(result.is_ok(), "L'application du TTL devrait réussir");
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_expire_on_existing_key() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        // 1. On crée une clé via secure_set
        let _ = redis_interface
            .secure_set("secure_expire_key", "value")
            .await;

        // 2. secure_expire sur une clé existante doit réussir
        let result = redis_interface.secure_expire("secure_expire_key", 30).await;
        assert!(
            result.is_ok(),
            "secure_expire devrait réussir si la clé existe"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_expire_on_non_existent_key() {
        let (_node, config) = start_redis_container().await;
        let redis_interface = Redis::new(&config.url);

        // secure_expire sur une clé qui n'existe pas ne doit pas planter (comportement no-op sécurisé)
        let result = redis_interface.secure_expire("ghost_key", 30).await;
        assert!(
            result.is_ok(),
            "secure_expire sur une clé inexistante doit renvoyer Ok (comportement sécurisé)"
        );
    }
}
