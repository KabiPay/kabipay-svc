//! Root query resolvers for kabipay-employee.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::require_tenant_id, subgraph::resolve_client_employee_id, subgraph::tenant_db,
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{
    DepartmentDto, DesignationDto, DocumentTypeDto, EmployeeDocumentDto, EmployeeDto,
    OnboardingChecklistItemDto, OrgChartRowDto,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::entities::d0008_document_system::employee_document;
use crate::resolvers::scope::{
    assert_employee_in_data_scope, data_scope_employee, resolve_viewer_employee,
};
use crate::services::document_file_service::{self, download_claims};
use crate::services::{document_service, employee_service, onboarding_service, org_service};

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
        resolve_employee_dto(ctx, id).await
    }

    /// Apollo Federation **entity** lookup (`_entities`) — not exposed as a public `Query` field.
    /// Enables `type Employee @key(fields: "id")` in the subgraph SDL (**M9**).
    #[graphql(entity)]
    async fn find_employee_by_id(&self, ctx: &Context<'_>, id: ID) -> Result<Option<EmployeeDto>> {
        resolve_employee_dto(ctx, id).await
    }

    /// List the first `limit` employees in the caller's tenant (capped at 100).
    async fn employees(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: u64,
    ) -> Result<Vec<EmployeeDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let scope = data_scope_employee(ctx);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let models = employee_service::list(&db, tenant_id, limit, scope, viewer)
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
            let eid = parse_uuid(id, "employee id")?;
            assert_employee_in_data_scope(ctx, &db, tenant_id, eid).await?;
            eid
        } else {
            resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?
        };
        let rows = document_service::list_employee_documents(&db, tenant_id, emp, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(EmployeeDocumentDto::from).collect())
    }

    /// Departments in the tenant (org hierarchy). Excludes soft-deleted rows.
    async fn departments(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<DepartmentDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = org_service::list_departments(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(DepartmentDto::from).collect())
    }

    /// Job titles / designations in the tenant. Excludes soft-deleted rows.
    async fn designations(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<DesignationDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = org_service::list_designations(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(DesignationDto::from).collect())
    }

    /// Reporting hierarchy as a **flat** list (`reportingManagerId` → parent). Build a tree in the client.
    /// Respects the same **`employee`** `resource_scopes` as **`employees`** (SELF / TEAM / DEPARTMENT / ALL).
    async fn org_chart(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 500)] limit: u64,
    ) -> Result<Vec<OrgChartRowDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let scope = data_scope_employee(ctx);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let models = employee_service::list_for_org_chart(&db, tenant_id, limit, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;

        let mut dept_ids: Vec<Uuid> = models.iter().filter_map(|e| e.department_id).collect();
        dept_ids.sort_unstable();
        dept_ids.dedup();
        let mut desig_ids: Vec<Uuid> = models.iter().filter_map(|e| e.designation_id).collect();
        desig_ids.sort_unstable();
        desig_ids.dedup();

        let dept_map = org_service::map_department_names(&db, tenant_id, &dept_ids)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let desig_map = org_service::map_designation_titles(&db, tenant_id, &desig_ids)
            .await
            .map_err(KabiPayError::into_graphql)?;

        let rows = models
            .into_iter()
            .map(|m| {
                let full_name = format!("{} {}", m.first_name.trim(), m.last_name.trim())
                    .trim()
                    .to_string();
                OrgChartRowDto {
                    employee_id: ID(m.id.to_string()),
                    employee_code: m.employee_code,
                    full_name,
                    reporting_manager_id: m.reporting_manager_id.map(|u| ID(u.to_string())),
                    department_name: m.department_id.and_then(|id| dept_map.get(&id).cloned()),
                    designation_title: m.designation_id.and_then(|id| desig_map.get(&id).cloned()),
                }
            })
            .collect();
        Ok(rows)
    }

    /// HMAC time-limited URL for `GET /files/employee-document?token=...` (no `Authorization` on GET).
    /// Caller must be able to read the employee who owns the document.
    async fn employee_document_signed_read_url(
        &self,
        ctx: &Context<'_>,
        employee_document_id: ID,
        #[graphql(default = 600)] ttl_seconds: i32,
    ) -> Result<String> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let doc_id = parse_uuid(&employee_document_id, "employeeDocumentId")?;
        let model = employee_document::Entity::find_by_id(doc_id)
            .filter(employee_document::Column::TenantId.eq(tenant_id))
            .filter(employee_document::Column::IsDeleted.eq(false))
            .one(&db)
            .await
            .map_err(|e: sea_orm::DbErr| KabiPayError::from(e).into_graphql())?
            .ok_or_else(|| {
                KabiPayError::NotFound {
                    entity: "employeeDocument",
                    id: doc_id.to_string(),
                }
                .into_graphql()
            })?;
        let file_id = model.file_storage_id.ok_or_else(|| {
            KabiPayError::Validation("document has no file yet".to_string()).into_graphql()
        })?;
        assert_employee_in_data_scope(ctx, &db, tenant_id, model.employee_id).await?;
        let ttl = ttl_seconds.clamp(60, 86_400) as i64;
        let claims = download_claims(tenant_id, file_id, None, ttl);
        Ok(document_file_service::public_download_url(&claims))
    }

    /// Onboarding tasks for an employee. Omit `employeeId` for the JWT subject’s checklist.
    /// HR / directory managers may pass another employee id (same data-scope rules as documents).
    async fn onboarding_checklist(
        &self,
        ctx: &Context<'_>,
        employee_id: Option<ID>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<OnboardingChecklistItemDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let emp = if let Some(id) = &employee_id {
            let eid = parse_uuid(id, "employee id")?;
            assert_employee_in_data_scope(ctx, &db, tenant_id, eid).await?;
            eid
        } else {
            resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?
        };
        let rows = onboarding_service::list_checklist_for_employee(&db, tenant_id, emp, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(OnboardingChecklistItemDto::from).collect())
    }
}

async fn resolve_employee_dto(ctx: &Context<'_>, id: ID) -> Result<Option<EmployeeDto>> {
    let tenant_id = require_tenant_id(ctx)?;
    let employee_id = parse_uuid(&id, "employee id")?;
    let db = tenant_db(ctx, tenant_id).await?;
    let model = employee_service::find_by_id(&db, tenant_id, employee_id)
        .await
        .map_err(KabiPayError::into_graphql)?;
    let model = if let Some(ref m) = model {
        let scope = data_scope_employee(ctx);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        if employee_service::is_employee_in_scope(scope, viewer, m) {
            model
        } else {
            None
        }
    } else {
        model
    };
    Ok(model.map(EmployeeDto::from))
}

fn parse_uuid(raw: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(raw.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}
