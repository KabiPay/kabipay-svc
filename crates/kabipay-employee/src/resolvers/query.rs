//! Root query resolvers for kabipay-employee.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{DocumentTypeDto, EmployeeDocumentDto, EmployeeDto};
use crate::services::{document_service, employee_service};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Liveness probe for this federated subgraph. Always returns `ok`.
    async fn employee_health(&self) -> &'static str {
        "ok"
    }

    /// Fetch one employee by UUID inside the caller's tenant.
    ///
    /// Returns `null` if the employee does not exist, is soft-deleted, or
    /// belongs to another tenant (never leaks cross-tenant rows).
    async fn employee(&self, ctx: &Context<'_>, id: ID) -> Result<Option<EmployeeDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let employee_id = parse_uuid(&id, "employee id")?;
        let db = tenant_db(ctx, tenant_id).await?;
        let model = employee_service::find_by_id(&db, tenant_id, employee_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(model.map(EmployeeDto::from))
    }

    /// List the first `limit` employees in the caller's tenant (capped at 100).
    async fn employees(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: u64,
    ) -> Result<Vec<EmployeeDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let models = employee_service::list(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(models.into_iter().map(EmployeeDto::from).collect())
    }

    /// Master list of document / policy types defined for the tenant.
    async fn document_types(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<DocumentTypeDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = document_service::list_document_types(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(DocumentTypeDto::from).collect())
    }

    /// Uploaded employee documents. Omit `employeeId` to list the caller’s own files (JWT).
    async fn employee_documents(
        &self,
        ctx: &Context<'_>,
        employee_id: Option<ID>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<EmployeeDocumentDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let emp = if let Some(id) = &employee_id {
            parse_uuid(id, "employee id")?
        } else {
            resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?
        };
        let rows = document_service::list_employee_documents(&db, tenant_id, emp, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows
            .into_iter()
            .map(EmployeeDocumentDto::from)
            .collect())
    }
}

fn parse_uuid(raw: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(raw.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}
