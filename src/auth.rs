// Authentication middleware for the Axum application
use axum::http::StatusCode;
use axum::{http::Request, middleware::Next, response::Response};

pub async fn auth_middleware(
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // For now, we'll just pass through all requests
    // In a real implementation, we would check for API keys, etc.
    let response = next.run(request).await;
    Ok(response)
}
