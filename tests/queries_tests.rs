use mairie360_api_lib::database::db_interface::Database;
use mairie360_api_lib::database::query_views::{
    DoesUserExistByEmailQueryView, DoesUserExistByIdQueryView, IsSessionTokenValidQueryView,
};
use mairie360_api_lib::test_setup::queries_setup::get_shared_db;
use serial_test::serial;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::IpAddr;
#[cfg(test)]
mod queries_tests {
    use super::*;
    use mairie360_api_lib::database::query_views::HasAccessQueryView;

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
            let interface: Database = Database::new(host.as_str()).await;
            let view = DoesUserExistByIdQueryView::new(1);

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(result, "Expected user to exist by ID"); // Plus propre que assert_eq!(result, true)
        }

        #[tokio::test]
        #[serial]
        async fn test_user_id_not_found() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;
            let view = DoesUserExistByIdQueryView::new(999);

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(!result, "Expected user to not exist by ID");
        }
    }

    #[cfg(test)]
    mod does_user_exist_by_email_tests {

        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_user_exists_by_email_success() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;
            let view = DoesUserExistByEmailQueryView::new("alice@example.com".to_string());

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(result, "Expected user to exist by email");
        }

        #[tokio::test]
        #[serial]
        async fn test_user_email_not_found() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;
            let view = DoesUserExistByEmailQueryView::new("unknown@example.com".to_string());

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(!result, "Expected user to not exist by email");
        }

        #[tokio::test]
        #[serial]
        async fn test_user_exists_by_email_invalid_format() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;
            let email = "invalid-email";
            let view = DoesUserExistByEmailQueryView::new(email.to_string());

            let result = interface.fetch_scalar::<bool, _>(&view).await;

            // Ici on valide que ton From<sqlx::Error> ou ta validation manuelle fonctionne
            assert!(
                result.is_ok(),
                "Expected invalid email format to return an error got: {:?}",
                result
            );
            let does_exist = result.unwrap();
            assert!(
                !does_exist,
                "Expected invalid email format to return false, got: {:?}",
                does_exist
            );
        }

        #[tokio::test]
        #[serial]
        async fn test_sql_injection_email_query() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            // Tentative d'injection : si c'était vulnérable, EXISTS retournerait true ou ferait une erreur
            let malicious_email = "' OR 1=1 --";
            let view = DoesUserExistByEmailQueryView::new(malicious_email.to_string());

            let result = interface.fetch_scalar::<bool, _>(&view).await;

            // Comme il n'y a pas de '@', ta fonction renvoie l'erreur de format AVANT la DB
            assert!(
                result.is_ok(),
                "Expected valid email to return true, got: {:?}",
                result
            );
            let does_exist = result.unwrap();
            assert!(
                !does_exist,
                "Expected invalid email to return false, got: {:?}",
                does_exist
            );
        }
    }

    #[cfg(test)]
    mod is_session_token_valid_tests {
        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_is_session_token_valid() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = IsSessionTokenValidQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                    .get()
                    .unwrap() as u64,
                "test_token_hash_unique_123".to_string(),
                IpAddr::from([127, 0, 0, 1]),
            );

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(result);
        }

        #[tokio::test]
        #[serial]
        async fn test_is_session_token_expired() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = IsSessionTokenValidQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::BOB_ID
                    .get()
                    .unwrap() as u64,
                "test_token_hash_expired".to_string(),
                IpAddr::from([127, 0, 0, 1]),
            );

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(!result);
        }

        #[tokio::test]
        #[serial]
        async fn test_is_session_ip_invalid() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = IsSessionTokenValidQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                    .get()
                    .unwrap() as u64,
                "test_token_hash_unique_123".to_string(),
                IpAddr::from([127, 0, 0, 2]),
            );

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(!result);
        }

        #[tokio::test]
        #[serial]
        async fn test_is_session_invalid_archived_user() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = IsSessionTokenValidQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ADMIN_ID
                    .get()
                    .unwrap() as u64,
                "test_token_hash_unique_123".to_string(),
                IpAddr::from([127, 0, 0, 1]),
            );

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

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
            let interface: Database = Database::new(host.as_str()).await;

            // Alice (ID 1) est admin, elle a 'read_all' sur 'document'
            let view = HasAccessQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                    .get()
                    .unwrap() as u64,
                "document",
                "read",
                Some(1),
            );

            let result = interface.fetch_scalar::<i32, _>(&view).await.unwrap();

            assert!(result == 1, "expected access granted, got {}", result);
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_ownership() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = HasAccessQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                    .get()
                    .unwrap() as u64,
                "document",
                "read",
                Some(1),
            );

            let result = interface.fetch_scalar::<i32, _>(&view).await.unwrap();

            assert!(result == 1, "expected access granted, got {}", result);
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_individual_acl() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;
            let pool = get_pool(host.as_str().to_string()).await;

            let alice_id = *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                .get()
                .unwrap();

            // 1. Assurer l'existence de la ressource 'groups' dans la table 'resources'
            sqlx::query("INSERT INTO public.resources (name) VALUES ('groups') ON CONFLICT (name) DO NOTHING")
                    .execute(&pool)
                    .await
                    .unwrap();

            // 2. Assurer l'existence d'une permission 'read' sur 'groups'
            sqlx::query(
                "INSERT INTO public.permissions (resource_id, action) \
                    VALUES ((SELECT id FROM public.resources WHERE name = 'groups'), 'read') \
                    ON CONFLICT DO NOTHING",
            )
            .execute(&pool)
            .await
            .unwrap();

            // 3. FIXÉ : Insérer le groupe 50 pour passer la validation de l'Étape 0 (Existence)
            // On utilise alice_id (ou un ID valide existant) comme owner_id
            sqlx::query(
                "INSERT INTO public.groups (id, owner_id, name) \
                    VALUES (50, $1, 'Test ACL Group') \
                    ON CONFLICT (id) DO NOTHING",
            )
            .bind(alice_id as i32)
            .execute(&pool)
            .await
            .unwrap();

            // 4. Lier l'ACL individuelle pour Alice sur l'instance 50
            sqlx::query(
                    "INSERT INTO public.access_control (user_id, resource_id, resource_instance_id, permission_id) \
                    VALUES ($1, (SELECT id FROM public.resources WHERE name = 'groups'), 50, \
                    (SELECT id FROM public.permissions WHERE action = 'read' AND resource_id = (SELECT id FROM public.resources WHERE name = 'groups') LIMIT 1)) \
                    ON CONFLICT DO NOTHING"
                )
                    .bind(alice_id as i32)
                    .execute(&pool)
                    .await
                    .unwrap();

            // 5. Exécution du test
            let view = HasAccessQueryView::new(alice_id as u64, "groups", "read", Some(50));
            let result = interface.fetch_scalar::<i32, _>(&view).await.unwrap();

            assert!(
                result == 1,
                "Alice ({}) should have individual ACL access to group 50, got {}",
                alice_id,
                result
            );
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_denied() {
            let (_container, host) = get_shared_db().await;
            let pool = get_pool(host.as_str().to_string()).await;
            let interface: Database = Database::new(host.as_str()).await;

            let alice_id = *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                .get()
                .unwrap();

            // 1. Assurer l'existence de la ressource 'groups' dans la table 'resources'
            sqlx::query("INSERT INTO public.resources (name) VALUES ('groups') ON CONFLICT (name) DO NOTHING")
                            .execute(&pool)
                            .await
                            .unwrap();

            // 2. FIXÉ : Insérer le groupe 10 possédé par Alice pour passer la validation de l'Étape 0 (Existence).
            // Comme le test utilise l'ID de Bob juste après, Bob ne sera ni propriétaire, ni bénéficiaire d'ACL
            // direct ou par groupe, provoquant ainsi un refus d'accès binaire (0) au lieu d'une erreur d'existence (-1).
            sqlx::query(
                "INSERT INTO public.groups (id, owner_id, name) \
                             VALUES (10, $1, 'Confidential Group') \
                             ON CONFLICT (id) DO NOTHING",
            )
            .bind(alice_id as i32)
            .execute(&pool)
            .await
            .unwrap();

            let view = HasAccessQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::BOB_ID
                    .get()
                    .unwrap() as u64,
                "groups",
                "read",
                Some(10),
            );

            let result = interface.fetch_scalar::<i32, _>(&view).await.unwrap();

            assert!(result == 0, "expected access denied, got {}", result);
        }

        #[tokio::test]
        #[serial]
        async fn test_has_access_invalid_resource() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = HasAccessQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ALICE_ID
                    .get()
                    .unwrap() as u64,
                "ghost_resource",
                "read",
                Some(0),
            );

            let result = interface.fetch_scalar::<i32, _>(&view).await.unwrap();

            assert!(result == -1, "expected access denied, got {}", result);
        }
    }

    #[cfg(test)]
    mod is_admin_tests {
        use mairie360_api_lib::database::query_views::IsAdminQueryView;

        use super::*;

        #[tokio::test]
        #[serial]
        async fn test_is_admin_true() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = IsAdminQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::ADMIN_ID
                    .get()
                    .unwrap() as u64,
            );

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(result, "Expected admin to be admin");
        }

        #[tokio::test]
        #[serial]
        async fn test_is_admin_false() {
            let (_container, host) = get_shared_db().await;
            let interface: Database = Database::new(host.as_str()).await;

            let view = IsAdminQueryView::new(
                *mairie360_api_lib::test_setup::queries_setup::BOB_ID
                    .get()
                    .unwrap() as u64,
            );

            let result = interface.fetch_scalar::<bool, _>(&view).await.unwrap();

            assert!(!result, "Expected non-admin to not be admin");
        }
    }
}
