//! kabipay-ops — unified ops-plane GraphQL subgraph (tenants, operators, billing).
//! Single process on `KABIPAY_OPS_PORT` (default 4010).

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
            service_name: "kabipay-ops",
            default_port: 4010,
            port_env: "KABIPAY_OPS_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
