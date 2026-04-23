//! Write operations for expense claims.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{ExpenseDto, SubmitExpenseInput};
use crate::services::expense_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a PENDING expense claim for the signed-in user’s employee record.
    async fn submit_expense(
        &self,
        ctx: &Context<'_>,
        input: SubmitExpenseInput,
    ) -> Result<ExpenseDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let category_id = parse_uuid(&input.expense_category_id, "expenseCategoryId")?;
        let amount = expense_service::parse_amount(&input.amount).map_err(KabiPayError::into_graphql)?;
        let m = expense_service::submit_expense(
            &db,
            tenant_id,
            employee_id,
            category_id,
            amount,
            &input.currency,
            input.expense_date,
            &input.title,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(ExpenseDto::from(m))
    }
}
