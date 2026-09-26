/// Identity asserted by a verified Keycloak access token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeycloakIdentity {
    /// Keycloak user id (`sub` claim), stable across logins but unrelated to the `users.id`
    /// column: accounts are matched on the e-mail.
    pub subject: String,
    /// E-mail address verified by Keycloak, trimmed, used to find the matching Mairie 360 account.
    pub email: String,
    /// Realm roles carried by the token (`realm_access.roles`), empty when the token has none.
    /// Informative only: permissions are still resolved from the roles stored in the database.
    pub roles: Vec<String>,
}
