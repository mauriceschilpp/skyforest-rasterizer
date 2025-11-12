use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower::ServiceBuilder;
use axum::extract::DefaultBodyLimit;

use super::handlers::*;

pub fn create_router() -> Router {
    Router::new()
        .route("/api/coordinate", get(get_coordinate_value))
        .route("/api/upload", post(upload_csv))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB limit
                .layer(CorsLayer::permissive())
        )
}
