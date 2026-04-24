//! Write operations for employee tax computations / declarations.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::resolvers::types::{
    SubmitTaxProofLineInput, TaxComputationDto, TaxProofLineDto, UpsertTaxComputationInput,
};
use crate::services::tax_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create or update the `tax_computation` row for this employee, config version, and year.
    ///
    /// **Note:** `totalDeductions` may be **overwritten** when tax proof lines are approved
    /// (see `submitTaxProofLine` / `approveTaxProofLine`); use `taxProofLines` + approved
    /// workflow for year-end truth.
    async fn upsert_tax_computation(
        &self,
        ctx: &Context<'_>,
        input: UpsertTaxComputationInput,
    ) -> Result<TaxComputationDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let v = parse_uuid(&input.tax_config_version_id, "taxConfigVersionId")?;
        let m = tax_service::upsert_tax_computation(
            &db,
            tenant_id,
            employee_id,
            v,
            input.fiscal_year,
            input.tax_regime_chosen,
            tax_service::opt_decimal(&input.gross_income).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.total_deductions).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.taxable_income).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.final_tax).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.tds_per_month).map_err(KabiPayError::into_graphql)?,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(TaxComputationDto::from(m))
    }

    /// Submit or update a deduction **proof** line (declared vs actual). Resets status to **PENDING**
    /// until an approver accepts it. Only **APPROVED** lines sum into `tax_computation.totalDeductions`.
    async fn submit_tax_proof_line(
        &self,
        ctx: &Context<'_>,
        input: SubmitTaxProofLineInput,
    ) -> Result<TaxProofLineDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let tid = parse_uuid(&input.tax_config_version_id, "taxConfigVersionId")?;
        let declared = Decimal::from_str(input.declared_amount.trim())
            .map_err(|_| KabiPayError::Validation("invalid declaredAmount".into()))?;
        let actual = Decimal::from_str(input.actual_amount.trim())
            .map_err(|_| KabiPayError::Validation("invalid actualAmount".into()))?;
        let fid = input
            .file_storage_id
            .as_ref()
            .map(|id| parse_uuid(id, "fileStorageId"))
            .transpose()?;
        let m = tax_service::submit_tax_proof_line(
            &db,
            tenant_id,
            employee_id,
            tid,
            input.fiscal_year,
            input.section_code,
            declared,
            actual,
            fid,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(TaxProofLineDto::from(m))
    }

    async fn approve_tax_proof_line(
        &self,
        ctx: &Context<'_>,
        tax_proof_line_id: ID,
    ) -> Result<TaxProofLineDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_approve_tax_proof() {
            return Err(
                KabiPayError::Forbidden(
                    "tax proof approve permission required (tax:approve or HR / tenant admin role)"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let id = parse_uuid(&tax_proof_line_id, "taxProofLineId")?;
        let m = tax_service::approve_tax_proof_line(&db, tenant_id, id, claims.sub)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(TaxProofLineDto::from(m))
    }

    async fn reject_tax_proof_line(
        &self,
        ctx: &Context<'_>,
        tax_proof_line_id: ID,
        reason: Option<String>,
    ) -> Result<TaxProofLineDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_approve_tax_proof() {
            return Err(
                KabiPayError::Forbidden(
                    "tax proof approve permission required (tax:approve or HR / tenant admin role)"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let id = parse_uuid(&tax_proof_line_id, "taxProofLineId")?;
        let m = tax_service::reject_tax_proof_line(&db, tenant_id, id, reason)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(TaxProofLineDto::from(m))
    }
}
