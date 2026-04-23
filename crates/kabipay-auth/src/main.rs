//! kabipay-auth
//!
//! REST auth service for both planes. Exposes (plain HTTP, no JWT required):
//!
//!   POST /auth/ops/login      { email, password }          → TokenPair
//!   POST /auth/ops/mfa        { mfaToken, code }           → 501 (scaffolded)
//!   POST /auth/ops/refresh    { refresh }                  → TokenPair (rotated)
//!   POST /auth/ops/logout     { refresh }                  → 204
//!
//!   POST /auth/client/login   { email, password, tenantId }→ TokenPair
//!   POST /auth/client/mfa     { mfaToken, code }           → 501 (scaffolded)
//!   POST /auth/client/refresh { refresh }                  → TokenPair (rotated)
//!   POST /auth/client/logout  { refresh }                  → 204
//!
//!   POST /auth/introspect     { token }                    → { active, userId, tenantId, issuer, email, exp }
//!
//!   GET  /healthz                                          → 200 "ok"

use axum::{
    routing::{get, post},
    Router,
};
use kabipay_common::{
    db::{connect_ops_db, TenantDbCache},
    subgraph::{ops_dsn_from_env, tenant_db_config_from_env},
    telemetry::init_tracing,
};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod handlers;
mod jwt;
mod password;
mod state;
mod tokens;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("kabipay-auth");

    let port: u16 = std::env::var("KABIPAY_AUTH_PORT")
        .unwrap_or_else(|_| "4001".to_string())
        .parse()
        .unwrap_or(4001);

    let dsn = ops_dsn_from_env();
    let ops_db = connect_ops_db(&dsn)
        .await
        .map_err(|e| anyhow::anyhow!("kabipay-auth: failed to connect to ops DB at {dsn}: {e}"))?;

    let app_state = AppState {
        ops_db,
        tenant_cache: TenantDbCache::new(),
        tenant_fallback: tenant_db_config_from_env(),
        jwt: handlers::jwt_config_from_env(),
    };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        // Operator plane
        .route("/auth/ops/login", post(handlers::ops_login))
        .route("/auth/ops/mfa", post(handlers::ops_mfa))
        .route("/auth/ops/refresh", post(handlers::ops_refresh))
        .route("/auth/ops/logout", post(handlers::ops_logout))
        // Client plane
        .route("/auth/client/login", post(handlers::client_login))
        .route("/auth/client/mfa", post(handlers::client_mfa))
        .route("/auth/client/refresh", post(handlers::client_refresh))
        .route("/auth/client/logout", post(handlers::client_logout))
        // Token introspection (used by gateway + subgraph auth middleware)
        .route("/auth/introspect", post(handlers::introspect))
        .with_state(app_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "kabipay-auth listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Fallback 501 response used by stub handlers until they are implemented.
pub fn not_implemented(
    endpoint: &'static str,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error": {
                "code": "NOT_IMPLEMENTED",
                "message": format!("{endpoint} is scaffolded but not yet implemented"),
            }
        })),
    )
}
