// Authentication middleware for the Axum application
use axum::http::StatusCode;
use axum::{http::Request, middleware::Next, response::Response};
use crate::db;

pub async fn auth_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get the Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    // If the header is missing or invalid, return an error
    let auth_header = match auth_header {
        Some(header) => header,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // Extract the token from the header
    let token = auth_header.trim_start_matches("Bearer ");

    // Create a database pool
    let pool = db::init("sqlite://./llamp.db").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get the user by proxy key
    let user = db::get_user_by_proxy_key(&pool, token).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If the user is not found or is disabled, return an error
    let user = match user {
        Some(user) if user.enabled => user,
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // Add user information to the request extensions
    let (mut parts, body) = request.into_parts();
    parts.extensions.insert(user);
    let request = Request::from_parts(parts, body);

    // Continue with the request
    let response = next.run(request).await;
    Ok(response)
}
