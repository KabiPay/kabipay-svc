//! kabipay-grievance — POSH-aware grievance cases, participants, actions.
//! Federated async-graphql subgraph on port 4026.

use async_graphql::{EmptySubscription, Schema};
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
            service_name: "kabipay-grievance",
            default_port: 4026,
            port_env: "KABIPAY_GRIEVANCE_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
