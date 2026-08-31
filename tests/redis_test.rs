use mairie360_api_lib::{
    redis::redis_interface::{Redis, RedisParam, RedisRequestDto},
    test_setup::redis_setup::start_redis_container,
};

struct TestDto {
    key: String,
    params: Vec<RedisParam>,
}

impl TestDto {
    fn new(key: &str, params: Vec<RedisParam>) -> Self {
        Self {
            key: key.to_string(),
            params,
        }
    }
}

impl RedisRequestDto for TestDto {
    fn key(&self) -> &str {
        &self.key
    }

    fn args(&self) -> &[RedisParam] {
        &self.params
    }
}

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

        let params = vec![];
        let dto = TestDto::new("get_unique_key", params);
        let second_response = redis_interface.get::<String, TestDto>(dto).await;
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

        let params = vec![];
        let dto = TestDto::new("test_key", params);
        let get_result = redis_interface.get::<String, TestDto>(dto).await;
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

        let params = vec![];
        let dto = TestDto::new("get_secured_key", params);
        let result = redis_interface.secure_get::<String, TestDto>(dto).await;
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

        let params = vec![];
        let dto = TestDto::new("delete_secured_key", params);

        // secure_get renvoie Result<Option<T>, RedisError>
        let result = redis_interface.secure_get::<String, TestDto>(dto).await;

        assert!(result.is_ok());
        assert!(
            result.unwrap().is_none(),
            "Key should not be found after deletion"
        );
    }
}
