//! Root query resolvers for kabipay-operator (ops plane).

use async_graphql::{Context, Object, Result};
use kabipay_common::{subgraph::ops_db, KabiPayError};

use crate::resolvers::types::{OperatorRoleDto, OperatorUserDto};
use crate::services::operator_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn operator_health(&self) -> &'static str {
        "ok"
    }

    async fn operator_users(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<OperatorUserDto>> {
        let db = ops_db(ctx)?;
        let rows = operator_service::list_users(db, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(OperatorUserDto::from).collect())
    }

    async fn operator_roles(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<OperatorRoleDto>> {
        let db = ops_db(ctx)?;
        let rows = operator_service::list_roles(db, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(OperatorRoleDto::from).collect())
    }
}
