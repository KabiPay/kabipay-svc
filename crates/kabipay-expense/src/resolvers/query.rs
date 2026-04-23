//! Root query resolvers for kabipay-expense.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{ExpenseCategoryDto, ExpenseDto};
use crate::services::expense_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn expense_health(&self) -> &'static str {
        "ok"
    }

    async fn expense_categories(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<ExpenseCategoryDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = expense_service::list_categories(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ExpenseCategoryDto::from).collect())
    }

    async fn expenses(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<ExpenseDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = expense_service::list_expenses(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ExpenseDto::from).collect())
    }
}
