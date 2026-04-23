//! Root query resolvers for kabipay-billing (ops plane).

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{subgraph::ops_db, KabiPayError};
use uuid::Uuid;

use crate::resolvers::types::{InvoiceDto, PaymentDto};
use crate::services::billing_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn billing_health(&self) -> &'static str {
        "ok"
    }

    /// List invoices across all tenants. Scope down with `tenantId` if provided.
    async fn invoices(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<InvoiceDto>> {
        let db = ops_db(ctx)?;
        let tenant = parse_uuid_opt(tenant_id, "tenantId")?;
        let rows = billing_service::list_invoices(db, tenant, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(InvoiceDto::from).collect())
    }

    async fn payments(
        &self,
        ctx: &Context<'_>,
        invoice_id: Option<ID>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<PaymentDto>> {
        let db = ops_db(ctx)?;
        let inv = parse_uuid_opt(invoice_id, "invoiceId")?;
        let rows = billing_service::list_payments(db, inv, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(PaymentDto::from).collect())
    }
}

fn parse_uuid_opt(id: Option<ID>, field: &str) -> Result<Option<Uuid>> {
    match id {
        None => Ok(None),
        Some(id) => Uuid::parse_str(&id.0).map(Some).map_err(|e| {
            KabiPayError::Validation(format!("{field} is not a UUID: {e}")).into_graphql()
        }),
    }
}
