mod db;
mod error;
mod handlers;
mod models;
mod scorecard;

use axum::{
    Router,
    routing::get,
};
use sqlx::mysql::MySqlPoolOptions;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Shared application state threaded through every handler via Axum's
/// `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::MySqlPool,
}

#[tokio::main]
async fn main() {
    // Load .env if present (DATABASE_URL, HOST, PORT, etc.)
    dotenvy::dotenv().ok();

    // Tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "qams_server=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = MySqlPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let state = AppState { db: pool };

    let app = router(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr: SocketAddr = format!("{host}:{port}").parse().expect("Invalid address");

    tracing::info!("Listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub fn router(state: AppState) -> Router {
    Router::new()
        // ── Scorecards ────────────────────────────────────────────────────
        .route("/scorecards",              get(handlers::scorecards::list)
                                          .post(handlers::scorecards::create))
        .route("/scorecards/{id}",         get(handlers::scorecards::show)
                                          .delete(handlers::scorecards::delete))

        // ── Agents ────────────────────────────────────────────────────────
        .route("/agents",                  get(handlers::agents::list)
                                          .post(handlers::agents::create))
        .route("/agents/{id}",             get(handlers::agents::show)
                                          .delete(handlers::agents::delete))

        // ── Reviews ───────────────────────────────────────────────────────
        // GET /reviews renders a blank review form for a given scorecard.
        // POST /reviews submits a completed review.
        .route("/reviews",                 get(handlers::reviews::form)
                                          .post(handlers::reviews::submit))
        .route("/reviews/{id}",            get(handlers::reviews::show))

        // ── Reports ───────────────────────────────────────────────────────
        .route("/reports",                 get(handlers::reports::list)
                                          .post(handlers::reports::generate))
        .route("/reports/{id}",            get(handlers::reports::show))
        .route("/reports/{id}/summary",    get(handlers::reports::summary))
        .route("/reports/{id}/agents",     get(handlers::reports::agent_index))
        .route("/reports/{id}/agents/{agent_id}", get(handlers::reports::agent_page))

        // ── State + middleware ────────────────────────────────────────────
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}