//! kabipay-common
//!
//! Shared primitives used by every KabiPay microservice:
//!   - Canonical error type ([`error::KabiPayError`])
//!   - Request contexts ([`context::OperatorContext`], [`context::ClientContext`])
//!   - Tenant database resolver ([`db::resolve_tenant_db`])
//!   - Axum middleware (auth for both planes, module-subscription guard)
//!   - Pagination helpers
//!   - Structured logging bootstrap
//!
//! Every service depends on this crate via `kabipay-common = { workspace = true }`.

pub mod error;
pub mod context;
pub mod db;
pub mod pagination;
pub mod middleware;
pub mod jwt;
pub mod telemetry;
pub mod ids;
pub mod subgraph;

pub use error::{KabiPayError, KabiPayResult};
pub use context::{ClientContext, OperatorContext, ScopeType};
pub use pagination::{PageInfo, PageInput};
