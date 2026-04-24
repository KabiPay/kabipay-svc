//! GraphQL mutations for employees.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    context::ClientClaims,
    subgraph::{
        require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db,
    },
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::scope::assert_employee_in_data_scope;
use crate::resolvers::types::{
    CreateEmployeeInput, EmployeeDocumentDto, EmployeeDto, OnboardingChecklistItemDto,
    UpdateEmployeeInput, UploadEmployeeDocumentInput,
};
use crate::services::document_file_service;
use crate::services::employee_service::{self, EmployeePatch, NewEmployee};
use crate::services::onboarding_service;

use crate::entities::d0008_document_system::document_type;
use crate::entities::d0017_onboarding_offboarding::onboarding_checklist;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

fn opt_uuid(id: &Option<ID>, field: &'static str) -> Result<Option<Uuid>> {
    match id {
        None => Ok(None),
        Some(i) => Ok(Some(parse_uuid(i, field)?)),
    }
}

/// Enforce RBAC for directory-changing employee writes.
///
/// - Valid **client JWT** must include `employee:write` or `employee:manage`, **or** role
///   `HR_ADMIN` / `TENANT_ADMIN` / `ORG_ADMIN` (from loaded `user_role` at login).
/// - **Dev only:** set `KABIPAY_EMPLOYEE_MUTATION_HEADER_OK=1` to allow unauthenticated
///   `x-tenant-id` (no claims) for local automation — never in production.
/// - **Insecure back-compat:** `KABIPAY_INSECURE_ALLOW_EMPTY_RBAC=1` allows a JWT with empty
///   `roles` + `permissions` (forces re-seed in real deployments).
fn require_employee_mutation_rbac(ctx: &Context<'_>) -> Result<()> {
    if ctx.data_opt::<ClientClaims>().is_none() {
        if std::env::var("KABIPAY_EMPLOYEE_MUTATION_HEADER_OK").as_deref() == Ok("1") {
            return Ok(());
        }
        return Err(KabiPayError::Unauthorised.into_graphql());
    }
    let claims = require_client_claims(ctx)?;
    if std::env::var("KABIPAY_INSECURE_ALLOW_EMPTY_RBAC").as_deref() == Ok("1")
        && claims.roles.is_empty()
        && claims.permissions.is_empty()
    {
        return Ok(());
    }
    if !claims.can_manage_employee_directory() {
        return Err(KabiPayError::Forbidden(
            "employee:write, employee:manage, or HR_ADMIN / TENANT_ADMIN role required".into(),
        )
        .into_graphql());
    }
    Ok(())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_employee(
        &self,
        ctx: &Context<'_>,
        input: CreateEmployeeInput,
    ) -> Result<EmployeeDto> {
        require_employee_mutation_rbac(ctx)?;
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let data = NewEmployee {
            employee_code: input.employee_code,
            first_name: input.first_name,
            last_name: input.last_name,
            date_of_joining: input.date_of_joining,
            department_id: opt_uuid(&input.department_id, "departmentId")?,
            designation_id: opt_uuid(&input.designation_id, "designationId")?,
            reporting_manager_id: opt_uuid(&input.reporting_manager_id, "reportingManagerId")?,
            employment_type: input.employment_type,
            status: input
                .status
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "ACTIVE".into()),
            user_id: opt_uuid(&input.user_id, "userId")?,
        };
        let m = employee_service::create(&db, tenant_id, data)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(EmployeeDto::from(m))
    }

    async fn update_employee(
        &self,
        ctx: &Context<'_>,
        input: UpdateEmployeeInput,
    ) -> Result<EmployeeDto> {
        require_employee_mutation_rbac(ctx)?;
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let eid = parse_uuid(&input.id, "id")?;
        let reporting_manager_id = match input.reporting_manager_id {
            None => None,
            Some(None) => Some(None),
            Some(Some(ref id)) => Some(Some(parse_uuid(id, "reportingManagerId")?)),
        };
        let patch = EmployeePatch {
            first_name: input.first_name,
            last_name: input.last_name,
            department_id: opt_uuid(&input.department_id, "departmentId")?,
            designation_id: opt_uuid(&input.designation_id, "designationId")?,
            reporting_manager_id,
            employment_type: input.employment_type,
            status: input.status,
            user_id: opt_uuid(&input.user_id, "userId")?,
        };
        let m = employee_service::update(&db, tenant_id, eid, patch)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(EmployeeDto::from(m))
    }

    /// Upload a file to local `KABIPAY_LOCAL_FILE_ROOT` and attach an `employee_document` row
    /// (`PENDING`). Caller must be in the same `employee` data scope as the target (self / team / dept
    /// / all), or set **dev** `KABIPAY_EMPLOYEE_MUTATION_HEADER_OK=1` for unauthenticated `x-tenant-id`.
    async fn upload_employee_document(
        &self,
        ctx: &Context<'_>,
        input: UploadEmployeeDocumentInput,
    ) -> Result<EmployeeDocumentDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let eid = parse_uuid(&input.employee_id, "employeeId")?;
        let dtid = parse_uuid(&input.document_type_id, "documentTypeId")?;
        assert_employee_in_data_scope(ctx, &db, tenant_id, eid).await?;

        if document_type::Entity::find_by_id(dtid)
            .filter(document_type::Column::TenantId.eq(tenant_id))
            .filter(document_type::Column::IsDeleted.eq(false))
            .one(&db)
            .await
            .map_err(|e: sea_orm::DbErr| KabiPayError::from(e).into_graphql())?
            .is_none()
        {
            return Err(KabiPayError::NotFound {
                entity: "documentType",
                id: dtid.to_string(),
            }
            .into_graphql());
        }

        let uploader = if let Some(c) = ctx.data_opt::<ClientClaims>() {
            Some(c.sub)
        } else if std::env::var("KABIPAY_EMPLOYEE_MUTATION_HEADER_OK").as_deref() == Ok("1") {
            None
        } else {
            return Err(KabiPayError::Unauthorised.into_graphql());
        };

        let bytes = STANDARD
            .decode(input.content_base64.as_bytes())
            .map_err(|e| KabiPayError::Validation(format!("contentBase64: {e}")).into_graphql())?;

        let m = document_file_service::upload_employee_document(
            &db,
            tenant_id,
            eid,
            dtid,
            uploader,
            input.file_name,
            input.mime_type,
            bytes,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(EmployeeDocumentDto::from(m))
    }

    /// Mark an onboarding checklist row complete or incomplete. Employees may update **their own**
    /// tasks; HR / directory roles may update tasks for employees in their data scope.
    async fn set_onboarding_checklist_item_completed(
        &self,
        ctx: &Context<'_>,
        checklist_item_id: ID,
        is_completed: bool,
    ) -> Result<OnboardingChecklistItemDto> {
        let claims = require_client_claims(ctx)?;
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let task_id = parse_uuid(&checklist_item_id, "checklistItemId")?;
        let row = onboarding_checklist::Entity::find_by_id(task_id)
            .filter(onboarding_checklist::Column::TenantId.eq(tenant_id))
            .one(&db)
            .await
            .map_err(|e: sea_orm::DbErr| KabiPayError::from(e).into_graphql())?
            .ok_or_else(|| {
                KabiPayError::NotFound {
                    entity: "onboardingChecklistItem",
                    id: task_id.to_string(),
                }
                .into_graphql()
            })?;
        let viewer = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        if claims.can_manage_employee_directory() {
            assert_employee_in_data_scope(ctx, &db, tenant_id, row.employee_id).await?;
        } else if row.employee_id != viewer {
            return Err(KabiPayError::Forbidden(
                "you can only update your own onboarding tasks".into(),
            )
            .into_graphql());
        }
        let m = onboarding_service::set_task_completed(&db, tenant_id, task_id, is_completed)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(OnboardingChecklistItemDto::from(m))
    }
}
