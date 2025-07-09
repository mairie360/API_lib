// use core_api::jwt_manager::generate_jwt::generate_jwt;
// use core_api::jwt_manager::get_jwt_secret::get_jwt_secret;
// use core_api::jwt_manager::get_user_id_from_jwt::get_user_id_from_jwt;
// use once_cell::sync::Lazy;
// use std::env;

// /**
//  * This module contains tests for the JWT manager.
//  * It tests the functionality of generating JWTs, retrieving user IDs from JWTs,
//  * and getting the JWT secret and timeout.
//  */
// static USER_ID: &str = "1";

// /**
//  * Global setup for tests.
//  */
// static INIT: Lazy<()> = Lazy::new(|| {
//     // This code runs ONCE before any test
//     env::set_var("JWT_SECRET", "b\"secret\"");
//     env::set_var("JWT_TIMEOUT", "3600");
//     println!("Global setup done");
// });

// /**
//  * Sets up the environment for tests.
//  * This function is called before each test to ensure the environment variables are set.
//  */
// fn setup() {
//     // Force INIT to run
//     Lazy::force(&INIT);
// }

// /**
//  * Tests for the JWT manager.
//  * These tests cover the generation of JWTs, retrieval of user IDs from JWTs,
//  * and the retrieval of JWT secrets and timeouts.
//  */
// #[cfg(test)]
// mod jwt_tests {
//     use super::*;

//     /**
//      * Tests the retrieval of the JWT secret.
//      * It checks if the secret can be retrieved successfully and matches the expected value.
//      * It also ensures that the secret is in the expected byte format.
//      */
//     #[test]
//     fn test_get_jwt_secret() {
//         setup();
//         let secret = get_jwt_secret();
//         println!("secret: {:?}", secret);
//         assert!(
//             secret.is_ok(),
//             "Failed to get JWT secret: {:?}",
//             secret.err()
//         );
//         assert_eq!(
//             secret.unwrap(),
//             "b\"secret\"".to_string().into_bytes(),
//             "JWT secret does not match expected value"
//         );
//     }

//     /**
//      * Tests the retrieval of the JWT timeout.
//      * It checks if the timeout can be retrieved successfully and matches the expected value.
//      * The expected timeout is set to 3600 seconds (1 hour).
//      */
//     #[test]
//     fn test_get_jwt_timeout() {
//         setup();
//         let timeout = core_api::jwt_manager::get_jwt_timeout::get_jwt_timeout();
//         assert!(
//             timeout.is_ok(),
//             "Failed to get JWT timeout: {:?}",
//             timeout.err()
//         );
//         assert_eq!(
//             timeout.unwrap(),
//             3600,
//             "JWT timeout does not match expected value"
//         );
//     }

//     /**
//      * Tests the generation of a JWT.
//      * It checks if the JWT can be generated successfully for a given user ID.
//      * The generated token should not be empty.
//      * If the generation fails, it asserts with an error message.
//      */
//     #[test]
//     fn test_generate_jwt() {
//         setup();
//         let token = generate_jwt(USER_ID);
//         assert!(token.is_ok(), "JWT generation failed: {:?}", token.err());
//         let token = token.unwrap();
//         assert!(!token.is_empty(), "Generated JWT token is empty");
//     }

//     /**
//      * Tests the retrieval of a user ID from a JWT.
//      * It checks if the user ID can be extracted from a valid JWT.
//      * The user ID should match the expected value.
//      * It also tests the case where an invalid JWT is provided, expecting None as the result
//      */
//     #[test]
//     fn test_get_user_id_from_jwt() {
//         setup();
//         let token = generate_jwt(USER_ID).unwrap();
//         let user_id = get_user_id_from_jwt(&token);
//         assert_eq!(
//             user_id.unwrap(),
//             USER_ID,
//             "User ID does not match expected value"
//         );
//     }

//     /**
//      * Tests the retrieval of a user ID from an invalid JWT.
//      * It checks if the function returns None when an invalid JWT is provided.
//      * This ensures that the function handles invalid tokens gracefully.
//      * If the function returns Some, it asserts with an error message.
//      */
//     #[test]
//     fn test_get_user_id_from_invalid_jwt() {
//         setup();
//         let invalid_token = "invalid.token.string";
//         let user_id = get_user_id_from_jwt(invalid_token);
//         assert_eq!(user_id, None, "Expected None for invalid JWT, got Some");
//     }
// }
