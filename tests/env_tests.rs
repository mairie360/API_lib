use mairie360_api_lib::env_manager::{get_critical_env_var, get_env_var};
use once_cell::sync::Lazy;
use std::env;

/**
 * This module provides tests for environment variable handling.
 * It includes tests for getting environment variables and ensuring
 * critical environment variables are set.
 */
static INIT: Lazy<()> = Lazy::new(|| {
    // This code runs ONCE before any test
    unsafe { env::set_var("MY_KEY", "global_value") };
    println!("Global setup done");
});

/**
 * This function sets up the environment for tests.
 * It initializes the environment variables needed for the tests.
 */
fn setup() {
    // Force INIT to run
    Lazy::force(&INIT);
}

/**
 * This module contains tests for environment variable handling.
 * It includes tests for getting environment variables and ensuring
 * critical environment variables are set.
 */
#[cfg(test)]
mod env_tests {
    use super::*;

    /**
     * This function sets up the environment for tests.
     * It initializes the environment variables needed for the tests.
     */
    #[test]
    fn test_get_env_var() {
        setup();
        assert_eq!(get_env_var("MY_KEY").unwrap(), "global_value");
    }

    /**
     * This function tests the behavior of getting an environment variable
     * that does not exist. It should return None.
     */
    #[test]
    fn test_get_env_var_not_found() {
        setup();
        assert!(get_env_var("NON_EXISTENT_KEY").is_none());
    }

    /**
     * This function tests the behavior of getting a critical environment variable.
     * It should return the value if it exists, or panic if it does not.
     */
    #[test]
    fn test_get_critical_env_var() {
        setup();
        assert_eq!(get_critical_env_var("MY_KEY"), "global_value");
    }

    /**
     * This function tests the behavior of getting a critical environment variable
     * that does not exist. It should panic with a specific message.
     */
    #[test]
    #[should_panic(expected = "Critical environment variable 'NON_EXISTENT_KEY' is not set")]
    fn test_get_critical_env_var_not_found() {
        setup();
        get_critical_env_var("NON_EXISTENT_KEY");
    }
}
