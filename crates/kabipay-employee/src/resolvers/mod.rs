//! GraphQL resolvers for kabipay-employee.
//!
//! Resolvers are the only place that imports `async_graphql`. Business logic lives
//! in `crate::services::*`. This keeps services unit-testable without a GraphQL ctx.

pub mod mutation;
pub mod query;
pub mod scope;
pub mod types;

pub use mutation::MutationRoot;
pub use query::QueryRoot;
