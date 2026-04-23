//! Shared application state passed into every auth handler.

use crate::jwt::JwtConfig;
use kabipay_common::db::{TenantDbCache, TenantDbConfig};
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub ops_db: DatabaseConnection,
    pub tenant_cache: TenantDbCache,
    pub tenant_fallback: TenantDbConfig,
    pub jwt: JwtConfig,
}
