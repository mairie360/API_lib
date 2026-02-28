pub mod common;
use common::redis_setup::{start_redis_container, set_redis_env_var, get_redis_connection};

#[cfg(test)]
mod unsecured_redis_tests {
    use super::*;
    use mairie360_api_lib::redis::simple_key::{add_key, delete_key, get_key, key_exist, set_key};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_add_key_success() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        let result = add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    #[serial]
    async fn test_add_key_failure() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        add_key(&mut conn, "unique_key", "value1").await.unwrap();
        let res2 = add_key(&mut conn, "unique_key", "value2").await;

        assert!(res2.is_err(), "The second addition should fail due to duplicate key");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_key() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        add_key(&mut conn, "test_key", "test_value").await.unwrap();
        let result = get_key(&mut conn, "test_key").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_set_key() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        set_key(&mut conn, "test_key", "test_value").await.unwrap();
        let value = get_key(&mut conn, "test_key").await.unwrap();
        assert_eq!(value, "test_value");
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_key() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        add_key(&mut conn, "test_key", "test_value").await.unwrap();
        delete_key(&mut conn, "test_key").await.unwrap();
        assert!(get_key(&mut conn, "test_key").await.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_set_key_exists() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        set_key(&mut conn, "test_key", "test_value").await.unwrap();
        let exists = key_exist(&mut conn, "test_key").await.unwrap();
        assert!(exists);
    }
}

#[cfg(test)]
mod safe_redis_tests {
    use super::*;
    use mairie360_api_lib::redis::simple_key::{secure_add_key, secure_delete_key, secure_get_key};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_secure_add_key_success() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        let result = secure_add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_get_key() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        secure_add_key(&mut conn, "test_key", "test_value").await.unwrap();
        let value = secure_get_key(&mut conn, "test_key").await.unwrap();
        assert_eq!(value, "test_value");
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_delete_key() {
        let (_node, config) = start_redis_container().await;
        let mut conn = get_redis_connection(&config).await;

        secure_add_key(&mut conn, "test_key", "test_value").await.unwrap();
        secure_delete_key(&mut conn, "test_key").await.unwrap();
        assert!(secure_get_key(&mut conn, "test_key").await.is_err());
    }
}

#[cfg(test)]
mod redis_manager_test {
    use super::*;
    use mairie360_api_lib::redis::redis_manager::{create_redis_manager, get_redis_manager, RedisManager};
    use serial_test::serial;
    use temp_env;

    #[tokio::test]
    #[serial]
    async fn test_create_redis_manager_success() {
        let (_node, config) = start_redis_container().await;
        let redis_url = config.url.clone();

        temp_env::with_var("REDIS_URL", Some(&redis_url), || {
            // Ici on teste le constructeur direct
            let _manager = RedisManager::new();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_get_redis_manager_singleton() {
        let (_node, config) = start_redis_container().await;

        // On utilise notre helper de setup pour l'env var
        set_redis_env_var(&config);

        create_redis_manager().await;
        let redis_manager = get_redis_manager().await;
        assert!(redis_manager.is_some());
    }
}
