use std::sync::Arc;

use axum::routing::{get, patch, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;
use crate::db::Db;
use crate::handlers;

pub fn build_app(config: &AppConfig, db: Arc<Db>) -> Router {
    let api = Router::new()
        .route(
            "/skills",
            get(handlers::list_skills).post(handlers::create_skill),
        )
        .route(
            "/skills/:id",
            patch(handlers::update_skill).delete(handlers::delete_skill),
        )
        .route("/skills/:id/observe", post(handlers::observe_skill))
        .route("/skills/:id/history", get(handlers::skill_history))
        .with_state(db);

    let index_path = config.frontend_dir.join("index.html");
    let frontend_assets = ServeDir::new(&config.frontend_dir).fallback(ServeFile::new(index_path));

    Router::new()
        .nest("/api", api)
        .fallback_service(frontend_assets)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
}
