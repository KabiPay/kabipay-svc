//! Mutations for kabipay-analytics (HR / directory tools).

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::OutboxEventDto;
use crate::services::analytics_service;

pub struct MutationRoot;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

#[Object]
impl MutationRoot {
    /// Send a **FAILED** or **PROCESSING** outbox row back to **PENDING** (same RBAC as `outboxEvents`).
    async fn requeue_outbox_event(&self, ctx: &Context<'_>, id: ID) -> Result<OutboxEventDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_employee_directory() {
            return Err(
                KabiPayError::Forbidden("HR or directory access required to requeue outbox".into())
                    .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let oid = parse_uuid(&id, "id")?;
        let m = analytics_service::requeue_outbox_event(&db, tenant_id, oid)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(OutboxEventDto::from(m))
    }
}
