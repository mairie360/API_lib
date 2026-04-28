use mairie360_api_lib::test_setup::queries_setup::get_shared_db;

use mairie360_api_lib::database::queries::QueryError;
use mairie360_api_lib::database::queries::{
    does_user_exist_by_email_query, does_user_exist_by_id_query, is_session_token_valid_query,
};
use mairie360_api_lib::database::query_views::{
    DoesUserExistByEmailQueryView, DoesUserExistByIdQueryView, IsSessionTokenValidQueryView,
};
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::IpAddr;
#[cfg(test)]
mod queries_tests {
    use super::*;
    use mairie360_api_lib::database::{
        errors::DatabaseError, queries::has_access_query, query_views::HasAccessQueryView,
    };

    async fn get_pool(url: String) -> PgPool {
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(&url)
            .await
            .expect("Failed to create Postgres pool")
    }

    #[cfg(test)]
    mod does_user_exist_by_id_tests {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_user_exists_by_id() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;
            let view = DoesUserExistByIdQueryView::new(1);

            // On passe pool car les fonctions attendent désormais une référence
            let result = does_user_exist_by_id_query(view, pool).await.unwrap();

            assert!(result); // Plus propre que assert_eq!(result, true)
        }

        #[tokio::test]
        #[serial]
        async fn test_user_id_not_found() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;
            let view = DoesUserExistByIdQueryView::new(999);

            let result = does_user_exist_by_id_query(view, pool).await.unwrap();

            assert!(!result);
        }
    }

    #[cfg(test)]
    mod does_user_exist_by_email_tests {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_user_exists_by_email_success() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;
            let view = DoesUserExistByEmailQueryView::new("alice@example.com".to_string());

            let result = does_user_exist_by_email_query(view, pool).await.unwrap();

            assert!(result);
        }

        #[tokio::test]
        #[serial]
        async fn test_user_email_not_found() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;
            let view = DoesUserExistByEmailQueryView::new("unknown@example.com".to_string());

            let result = does_user_exist_by_email_query(view, pool).await.unwrap();

            assert!(!result);
        }

        #[tokio::test]
        #[serial]
        async fn test_user_exists_by_email_invalid_format() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;
            let email = "invalid-email";
            let view = DoesUserExistByEmailQueryView::new(email.to_string());

            let result = does_user_exist_by_email_query(view, pool).await;

            // Ici on valide que ton From<sqlx::Error> ou ta validation manuelle fonctionne
            assert!(result.is_err());
            let err = result.err().unwrap();

            assert_eq!(
                err,
                DatabaseError::Query(QueryError::InvalidEmailFormat(email.to_string()))
            );
        }

        #[tokio::test]
        #[serial]
        async fn test_sql_injection_email_query() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            // Tentative d'injection : si c'était vulnérable, EXISTS retournerait true ou ferait une erreur
            let malicious_email = "' OR 1=1 --";
            let view = DoesUserExistByEmailQueryView::new(malicious_email.to_string());

            let result = does_user_exist_by_email_query(view, pool).await;

            // Comme il n'y a pas de '@', ta fonction renvoie l'erreur de format AVANT la DB
            assert!(result.is_err());
            if let Err(DatabaseError::Query(QueryError::InvalidEmailFormat(_))) = result {
                assert!(true);
            } else {
                panic!("Should have failed with InvalidEmailFormat");
            }
        }
    }

    #[cfg(test)]
    mod is_session_token_valid_tests {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_is_session_token_valid() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = IsSessionTokenValidQueryView::new(
                1,
                "test_token_hash_unique_123".to_string(),
                IpAddr::from([127, 0, 0, 1]),
            );

            let result = is_session_token_valid_query(view, pool).await.unwrap();

            assert!(result);
        }

        #[tokio::test]
        #[serial]
        async fn test_is_session_token_expired() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = IsSessionTokenValidQueryView::new(
                1,
                "test_token_hash_expired".to_string(),
                IpAddr::from([127, 0, 0, 1]),
            );

            let result = is_session_token_valid_query(view, pool).await.unwrap();

            assert!(!result);
        }

        #[tokio::test]
        #[serial]
        async fn test_is_session_ip_invalid() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = IsSessionTokenValidQueryView::new(
                1,
                "test_token_hash_unique_123".to_string(),
                IpAddr::from([127, 0, 0, 2]),
            );

            let result = is_session_token_valid_query(view, pool).await.unwrap();

            assert!(!result);
        }

        #[tokio::test]
        #[serial]
        async fn test_is_session_invalid_archived_user() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = IsSessionTokenValidQueryView::new(
                3,
                "test_token_hash_unique_123".to_string(),
                IpAddr::from([127, 0, 0, 1]),
            );

            let result = is_session_token_valid_query(view, pool).await.unwrap();

            assert!(!result);
        }
    }

    #[cfg(test)]
    mod has_access_tests {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_has_access_global_admin() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            // Alice (ID 1) est admin, elle a 'read_all' sur 'document'
            let view = HasAccessQueryView::new(1, "document", "read", 10);

            let result = has_access_query(view, pool.clone()).await.unwrap();

            assert!(result);
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_ownership() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = HasAccessQueryView::new(1, "document", "read", 10);

            let result = has_access_query(view, pool.clone()).await.unwrap();

            assert!(result);
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_individual_acl() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = HasAccessQueryView::new(2, "groups", "read", 50);

            let result = has_access_query(view, pool).await.unwrap();

            assert!(
                result,
                "Guest (2) should have individual ACL access to group 50"
            );
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_denied() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = HasAccessQueryView::new(1, "document", "read", 1);

            let result = has_access_query(view, pool.clone()).await.unwrap();

            assert!(!result);
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_invalid_resource() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;

            let view = HasAccessQueryView::new(1, "ghost_resource", "read", 0);

            let result = has_access_query(view, pool.clone()).await.unwrap();

            assert!(!result);
        }
    }
}
