//! kabipay-performance — review cycles, goals, KPIs, feedback, ratings.
//! Federated async-graphql subgraph on port 4021.

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
            service_name: "kabipay-performance",
            default_port: 4021,
            port_env: "KABIPAY_PERFORMANCE_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
