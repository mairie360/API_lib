use deadpool_redis::{Config, Runtime};
use mairie360_api_lib::test_setup::redis_setup::start_redis_container;

#[cfg(test)]
mod unsecured_redis_tests {
    use super::*;
    use mairie360_api_lib::pool::redis::simple_key::unsecured::{
        handle_check_key, handle_delete_data, handle_get_data, handle_post_data,
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_add_key_success() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let response =
            handle_post_data(redis_pool.get().await.unwrap(), "test_key", "test_value").await;

        assert!(response.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_add_key_failure() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let first_response =
            handle_post_data(redis_pool.get().await.unwrap(), "unique_key", "value1").await;
        assert!(first_response.is_ok());

        let second_response =
            handle_post_data(redis_pool.get().await.unwrap(), "unique_key", "value2").await;
        assert!(
            second_response.is_err(),
            "La clé existe déjà, le deuxième ajout doit échouer"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_get_key() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let first_response =
            handle_post_data(redis_pool.get().await.unwrap(), "unique_key", "value1").await;
        assert!(first_response.is_ok());

        let second_response = handle_get_data(redis_pool.get().await.unwrap(), "unique_key").await;
        assert!(
            second_response.is_ok(),
            "La clé doit être trouvée après un premier ajout réussi"
        );

        let value = second_response.unwrap();
        assert_eq!(
            value, "value1",
            "La valeur récupérée doit correspondre à la valeur ajoutée"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_key() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let _ = handle_post_data(redis_pool.get().await.unwrap(), "test_key", "test_value").await;
        let result = handle_delete_data(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(
            result.is_ok(),
            "Key should be deleted and return an error on GET"
        );

        let get_result = handle_get_data(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(
            get_result.is_err(),
            "Key should not be found after deletion"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_check_key() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let _ = handle_post_data(redis_pool.get().await.unwrap(), "test_key", "test_value").await;
        let result = handle_check_key(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(result.is_ok(), "Key should be found");

        let result = handle_check_key(redis_pool.get().await.unwrap(), "non_existent_key").await;
        assert!(result.is_err(), "Non-existent key should not be found");
    }
}

#[cfg(test)]
mod safe_redis_tests {
    use super::*;
    use mairie360_api_lib::pool::redis::simple_key::secured::{
        handle_secure_delete, handle_secure_get, handle_secure_post, handle_update_data,
    };
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_secure_add_key_success() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let result =
            handle_secure_post(redis_pool.get().await.unwrap(), "test_key", "test_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_get_key() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let _ = handle_secure_post(redis_pool.get().await.unwrap(), "test_key", "test_value").await;
        let result = handle_secure_get(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, "test_value");
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_delete_key() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let _ = handle_secure_post(redis_pool.get().await.unwrap(), "test_key", "test_value").await;
        let result = handle_secure_delete(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(result.is_ok());
        let result = handle_secure_get(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_set_key() {
        let (_node, config) = start_redis_container().await;
        let redis_cfg = Config::from_url(&config.url);
        let redis_pool = redis_cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Failed to create Redis pool");

        let _ = handle_secure_post(redis_pool.get().await.unwrap(), "test_key", "test_value").await;
        let result = handle_secure_get(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, "test_value");
        let result =
            handle_update_data(redis_pool.get().await.unwrap(), "test_key", "updated_value").await;
        assert!(result.is_ok());
        let result = handle_secure_get(redis_pool.get().await.unwrap(), "test_key").await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value, "updated_value");
    }
}
