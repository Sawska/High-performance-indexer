//! HTTP API for the web UI, plus the built UI itself.
//!
//!   /api/auth/*         register, login, logout, me          -- public
//!   /api/status         live scrape progress + table counts  -- signed in
//!   /api/overview, /api/markets[/{id}], /api/traders[/{wallet}], /api/trades,
//!   /api/activity       Polymarket statistics                -- signed in
//!   everything else     files from `web_dir`, `index.html` as the SPA fallback
//!
//! Every handler reads Postgres directly; the scraper is the only writer.

mod auth;
mod polymarket;
#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::path::Path;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use tower_http::services::{ServeDir, ServeFile};

use crate::stats;

/// An error a handler returns. Server-side causes are logged and hidden from
/// the client; client-side ones carry their message back as `{"error": ...}`.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        eprintln!("api: {e}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self::internal(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Serialize, sqlx::FromRow)]
struct TableCount {
    name: String,
    rows: i64,
}

#[derive(Serialize)]
struct Status {
    now: DateTime<Utc>,
    live: stats::Snapshot,
    tables: Vec<TableCount>,
}

async fn status(State(pool): State<PgPool>) -> ApiResult<Status> {
    // The stats view is an estimate that lags commits by a moment, but it is
    // free; exact count(*) over the history tables would not be.
    let tables = sqlx::query_as::<_, TableCount>(
        "SELECT relname::text AS name, n_live_tup AS rows
            FROM pg_stat_user_tables
        ORDER BY relname",
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(Status {
        now: Utc::now(),
        live: stats::snapshot(),
        tables,
    }))
}

/// Every route, API and UI, over one pool.
pub fn router(pool: PgPool, web_dir: &Path) -> Router {
    let signed_in = Router::new()
        .route("/api/status", get(status))
        .route("/api/overview", get(polymarket::overview))
        .route("/api/markets", get(polymarket::markets))
        .route("/api/markets/{id}", get(polymarket::market))
        .route("/api/traders", get(polymarket::traders))
        .route("/api/traders/{wallet}", get(polymarket::trader))
        .route("/api/trades", get(polymarket::trades))
        .route("/api/activity", get(polymarket::activity))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            auth::require_user,
        ));

    let public = Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me));

    // An unknown /api path must answer 404, not the SPA's index.html.
    let api_404 = Router::new().route(
        "/api/{*rest}",
        get(|| async { ApiError::new(StatusCode::NOT_FOUND, "no such endpoint") }),
    );

    let ui = ServeDir::new(web_dir).fallback(ServeFile::new(web_dir.join("index.html")));

    signed_in
        .merge(public)
        .merge(api_404)
        .with_state(pool)
        .fallback_service(ui)
}

pub async fn serve(pool: PgPool, addr: SocketAddr, web_dir: &Path) -> std::io::Result<()> {
    let app = router(pool, web_dir);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    if !web_dir.join("index.html").exists() {
        eprintln!(
            "api: no UI build at {}; run `npm run build` in web/",
            web_dir.display()
        );
    }
    println!("api: listening on http://{addr}");
    axum::serve(listener, app).await
}
