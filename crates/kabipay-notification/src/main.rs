//! kabipay-notification — in-app notifications and company announcements.
//! Federated async-graphql subgraph on port 4028.

use async_graphql::EmptySubscription;
use async_graphql::Schema;
use kabipay_common::subgraph::{serve_subgraph, SubgraphConfig};

mod resolvers;
mod services;

use resolvers::{MutationRoot, QueryRoot};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription);
    serve_subgraph(
        SubgraphConfig {
            service_name: "kabipay-notification",
            default_port: 4028,
            port_env: "KABIPAY_NOTIFICATION_PORT",
            needs_db: true,
        },
        schema,
    )
    .await
}
