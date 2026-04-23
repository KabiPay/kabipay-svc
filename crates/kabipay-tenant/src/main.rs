//! kabipay-tenant — ops-plane tenant catalogue, module catalog, subscriptions.
//! Federated async-graphql subgraph on port 4011.

use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use kabipay_common::subgraph::{serve_subgraph, SubgraphConfig};

mod resolvers;
mod services;

use resolvers::QueryRoot;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription);
    serve_subgraph(
        SubgraphConfig {
            service_name: "kabipay-tenant",
            default_port: 4011,
            port_env: "KABIPAY_TENANT_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
