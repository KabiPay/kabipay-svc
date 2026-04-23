//! kabipay-employee
//!
//! Employee core: profiles, org hierarchy, documents. Canonical source for
//! `EMPLOYEE.id` (Gap A).
//!
//! Exposes a federated async-graphql subgraph on port 4013
//! (override via `KABIPAY_EMPLOYEE_PORT`). All request wiring —
//! tenant-header extraction, ops DB, `/graphql` + playground + `/healthz`,
//! CORS and tracing — lives in `kabipay_common::subgraph::serve_subgraph`.

use async_graphql::EmptySubscription;
use async_graphql::Schema;
use kabipay_common::subgraph::{serve_subgraph, SubgraphConfig};

mod entities;
mod resolvers;
mod services;

use resolvers::{MutationRoot, QueryRoot};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription);
    serve_subgraph(
        SubgraphConfig {
            service_name: "kabipay-employee",
            default_port: 4013,
            port_env: "KABIPAY_EMPLOYEE_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
