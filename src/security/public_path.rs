/// Path prefix of the unauthenticated authentication routes (login, register, password reset...).
const AUTH_ROUTES_PREFIX: &str = "/api/v1/auth/";

/// Documentation roots served without authentication (Swagger UI and the `OpenAPI` documents).
const PUBLIC_DOC_ROOTS: [&str; 2] = ["/swagger-ui", "/api-docs"];

/// Returns `true` when `path` may be served without a token by [`JwtMiddleware`](super::JwtMiddleware).
///
/// The allow-list is explicit, every other path requires authentication:
/// - exactly `/`;
/// - `/swagger-ui` and `/api-docs`, and anything below them (`/swagger-ui/...`, `/api-docs/...`);
/// - anything below `/api/v1/auth/` (e.g. `/api/v1/auth/login`), as long as it has no `.` or
///   `..` segment, so a dot segment cannot walk out of the prefix.
///
/// Matching is done on whole path segments: `/api/v1/authx`, `/api/v1/projects/author` or
/// `/api/v1/admin/auth-logs` are **not** public.
#[must_use]
pub fn is_public_path(path: &str) -> bool {
    if path == "/" {
        return true;
    }
    if PUBLIC_DOC_ROOTS
        .iter()
        .any(|root| is_same_or_below(path, root))
    {
        return true;
    }
    path.strip_prefix(AUTH_ROUTES_PREFIX)
        .is_some_and(|rest| !rest.split('/').any(is_dot_segment))
}

/// `path` is `root` itself or a path below it (`root/...`).
fn is_same_or_below(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// A `.` or `..` segment, plain or percent-encoded (`%2e`, `%2E`).
fn is_dot_segment(segment: &str) -> bool {
    let decoded = segment.to_ascii_lowercase().replace("%2e", ".");
    decoded == "." || decoded == ".."
}

#[cfg(test)]
mod tests {
    use super::is_public_path;

    #[test]
    fn public_paths_bypass_authentication() {
        for path in [
            "/",
            "/swagger-ui",
            "/swagger-ui/",
            "/swagger-ui/index.html",
            "/api-docs",
            "/api-docs/openapi.json",
            "/api/v1/auth/login",
            "/api/v1/auth/register",
            "/api/v1/auth/forgot_password",
            "/api/v1/auth/reset_password",
            "/api/v1/auth/force_change_password",
            "/api/v1/auth/keycloak",
        ] {
            assert!(is_public_path(path), "{path} should be public");
        }
    }

    #[test]
    fn other_paths_require_authentication() {
        for path in [
            "",
            "/api",
            "/api/v1",
            "/api/v1/auth",
            "/api/v1/authx",
            "/api/v1/authx/login",
            "/api/v1/projects/author",
            "/api/v1/admin/auth-logs",
            "/api/v1/admin/auth/login",
            "/api/v1/users/1/auth/login",
            "/api/v2/auth/login",
            "/auth/login",
            "/API/V1/AUTH/login",
            "/swagger-uix",
            "/api-docs-private",
            "/api/v1/projects",
            "/api/v1/user/me",
            "//api/v1/auth/login",
        ] {
            assert!(
                !is_public_path(path),
                "{path} should require authentication"
            );
        }
    }

    #[test]
    fn dot_segments_cannot_escape_the_auth_prefix() {
        for path in [
            "/api/v1/auth/../admin/users",
            "/api/v1/auth/./login",
            "/api/v1/auth/%2e%2e/admin/users",
            "/api/v1/auth/%2E%2E/admin/users",
            "/api/v1/auth/login/..",
        ] {
            assert!(
                !is_public_path(path),
                "{path} should require authentication"
            );
        }
    }
}
