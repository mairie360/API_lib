use actix_web::HttpRequest;

/**
 * Extracts a JWT from the Authorization header of an HTTP request.
 *
 * # Arguments
 * * `req` - A reference to the HttpRequest from which to extract the JWT.
 *
 * # Returns
 * An `Option<String>` containing the JWT if it exists and is valid, or `None` if it does not.
 */
pub fn get_jwt_from_request(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|auth_str| {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        })
}
