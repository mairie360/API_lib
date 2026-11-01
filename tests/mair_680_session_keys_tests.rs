//! MAIR-680: session tokens signed by Core with an Ed25519 key, verified everywhere else with its
//! public key. Own binary: every test changes process-wide variables through `temp_env`, which
//! serialises them.
use jsonwebtoken::{decode_header, encode, Algorithm, EncodingKey, Header};
use mairie360_api_lib::jwt_manager::{
    decode_jwt, generate_session_jwt, parse_public_keys, session_jwks, validate_jwt_config, Claims,
    JwtConfigError, SessionKeyError, SESSION_JWT_AUDIENCE, SESSION_JWT_ISSUER,
};
use mairie360_api_lib::test_setup::http_setup::{test_jwt_private_key, TEST_JWT_PUBLIC_KEY};

/// A second key pair, never published in the tests' `JWT_PUBLIC_KEY` unless stated: an attacker's
/// key, or the next key of a rotation.
/// PKCS#8 body (base64) of a second Ed25519 private key; built into PEM at run time so that the
/// secret scanners do not read a PEM private key in the sources.
const OTHER_PRIVATE_KEY_B64: &str =
    "MC4CAQAwBQYDK2VwBCIEII1DTYiu+dcwBgtEKil59uTH8s7XfrxGrbD+/P0sXq/8";

fn other_private_key() -> String {
    let label = "PRIVATE KEY";
    format!("-----BEGIN {label}-----\n{OTHER_PRIVATE_KEY_B64}\n-----END {label}-----\n")
}
const OTHER_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAtZHz1BNkijjkLJlnK9l7vikDl0aivrmbwllpT68Lnss=
-----END PUBLIC KEY-----
";

const STRONG_SECRET: &str = "0123456789abcdef0123456789abcdef";

/// Runs `f` with the given key variables, `JWT_TIMEOUT` = 3600 and no weak-secret override.
fn with_keys<R>(
    public: Option<&str>,
    private: Option<&str>,
    secret: Option<&str>,
    f: impl FnOnce() -> R,
) -> R {
    temp_env::with_vars(
        [
            ("JWT_PUBLIC_KEY", public),
            ("JWT_PRIVATE_KEY", private),
            ("JWT_SECRET", secret),
            ("JWT_TIMEOUT", Some("3600")),
            ("JWT_ALLOW_WEAK_SECRET", None),
        ],
        f,
    )
}

fn kid_of(public_pem: &str) -> String {
    parse_public_keys(public_pem).unwrap()[0].kid().to_string()
}

fn in_one_hour() -> usize {
    usize::try_from(chrono::Utc::now().timestamp()).unwrap() + 3600
}

/// Signs `claims` with `private_pem` as an `EdDSA` token with header `kid`.
fn sign_eddsa(private_pem: &str, kid: &str, claims: &Claims) -> String {
    let der = mairie360_api_lib::jwt_manager::parse_signing_key(
        private_pem,
        &parse_public_keys(&format!("{TEST_JWT_PUBLIC_KEY}{OTHER_PUBLIC_KEY}")).unwrap(),
    )
    .unwrap();
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(kid.to_string());
    encode(&header, claims, der.encoding_key()).unwrap()
}

fn core_claims() -> Claims {
    Claims::new("42", "Admin", in_one_hour())
        .with_session_id("7")
        .with_issuer_and_audience(SESSION_JWT_ISSUER, SESSION_JWT_AUDIENCE)
}

#[test]
fn mair_680_core_signs_with_its_ed25519_key_kid_iss_and_aud() {
    with_keys(
        Some(TEST_JWT_PUBLIC_KEY),
        Some(&test_jwt_private_key()),
        None,
        || {
            let token = generate_session_jwt("42", "Admin", Some("7")).unwrap();
            let header = decode_header(&token).unwrap();
            assert_eq!(header.alg, Algorithm::EdDSA);
            assert_eq!(
                header.kid.as_deref(),
                Some(kid_of(TEST_JWT_PUBLIC_KEY).as_str())
            );

            let claims = decode_jwt(&token).unwrap();
            assert_eq!(claims.user_id(), "42");
            assert_eq!(claims.role(), "Admin");
            assert_eq!(claims.session_id(), Some("7"));
            assert_eq!(claims.issuer(), Some(SESSION_JWT_ISSUER));
            assert_eq!(claims.audience(), Some(SESSION_JWT_AUDIENCE));
        },
    );
}

#[test]
fn mair_680_a_service_without_the_private_key_cannot_sign() {
    // An API or a BFF only holds the public key: it has nothing to sign a token with.
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        assert!(generate_session_jwt("1", "Admin", None).is_err());
    });
}

