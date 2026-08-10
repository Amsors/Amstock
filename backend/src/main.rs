mod error;
mod models;
mod routes;
mod validation;

use axum::{
    Router,
    http::{HeaderValue, Method},
    routing::get,
};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{env, path::PathBuf};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    image_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow_main::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "amstock_backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        env::var("AMSTOCK_DATABASE_URL").unwrap_or_else(|_| "sqlite://data/amstock.db".into());
    let image_dir =
        PathBuf::from(env::var("AMSTOCK_IMAGE_DIR").unwrap_or_else(|_| "data/images".into()));
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    tokio::fs::create_dir_all(&image_dir).await?;
    let options: SqliteConnectOptions = database_url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::raw_sql(include_str!("schema.sql"))
        .execute(&pool)
        .await?;

    let state = AppState { pool, image_dir };
    let api = routes::router();
    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .nest("/api", api)
        .route("/images/{serial}", get(routes::get_image))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(HeaderValue::from_static("http://localhost:5173"))
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state);
    let bind = env::var("AMSTOCK_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "Amstock backend listening");
    axum::serve(listener, app).await?;
    Ok(())
}

mod anyhow_main {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
