use actix_web::HttpRequest;

pub fn get_jwt_from_request(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "))
        .map(str::to_string)
}
