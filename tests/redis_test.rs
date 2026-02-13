#[cfg(test)]
mod unsecured_redis_tests {
    use mairie360_api_lib::redis::simple_key::{add_key, delete_key, get_key, key_exist, set_key};
    use redis::Client;
    use testcontainers::core::{ContainerPort, WaitFor};
    use testcontainers::runners::AsyncRunner;
    use testcontainers::GenericImage;

    #[tokio::test]
    async fn test_add_key_success() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()), got {:?}", result);
    }

    #[tokio::test]
    async fn test_add_key_failure() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let res1 = add_key(&mut conn, "unique_key", "value1").await;
        assert!(res1.is_ok());

        let res2 = add_key(&mut conn, "unique_key", "value2").await;
        assert!(
            res2.is_err(),
            "The second addition should fail due to duplicate key"
        );
    }

    #[tokio::test]
    async fn test_get_key() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        add_key(&mut conn, "test_key", "test_value").await.unwrap();

        let result = get_key(&mut conn, "test_key").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_key_failure() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        assert!(get_key(&mut conn, "non_existent_key").await.is_err());
    }

    #[tokio::test]
    async fn test_set_key() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        set_key(&mut conn, "test_key", "test_value").await.unwrap();
        let value = get_key(&mut conn, "test_key").await.unwrap();
        assert_eq!(value, "test_value");
    }

    #[tokio::test]
    async fn test_delete_key() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        add_key(&mut conn, "test_key", "test_value").await.unwrap();
        delete_key(&mut conn, "test_key").await.unwrap();
        assert!(get_key(&mut conn, "test_key").await.is_err());
    }

    #[tokio::test]
    async fn test_delete_key_failure() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        assert!(delete_key(&mut conn, "non_existent_key").await.is_err());
    }

    #[tokio::test]
    async fn test_unset_key_exists() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let exists = key_exist(&mut conn, "non_existent_key").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_set_key_exists() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        set_key(&mut conn, "test_key", "test_value").await.unwrap();
        let exists = key_exist(&mut conn, "test_key").await.unwrap();
        assert!(exists);
    }
}

mod secured_redis_tests {
    use mairie360_api_lib::redis::simple_key::{secure_add_key, secure_delete_key, secure_get_key};
    use redis::Client;
    use testcontainers::core::{ContainerPort, WaitFor};
    use testcontainers::runners::AsyncRunner;
    use testcontainers::GenericImage;

    #[tokio::test]
    async fn test_secure_add_key_success() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        let result = secure_add_key(&mut conn, "test_key", "test_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_secure_add_key_failure() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        secure_add_key(&mut conn, "test_key", "test_value")
            .await
            .unwrap();
        let result = secure_add_key(&mut conn, "test_key", "new_value").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_secure_get_key() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        secure_add_key(&mut conn, "test_key", "test_value")
            .await
            .unwrap();
        let value = secure_get_key(&mut conn, "test_key").await.unwrap();
        assert_eq!(value, "test_value");
    }

    #[tokio::test]
    async fn test_secure_get_key_failure() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        assert!(secure_get_key(&mut conn, "test_key").await.is_err());
    }

    #[tokio::test]
    async fn test_secure_delete_key() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        secure_add_key(&mut conn, "test_key", "test_value")
            .await
            .unwrap();
        assert!(secure_delete_key(&mut conn, "test_key").await.is_ok());
    }

    #[tokio::test]
    async fn test_secure_delete_key_failure() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");
        let port = node
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port");
        let client = Client::open(format!("redis://127.0.0.1:{}", port)).unwrap();
        let mut conn = client.get_connection().expect("Failed to connect");

        assert!(secure_delete_key(&mut conn, "test_key").await.is_err());
    }
}

#[cfg(test)]
mod redis_manager_test {
    use mairie360_api_lib::redis::redis_manager::{
        create_redis_manager, get_redis_manager, RedisManager,
    };
    use serial_test::serial;
    use temp_env;
    use testcontainers::core::{ContainerPort, WaitFor};
    use testcontainers::runners::AsyncRunner;
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
        let _node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        temp_env::with_var_unset("REDIS_URL", || {
            RedisManager::new();
        });
    }

    #[tokio::test]
    #[should_panic]
    #[serial]
    async fn test_create_redis_manager_failure_with_bad_url() {
        let _node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        temp_env::with_var("REDIS_URL", Some("invalid_url"), || {
            RedisManager::new();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_create_redis_manager_success() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://127.0.0.1:{}", port);

        temp_env::with_var("REDIS_URL", Some(&redis_url), || {
            RedisManager::new();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_set_redis_manager_singleton() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://127.0.0.1:{}", port);

        std::env::set_var("REDIS_URL", redis_url);
        create_redis_manager().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_get_redis_manager_singleton() {
        let node = GenericImage::new("redis", "7.2.4")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .start()
            .await
            .expect("Failed to start Redis");

        let port = node.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://127.0.0.1:{}", port);

        std::env::set_var("REDIS_URL", redis_url);
        create_redis_manager().await;
        let redis_manager = get_redis_manager().await;
        assert!(redis_manager.is_some());
    }
}
