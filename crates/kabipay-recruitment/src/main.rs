//! kabipay-recruitment — job postings, applications, interviews, offers.
//! Federated async-graphql subgraph on port 4020.

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
            service_name: "kabipay-recruitment",
            default_port: 4020,
            port_env: "KABIPAY_RECRUITMENT_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
