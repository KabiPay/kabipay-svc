//! Write operations for grievance cases.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{
        require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db,
    },
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{GrievanceCaseDto, SubmitGrievanceCaseInput};
use crate::services::grievance_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// File a grievance case for the signed-in employee.
    async fn submit_grievance_case(
        &self,
        ctx: &Context<'_>,
        input: SubmitGrievanceCaseInput,
    ) -> Result<GrievanceCaseDto> {
        let _claims = require_client_claims(ctx)?;
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let cat = parse_uuid(&input.grievance_category_id, "grievanceCategoryId")?;
        let m = grievance_service::submit_case(
            &db,
            tenant_id,
            employee_id,
            cat,
            &input.subject,
            input.description.as_deref(),
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(GrievanceCaseDto::from(m))
    }
}
