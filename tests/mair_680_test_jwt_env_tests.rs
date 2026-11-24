//! MAIR-680: a harness that presets only `JWT_PUBLIC_KEY` to the committed test key (as the API
//! test stacks do) still gets the matching private key from `init_test_jwt_env`, so
//! `bearer_token` can sign. Own binary: it changes the process environment before anything
//! reads it.
#![cfg(feature = "test-utils")]

use mairie360_api_lib::test_setup::http_setup::{
    bearer_token, init_test_jwt_env, TEST_JWT_PUBLIC_KEY,
};

#[test]
fn mair_680_a_preset_test_public_key_still_gets_the_test_private_key() {
    std::env::set_var("JWT_PUBLIC_KEY", TEST_JWT_PUBLIC_KEY);
    std::env::remove_var("JWT_PRIVATE_KEY");

    init_test_jwt_env();

    assert!(
        std::env::var_os("JWT_PRIVATE_KEY").is_some(),
        "the test private key must be set next to the preset test public key"
    );
    assert_eq!(
        bearer_token(1).split('.').count(),
        3,
        "a signed compact JWT"
    );
}
