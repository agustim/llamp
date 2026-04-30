// Authentication middleware for the Axum application
use axum::http::StatusCode;
use axum::{http::Request, middleware::Next, response::Response};
use crate::db;

pub async fn auth_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Log authentication attempt for debugging
    let auth_header = request.headers().get("Authorization");
    
    if tracing::enabled!(tracing::Level::DEBUG) {
        if let Some(header) = auth_header {
            tracing::debug!("Authorization header present, attempting authentication");
        } else {
            tracing::debug!("No Authorization header found, returning 401");
        }
    }

    // If the header is missing or invalid, return an error
    let auth_header = match auth_header.and_then(|h| h.to_str().ok()) {
        Some(h) => h,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // Extract the token from the header
    let token = auth_header.trim_start_matches("Bearer ");

    // Log token info in debug mode (not the full token for security)
    if tracing::enabled!(tracing::Level::DEBUG) {
        let token_preview = if token.len() > 20 {
            format!("{}...{}", &token[..8], &token[token.len()-8..])
        } else {
            token.to_string()
        };
        tracing::debug!(token = token_preview, "Attempting to find user by proxy key");
    }

    // Create a database pool
    let pool = db::init("sqlite://./llamp.db").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get the user by proxy key
    let user = db::get_user_by_proxy_key(&pool, token).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Log authentication result
    match &user {
        Some(u) => {
            if u.enabled {
                tracing::info!(user_id = u.id, username = u.username, "Authentication successful");
            } else {
                tracing::warn!(user_id = u.id, username = u.username, "User is disabled");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => {
            tracing::warn!("User not found for provided token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Add user information to the request extensions
    let (mut parts, body) = request.into_parts();
    parts.extensions.insert(user.unwrap());
    let request = Request::from_parts(parts, body);

    // Continue with the request
    let response = next.run(request).await;
    Ok(response)
}
