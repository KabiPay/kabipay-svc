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

pub mod client_data_scope;
pub mod context;
pub mod db;
pub mod error;
pub mod ids;
pub mod jwt;
pub mod middleware;
pub mod pagination;
pub mod subgraph;
pub mod telemetry;

pub use context::{
    ClientContext, ClientViewerEmployee, OperatorContext, ScopeType, PERM_EMPLOYEE_MANAGE,
    PERM_EMPLOYEE_WRITE, SCOPE_RES_ATTENDANCE, SCOPE_RES_EMPLOYEE, SCOPE_RES_EXPENSE,
    SCOPE_RES_LEAVE,
};
pub use error::{KabiPayError, KabiPayResult};
pub use pagination::{PageInfo, PageInput};
