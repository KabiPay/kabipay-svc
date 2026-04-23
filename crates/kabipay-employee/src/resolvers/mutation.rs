//! GraphQL mutations for employees.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{CreateEmployeeInput, EmployeeDto, UpdateEmployeeInput};
use crate::services::employee_service::{self, EmployeePatch, NewEmployee};

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

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_employee(
        &self,
        ctx: &Context<'_>,
        input: CreateEmployeeInput,
    ) -> Result<EmployeeDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let data = NewEmployee {
            employee_code: input.employee_code,
            first_name: input.first_name,
            last_name: input.last_name,
            date_of_joining: input.date_of_joining,
            department_id: opt_uuid(&input.department_id, "departmentId")?,
            designation_id: opt_uuid(&input.designation_id, "designationId")?,
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
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let eid = parse_uuid(&input.id, "id")?;
        let patch = EmployeePatch {
            first_name: input.first_name,
            last_name: input.last_name,
            department_id: opt_uuid(&input.department_id, "departmentId")?,
            designation_id: opt_uuid(&input.designation_id, "designationId")?,
            employment_type: input.employment_type,
            status: input.status,
            user_id: opt_uuid(&input.user_id, "userId")?,
        };
        let m = employee_service::update(&db, tenant_id, eid, patch)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(EmployeeDto::from(m))
    }
}
