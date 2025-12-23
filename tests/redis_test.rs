#[cfg(test)]
mod unsecured_redis_tests {
    use mairie360_api_lib::redis::simple_key::{add_key, delete_key, get_key, key_exist, set_key};
    use redis::Client;
    use testcontainers::clients::Cli;
    use testcontainers::GenericImage;

    #[tokio::test]
    async fn test_add_key_success() {
        let docker = Cli::default();

        // On définit l'image redis manuellement
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);

        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();

        let mut conn = client.get_connection().expect("Failed to connect");

        let result = add_key(&mut conn, "test_key", "test_value").await;

        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    async fn test_add_key_failure() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let res1 = add_key(&mut conn, "unique_key", "value1").await;
        println!("Premier ajout: {:?}", res1);
        assert!(res1.is_ok(), "Le premier ajout devrait réussir");

        let res2 = add_key(&mut conn, "unique_key", "value2").await;
        println!("Deuxième ajout: {:?}", res2);

        assert!(
            res2.is_err(),
            "The second addition should fail due to duplicate key"
        );
    }

    //test get key

    #[tokio::test]
    async fn test_get_key() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        match add_key(&mut conn, "test_key", "test_value").await {
            Ok(_) => (),
            Err(e) => panic!("Failed to add key for get_key test: {:?}", e),
        }

        let result = get_key(&mut conn, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(value), got {:?}", result);
    }

    #[tokio::test]
    async fn test_get_key_failure() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        assert!(get_key(&mut conn, "non_existent_key").await.is_err());
    }

    #[tokio::test]
    async fn test_set_key() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = set_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = get_key(&mut conn, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(value), got {:?}", result);

        let value = result.unwrap();
        assert_eq!(
            value, "test_value",
            "Expected value to be 'test_value', got {}",
            value
        );
    }

    #[tokio::test]
    async fn test_delete_key() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = delete_key(&mut conn, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = get_key(&mut conn, "test_key").await;
        assert!(result.is_err(), "Expected Err(_), got {:?}", result);
    }

    #[tokio::test]
    async fn test_delete_key_failure() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        assert!(
            delete_key(&mut conn, "non_existent_key").await.is_err(),
            "Expected Err(_) when deleting non_existent_key"
        );
    }

    #[tokio::test]
    async fn test_unset_key_exist() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = key_exist(&mut conn, "non_existent_key").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let exists = result.unwrap();
        assert!(!exists, "Expected key to not exist");
    }

    #[tokio::test]
    async fn test_set_key_exist() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = set_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = key_exist(&mut conn, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let exists = result.unwrap();
        assert!(exists, "Expected key to exist");
    }
}

mod secured_redis_tests {
    use mairie360_api_lib::redis::simple_key::{secure_add_key, secure_delete_key, secure_get_key};
    use redis::Client;
    use testcontainers::clients::Cli;
    use testcontainers::GenericImage;

    #[tokio::test]
    async fn test_secure_add_key_success() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    async fn test_secure_add_key_failure() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = secure_add_key(&mut conn, "test_key", "new_value").await;
        assert!(result.is_err(), "Expected Err(_), got {:?}", result);
    }

    #[tokio::test]
    async fn test_secure_get_key() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = secure_get_key(&mut conn, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let value = result.unwrap();
        assert_eq!(
            value, "test_value",
            "Expected value to be 'test_value', got {:?}",
            value
        );
    }

    #[tokio::test]
    async fn test_secure_get_key_failure() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);

        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_get_key(&mut conn, "test_key").await;
        assert!(result.is_err(), "Expected Err(_), got {:?}", result);
    }

    #[tokio::test]
    async fn test_secure_delete_key() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);

        let result = secure_delete_key(&mut conn, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    async fn test_secure_delete_key_failure() {
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        let node = docker.run(redis_image);
        let port = node.get_host_port_ipv4(6379);
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_delete_key(&mut conn, "test_key").await;
        assert!(result.is_err(), "Expected Err(_), got {:?}", result);
    }
}

#[cfg(test)]
mod redis_manager_test {
    use mairie360_api_lib::redis::redis_manager::{
        create_redis_manager, get_redis_manager, RedisManager,
    };
    use serial_test::serial;
    use temp_env;
    use testcontainers::clients::Cli;
    use testcontainers::GenericImage;

    #[tokio::test]
    #[should_panic]
    async fn test_create_redis_manager_failure_without_starting_redis() {
        RedisManager::new();
    }

    #[tokio::test]
    #[should_panic]
    #[serial]
    async fn test_create_redis_manager_failure_without_env_var() {
        temp_env::with_var_unset("REDIS_URL", || {
            let docker = Cli::default();
            let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
            docker.run(redis_image);
            assert!(
                std::env::var("REDIS_URL").is_err(),
                "REDIS_URL should not be set"
            );

            RedisManager::new();
        });
    }

    #[tokio::test]
    #[should_panic]
    #[serial]
    async fn test_create_redis_manager_failure_with_bad_url() {
        temp_env::with_var("REDIS_URL", Some("invalid_url"), || {
            let docker = Cli::default();
            let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
            docker.run(redis_image);

            assert!(
                std::env::var("REDIS_URL").is_ok(),
                "REDIS_URL should be set"
            );
            assert_eq!(
                std::env::var("REDIS_URL").unwrap(),
                "invalid_url",
                "REDIS_URL should be 'invalid_url'"
            );
            RedisManager::new();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_create_redis_manager_success() {
        temp_env::with_var("REDIS_URL", Some("redis://127.0.0.1:6379"), || {
            let docker = Cli::default();
            let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
            docker.run(redis_image);

            assert!(
                std::env::var("REDIS_URL").is_ok(),
                "REDIS_URL should be set"
            );
            assert_eq!(
                std::env::var("REDIS_URL").unwrap(),
                "redis://127.0.0.1:6379",
                "REDIS_URL should be 'redis://127.0.0.1:6379'"
            );
            RedisManager::new();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_set_redis_manager_singleton() {
        std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");
        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        docker.run(redis_image);

        assert!(
            std::env::var("REDIS_URL").is_ok(),
            "REDIS_URL should be set"
        );
        assert_eq!(
            std::env::var("REDIS_URL").unwrap(),
            "redis://127.0.0.1:6379",
            "REDIS_URL should be 'redis://127.0.0.1:6379'"
        );
        create_redis_manager().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_get_redis_manager_singleton() {
        std::env::set_var("REDIS_URL", "redis://127.0.0.1:6379");

        let docker = Cli::default();
        let redis_image = GenericImage::new("redis", "7.2.4").with_exposed_port(6379);
        docker.run(redis_image);

        assert!(
            std::env::var("REDIS_URL").is_ok(),
            "REDIS_URL should be set"
        );
        assert_eq!(
            std::env::var("REDIS_URL").unwrap(),
            "redis://127.0.0.1:6379",
            "REDIS_URL should be 'redis://127.0.0.1:6379'"
        );
        create_redis_manager().await;
        let redis_manager = get_redis_manager().await;
        assert!(redis_manager.is_some());
    }
}
