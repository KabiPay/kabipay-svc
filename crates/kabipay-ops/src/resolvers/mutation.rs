//! Operator mutations for kabipay-ops.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::db::TenantDbCache;
use kabipay_common::subgraph::{ops_db, require_operator_context};
use kabipay_common::KabiPayError;
use uuid::Uuid;

use crate::resolvers::types::{
    CreateInvoiceInput, CreateOperatorUserInput, FeatureFlagDto, InvoiceDto, ModuleDto,
    OperatorUserDto, PaymentDto, ProvisionTenantInput, ProvisionTenantPayload, RecordPaymentInput,
    TenantDto, TenantSubscriptionDto, UpdateTenantInput, UpsertTenantSubscriptionInput,
};
use crate::services::{operator_service, ops_write_service, provision_service, tenant_write_service};

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn provision_tenant(
        &self,
        ctx: &Context<'_>,
        input: ProvisionTenantInput,
    ) -> Result<ProvisionTenantPayload> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let cache = ctx.data::<TenantDbCache>().map_err(|_| {
            KabiPayError::Internal("TenantDbCache missing from schema data".into()).into_graphql()
        })?;
        let out = provision_service::provision_tenant(
            db,
            cache,
            input.name,
            input.code,
            input.country,
            input.currency,
            input.schema_name_override,
            input.run_migrations,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(ProvisionTenantPayload {
            tenant: TenantDto::from(out.tenant),
            schema_name: out.schema_name,
            migrations_ran: out.migrations_ran,
            detail: out.detail,
        })
    }

    async fn run_tenant_migrations(
        &self,
        ctx: &Context<'_>,
        tenant_id: ID,
    ) -> Result<ProvisionTenantPayload> {
        require_operator_context(ctx)?;
        let tid = Uuid::parse_str(&tenant_id.0).map_err(|e| {
            KabiPayError::Validation(format!("tenantId: {e}")).into_graphql()
        })?;
        let db = ops_db(ctx)?;
        let cache = ctx.data::<TenantDbCache>().map_err(|_| {
            KabiPayError::Internal("TenantDbCache missing from schema data".into()).into_graphql()
        })?;
        let out = provision_service::run_tenant_migrations(db, cache, tid)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(ProvisionTenantPayload {
            tenant: TenantDto::from(out.tenant),
            schema_name: out.schema_name,
            migrations_ran: out.migrations_ran,
            detail: out.detail,
        })
    }

    async fn upsert_tenant_subscription(
        &self,
        ctx: &Context<'_>,
        input: UpsertTenantSubscriptionInput,
    ) -> Result<TenantSubscriptionDto> {
        let op = require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let tenant_id = Uuid::parse_str(&input.tenant_id.0).map_err(|e| {
            KabiPayError::Validation(format!("tenantId: {e}")).into_graphql()
        })?;
        let module_id = Uuid::parse_str(&input.module_id.0).map_err(|e| {
            KabiPayError::Validation(format!("moduleId: {e}")).into_graphql()
        })?;
        let status = input.status.unwrap_or_else(|| "ACTIVE".into());
        let overage = input.overage_policy.unwrap_or_else(|| "BLOCK".into());
        let row = tenant_write_service::upsert_tenant_subscription(
            db,
            op.operator_user_id,
            tenant_id,
            module_id,
            status,
            input.contracted_seats,
            overage,
            input.activated_at,
            input.expires_at,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(TenantSubscriptionDto::from(row))
    }

    async fn remove_tenant_subscription(
        &self,
        ctx: &Context<'_>,
        subscription_id: ID,
    ) -> Result<bool> {
        let op = require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let sid = Uuid::parse_str(&subscription_id.0).map_err(|e| {
            KabiPayError::Validation(format!("subscriptionId: {e}")).into_graphql()
        })?;
        tenant_write_service::remove_tenant_subscription(db, op.operator_user_id, sid)
            .await
            .map_err(KabiPayError::into_graphql)
    }

    async fn upsert_feature_flag(
        &self,
        ctx: &Context<'_>,
        tenant_id: ID,
        feature_name: String,
        is_enabled: bool,
    ) -> Result<FeatureFlagDto> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let tid = Uuid::parse_str(&tenant_id.0).map_err(|e| {
            KabiPayError::Validation(format!("tenantId: {e}")).into_graphql()
        })?;
        let row = tenant_write_service::upsert_feature_flag(db, tid, feature_name, is_enabled)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(FeatureFlagDto::from(row))
    }

    async fn set_module_active(
        &self,
        ctx: &Context<'_>,
        module_id: ID,
        is_active: bool,
    ) -> Result<ModuleDto> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let mid = Uuid::parse_str(&module_id.0).map_err(|e| {
            KabiPayError::Validation(format!("moduleId: {e}")).into_graphql()
        })?;
        let row = tenant_write_service::set_module_active(db, mid, is_active)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(ModuleDto::from(row))
    }

    async fn update_tenant(
        &self,
        ctx: &Context<'_>,
        input: UpdateTenantInput,
    ) -> Result<TenantDto> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let tid = Uuid::parse_str(&input.tenant_id.0).map_err(|e| {
            KabiPayError::Validation(format!("tenantId: {e}")).into_graphql()
        })?;
        let row = tenant_write_service::update_tenant_fields(
            db,
            tid,
            input.name,
            input.status,
            input.plan,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(TenantDto::from(row))
    }

    async fn create_operator_user(
        &self,
        ctx: &Context<'_>,
        input: CreateOperatorUserInput,
    ) -> Result<OperatorUserDto> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let row = ops_write_service::create_operator_user(
            db,
            input.email,
            input.password,
            input.full_name,
            input.phone,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(OperatorUserDto::from(row))
    }

    async fn create_invoice(
        &self,
        ctx: &Context<'_>,
        input: CreateInvoiceInput,
    ) -> Result<InvoiceDto> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let tenant_id = Uuid::parse_str(&input.tenant_id.0).map_err(|e| {
            KabiPayError::Validation(format!("tenantId: {e}")).into_graphql()
        })?;
        let cycle_id = match input.billing_cycle_id {
            None => None,
            Some(id) => Some(Uuid::parse_str(&id.0).map_err(|e| {
                KabiPayError::Validation(format!("billingCycleId: {e}")).into_graphql()
            })?),
        };
        let row = ops_write_service::create_invoice(
            db,
            tenant_id,
            cycle_id,
            input.subtotal,
            input.discount_total,
            input.tax_amount,
            input.total_amount,
            input.currency,
            input.status,
            input.due_date,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(InvoiceDto::from(row))
    }

    async fn record_payment(
        &self,
        ctx: &Context<'_>,
        input: RecordPaymentInput,
    ) -> Result<PaymentDto> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let invoice_id = Uuid::parse_str(&input.invoice_id.0).map_err(|e| {
            KabiPayError::Validation(format!("invoiceId: {e}")).into_graphql()
        })?;
        let row = ops_write_service::record_payment(
            db,
            invoice_id,
            input.amount,
            input.payment_method,
            input.gateway_ref,
            input.status,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(PaymentDto::from(row))
    }

    /// Replace role assignments for an operator user (`role_ids` may be empty to clear all).
    async fn set_operator_user_roles(
        &self,
        ctx: &Context<'_>,
        operator_user_id: ID,
        role_ids: Vec<ID>,
    ) -> Result<bool> {
        require_operator_context(ctx)?;
        let db = ops_db(ctx)?;
        let uid = Uuid::parse_str(&operator_user_id.0).map_err(|e| {
            KabiPayError::Validation(format!("operatorUserId: {e}")).into_graphql()
        })?;
        let mut rids = Vec::with_capacity(role_ids.len());
        for id in role_ids {
            rids.push(Uuid::parse_str(&id.0).map_err(|e| {
                KabiPayError::Validation(format!("roleId: {e}")).into_graphql()
            })?);
        }
        operator_service::set_user_roles(db, uid, rids)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(true)
    }
}
