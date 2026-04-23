//! kabipay-billing — ops-plane billing: invoices, payments, dunning.
//! Federated async-graphql subgraph on port 4012.

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
            service_name: "kabipay-billing",
            default_port: 4012,
            port_env: "KABIPAY_BILLING_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