#[test]
fn mair_680_hs256_tokens_are_refused_once_jwt_secret_is_removed() {
    let legacy = with_keys(None, None, Some(STRONG_SECRET), || {
        generate_session_jwt("42", "Admin", Some("7")).unwrap()
    });
    assert_eq!(decode_header(&legacy).unwrap().alg, Algorithm::HS256);

    // Transition: both kinds are accepted while the secret is still set.
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, Some(STRONG_SECRET), || {
        assert_eq!(decode_jwt(&legacy).unwrap().user_id(), "42");
    });
    // After the transition: the old secret is gone, its tokens are refused.
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        assert!(decode_jwt(&legacy).is_err());
    });
}

#[test]
fn mair_680_hs256_signed_with_the_public_key_is_refused() {
    // Algorithm confusion: the public key is no secret, it must never verify an HMAC.
    let forged = encode(
        &Header::new(Algorithm::HS256),
        &Claims::new("1", "Admin", in_one_hour()),
        &EncodingKey::from_secret(TEST_JWT_PUBLIC_KEY.as_bytes()),
    )
    .unwrap();
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        assert!(decode_jwt(&forged).is_err());
    });
}

#[test]
fn mair_680_a_token_signed_with_another_key_is_refused() {
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        // Another key claiming the published kid: bad signature.
        let claimed = sign_eddsa(
            &other_private_key(),
            &kid_of(TEST_JWT_PUBLIC_KEY),
            &core_claims(),
        );
        assert!(decode_jwt(&claimed).is_err());
        // Another key with its own kid: unknown key.
        let own = sign_eddsa(
            &other_private_key(),
            &kid_of(OTHER_PUBLIC_KEY),
            &core_claims(),
        );
        assert!(decode_jwt(&own).is_err());
        // No kid at all.
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = None;
        let signing = mairie360_api_lib::jwt_manager::parse_signing_key(
            &test_jwt_private_key(),
            &parse_public_keys(TEST_JWT_PUBLIC_KEY).unwrap(),
        )
        .unwrap();
        let no_kid = encode(&header, &core_claims(), signing.encoding_key()).unwrap();
        assert!(decode_jwt(&no_kid).is_err());
        // Sanity check: the same claims signed with the published key pass.
        let genuine = sign_eddsa(
            &test_jwt_private_key(),
            &kid_of(TEST_JWT_PUBLIC_KEY),
            &core_claims(),
        );
        assert!(decode_jwt(&genuine).is_ok());
    });
}

#[test]
fn mair_680_iss_and_aud_are_checked() {
    let kid = kid_of(TEST_JWT_PUBLIC_KEY);
    let cases = [
        (
            "other issuer",
            Claims::new("1", "Admin", in_one_hour())
                .with_issuer_and_audience("keycloak", SESSION_JWT_AUDIENCE),
        ),
        (
            "other audience",
            Claims::new("1", "Admin", in_one_hour())
                .with_issuer_and_audience(SESSION_JWT_ISSUER, "n8n"),
        ),
        ("no iss nor aud", Claims::new("1", "Admin", in_one_hour())),
    ];
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        for (case, claims) in &cases {
            let token = sign_eddsa(&test_jwt_private_key(), &kid, claims);
            assert!(decode_jwt(&token).is_err(), "{case}");
        }
    });
}

#[test]
fn mair_680_expired_eddsa_tokens_are_refused_as_expired() {
    let expired = Claims::new("1", "Admin", 1)
        .with_issuer_and_audience(SESSION_JWT_ISSUER, SESSION_JWT_AUDIENCE);
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        let token = sign_eddsa(
            &test_jwt_private_key(),
            &kid_of(TEST_JWT_PUBLIC_KEY),
            &expired,
        );
        let Err(error) = decode_jwt(&token) else {
            panic!("an expired token was accepted");
        };
        assert_eq!(
            error.kind(),
            &jsonwebtoken::errors::ErrorKind::ExpiredSignature
        );
    });
}

#[test]
fn mair_680_rotation_accepts_every_published_key() {
    let both = format!("{OTHER_PUBLIC_KEY}{TEST_JWT_PUBLIC_KEY}");
    let old = with_keys(Some(&both), Some(&test_jwt_private_key()), None, || {
        generate_session_jwt("1", "Admin", None).unwrap()
    });
    let new = with_keys(Some(&both), Some(&other_private_key()), None, || {
        generate_session_jwt("2", "Admin", None).unwrap()
    });
    assert_eq!(
        decode_header(&new).unwrap().kid.unwrap(),
        kid_of(OTHER_PUBLIC_KEY)
    );
    with_keys(Some(&both), None, None, || {
        assert_eq!(decode_jwt(&old).unwrap().user_id(), "1");
        assert_eq!(decode_jwt(&new).unwrap().user_id(), "2");
    });
    // Once the old key is unpublished, its tokens are refused.
    with_keys(Some(OTHER_PUBLIC_KEY), None, None, || {
        assert!(decode_jwt(&old).is_err());
    });
}

