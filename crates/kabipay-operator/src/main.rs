//! kabipay-operator — ops-plane operator users, roles, RBAC.
//! Federated async-graphql subgraph on port 4010.

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
            service_name: "kabipay-operator",
            default_port: 4010,
            port_env: "KABIPAY_OPERATOR_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
