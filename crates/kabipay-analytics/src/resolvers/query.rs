//! Root query resolvers for kabipay-analytics.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{
    DashboardRowDto, DashboardWidgetRowDto, OutboxEventDto, ReportDefinitionDto, ReportScheduleDto,
    WorkforceSnapshotDto,
};
use crate::services::analytics_service;

pub struct QueryRoot;

fn parse_opt_uuid(id: &Option<ID>, field: &'static str) -> Result<Option<Uuid>> {
    match id {
        None => Ok(None),
        Some(v) => Uuid::parse_str(v.as_str())
            .map_err(|e| {
                KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql()
            })
            .map(Some),
    }
}

#[Object]
impl QueryRoot {
    async fn analytics_health(&self) -> &'static str {
        "ok"
    }

    async fn report_definitions(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<ReportDefinitionDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = analytics_service::list_report_definitions(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ReportDefinitionDto::from).collect())
    }

    async fn report_schedules(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<ReportScheduleDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = analytics_service::list_report_schedules(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ReportScheduleDto::from).collect())
    }

    async fn dashboards(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<DashboardRowDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = analytics_service::list_dashboards(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(DashboardRowDto::from).collect())
    }

    async fn dashboard_widgets(
        &self,
        ctx: &Context<'_>,
        dashboard_id: Option<ID>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<DashboardWidgetRowDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let did = parse_opt_uuid(&dashboard_id, "dashboardId")?;
        let rows = analytics_service::list_dashboard_widgets(&db, tenant_id, did, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(DashboardWidgetRowDto::from).collect())
    }

    async fn workforce_snapshots(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 24)] limit: u64,
    ) -> Result<Vec<WorkforceSnapshotDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = analytics_service::list_workforce_snapshots(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(WorkforceSnapshotDto::from).collect())
    }

    /// **HR / directory admins only** — inspect transactional outbox rows (e.g. after leave approval).
    async fn outbox_events(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<OutboxEventDto>> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_employee_directory() {
            return Err(
                KabiPayError::Forbidden(
                    "HR or employee directory access required to view outbox".into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = analytics_service::list_outbox_events(&db, tenant_id, status, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(OutboxEventDto::from).collect())
    }
}