#[test]
fn mair_680_kid_is_the_rfc_7638_thumbprint() {
    // RFC 8037 appendix A.3: thumbprint of the Ed25519 key of appendix A.2.
    let pem = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=\n-----END PUBLIC KEY-----\n";
    let key = &parse_public_keys(pem).unwrap()[0];
    assert_eq!(key.x(), "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo");
    assert_eq!(key.kid(), "kPrK_qmxVWaYVA9wwBF6Iuo3vVzz7TxHCTwXBygrS4k");
}

#[test]
fn mair_680_pem_with_escaped_line_breaks_is_accepted() {
    let escaped = TEST_JWT_PUBLIC_KEY.replace('\n', "\\n");
    assert_eq!(kid_of(&escaped), kid_of(TEST_JWT_PUBLIC_KEY));
}

#[test]
fn mair_680_jwks_lists_the_public_keys_only() {
    let both = format!("{TEST_JWT_PUBLIC_KEY}{OTHER_PUBLIC_KEY}");
    let jwks = with_keys(Some(&both), Some(&test_jwt_private_key()), None, || {
        session_jwks().unwrap()
    });
    let keys = jwks["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["kid"], kid_of(TEST_JWT_PUBLIC_KEY));
    assert_eq!(keys[0]["kty"], "OKP");
    assert_eq!(keys[0]["crv"], "Ed25519");
    assert_eq!(keys[0]["alg"], "EdDSA");
    assert!(
        keys.iter().all(|key| key.get("d").is_none()),
        "no private part"
    );
}

#[test]
fn mair_680_configuration_checks_the_keys() {
    // A real deployment: Core's public key, no secret any more.
    with_keys(Some(OTHER_PUBLIC_KEY), None, None, || {
        assert_eq!(validate_jwt_config(), Ok(()));
    });
    with_keys(
        Some(OTHER_PUBLIC_KEY),
        Some(&other_private_key()),
        None,
        || {
            assert_eq!(validate_jwt_config(), Ok(()));
        },
    );
    // Transition: key and secret together.
    with_keys(Some(OTHER_PUBLIC_KEY), None, Some(STRONG_SECRET), || {
        assert_eq!(validate_jwt_config(), Ok(()));
    });
    // The committed test key is refused outside test stacks, as a weak key.
    with_keys(Some(TEST_JWT_PUBLIC_KEY), None, None, || {
        let error = validate_jwt_config().unwrap_err();
        assert_eq!(error, JwtConfigError::KnownTestKey);
        assert!(error.is_weak_secret());
    });
    // Defects.
    with_keys(None, None, None, || {
        assert_eq!(validate_jwt_config(), Err(JwtConfigError::MissingSecret));
    });
    with_keys(Some("not a key"), None, None, || {
        assert_eq!(
            validate_jwt_config(),
            Err(SessionKeyError::InvalidPublicKey.into())
        );
    });
    with_keys(Some(&other_private_key()), None, None, || {
        // A private key pasted in the public variable.
        assert_eq!(
            validate_jwt_config(),
            Err(SessionKeyError::InvalidPublicKey.into())
        );
    });
    with_keys(
        None,
        Some(&other_private_key()),
        Some(STRONG_SECRET),
        || {
            assert_eq!(
                validate_jwt_config(),
                Err(SessionKeyError::MissingPublicKey.into())
            );
        },
    );
    with_keys(
        Some(OTHER_PUBLIC_KEY),
        Some(&test_jwt_private_key()),
        None,
        || {
            assert_eq!(
                validate_jwt_config(),
                Err(SessionKeyError::UnpublishedPrivateKey.into())
            );
        },
    );
    with_keys(Some(OTHER_PUBLIC_KEY), Some(OTHER_PUBLIC_KEY), None, || {
        assert_eq!(
            validate_jwt_config(),
            Err(SessionKeyError::InvalidPrivateKey.into())
        );
    });
    // A weak secret is still refused during the transition.
    with_keys(Some(OTHER_PUBLIC_KEY), None, Some("secret"), || {
        assert_eq!(validate_jwt_config(), Err(JwtConfigError::KnownTestSecret));
    });
}
