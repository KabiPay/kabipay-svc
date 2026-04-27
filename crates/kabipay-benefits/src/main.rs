//! kabipay-benefits — benefit types, plans, enrolments, claims.
//! Federated async-graphql subgraph on port 4018.

use async_graphql::{EmptySubscription, Schema};
use kabipay_common::subgraph::{serve_subgraph, SubgraphConfig};

mod resolvers;
mod services;

use resolvers::{MutationRoot, QueryRoot};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription);
    serve_subgraph(
        SubgraphConfig {
            service_name: "kabipay-benefits",
            default_port: 4018,
            port_env: "KABIPAY_BENEFITS_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
