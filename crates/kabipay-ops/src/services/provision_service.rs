//! Tenant provisioning (ops plane) — aligns with `scripts/provision-tenant.ps1`.

use chrono::Utc;
use kabipay_common::{
    db::{derive_tenant_schema_name, TenantDbCache},
    deterministic_tenant_database_row_uuid, deterministic_tenant_uuid, KabiPayError, KabiPayResult,
};
use kabipay_db_entities::ops::{tenant, tenant_database};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use uuid::Uuid;

fn validate_tenant_code(code: &str) -> KabiPayResult<()> {
    let c = code.trim();
    if c.len() < 2 || c.len() > 32 {
        return Err(KabiPayError::Validation(
            "tenant code must be 2–32 characters".into(),
        ));
    }
    let mut chars = c.chars();
    let Some(first) = chars.next() else {
        return Err(KabiPayError::Validation("tenant code is empty".into()));
    };
    if !first.is_ascii_alphanumeric() {
        return Err(KabiPayError::Validation(
            "tenant code must start with a letter or digit".into(),
        ));
    }
    for ch in std::iter::once(first).chain(chars) {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            continue;
        }
        return Err(KabiPayError::Validation(
            "tenant code may only contain a-z, 0-9, underscore, hyphen".into(),
        ));
    }
    Ok(())
}

fn validate_schema_override(schema: &str) -> KabiPayResult<()> {
    if schema.len() < 8 || schema.len() > 50 {
        return Err(KabiPayError::Validation(
            "schema name length invalid".into(),
        ));
    }
    if !schema.starts_with("tenant_") {
        return Err(KabiPayError::Validation(
            "schema name must start with tenant_".into(),
        ));
    }
    for ch in schema.chars().skip(7) {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' {
            continue;
        }
        return Err(KabiPayError::Validation(
            "schema name may only contain a-z, 0-9, underscore after tenant_".into(),
        ));
    }
    Ok(())
}

async fn invoke_tenant_liquibase(schema_name: &str) -> KabiPayResult<()> {
    let db_dir = std::env::var("KABIPAY_DATABASE_DIR").map_err(|_| {
        KabiPayError::Validation(
            "KABIPAY_DATABASE_DIR must point at the kabipay-database repo to run Liquibase".into(),
        )
    })?;
    if db_dir.is_empty() {
        return Err(KabiPayError::Validation(
            "KABIPAY_DATABASE_DIR is empty".into(),
        ));
    }

    let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".into());
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".into());
    let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "kabipay_dev".into());
    let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "kabipay".into());
    let pass = std::env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "changeme".into());
    let ssl = std::env::var("POSTGRES_SSLMODE").unwrap_or_default();

    let mut jdbc = format!("jdbc:postgresql://{host}:{port}/{db}");
    if ssl == "require" {
        jdbc.push_str(if jdbc.contains('?') {
            "&sslmode=require"
        } else {
            "?sslmode=require"
        });
    }

    let tracking = format!("{schema_name}_databasechangelog");
    let props_name = format!(".generated-tenant-{schema_name}.properties");
    let props_path = std::path::Path::new(&db_dir).join(&props_name);
    let props_body = format!(
        "changeLogFile=changelog/tenant.changelog-master.xml\n\
         url={jdbc}\n\
         username={user}\n\
         password={pass}\n\
         driver=org.postgresql.Driver\n\
         logLevel=INFO\n\
         defaultSchemaName={schema_name}\n\
         databaseChangeLogTableName={tracking}\n\
         parameter.schema={schema_name}\n\
         liquibase.hub.mode=off\n"
    );

    tokio::fs::write(&props_path, props_body)
        .await
        .map_err(|e| KabiPayError::Internal(format!("write liquibase props: {e}")))?;

    let status = tokio::process::Command::new("node")
        .arg("run-liquibase.cjs")
        .arg(format!("--defaults-file={props_name}"))
        .arg("update")
        .current_dir(&db_dir)
        .status()
        .await
        .map_err(|e| KabiPayError::Internal(format!("liquibase spawn: {e}")))?;

    let _ = tokio::fs::remove_file(&props_path).await;

    if !status.success() {
        return Err(KabiPayError::Internal(format!(
            "Liquibase tenant update failed (exit {status}). Check node, kabipay-database npm install, and DB connectivity."
        )));
    }
    Ok(())
}

pub struct ProvisionOutcome {
    pub tenant: tenant::Model,
    pub schema_name: String,
    pub migrations_ran: bool,
    pub detail: Option<String>,
}

/// Create/update ops rows, create PostgreSQL schema, optionally run Liquibase tenant changelog.
pub async fn provision_tenant(
    db: &DatabaseConnection,
    cache: &TenantDbCache,
    name: String,
    code: String,
    country: Option<String>,
    currency: Option<String>,
    schema_name_override: Option<String>,
    run_migrations: bool,
) -> KabiPayResult<ProvisionOutcome> {
    validate_tenant_code(&code)?;
    let code = code.trim().to_string();

    let tenant_id = deterministic_tenant_uuid(&code);
    let tenant_db_row_id = deterministic_tenant_database_row_uuid(&code);

    let schema_name = if let Some(ref s) = schema_name_override {
        validate_schema_override(s)?;
        s.clone()
    } else {
        derive_tenant_schema_name(tenant_id)
    };

    if let Some(existing) = tenant::Entity::find_by_id(tenant_id).one(db).await? {
        if existing.status == "TERMINATED" {
            return Err(KabiPayError::Conflict(
                "tenant is terminated; restore it before re-provisioning".into(),
            ));
        }
    }

    let country = country.unwrap_or_else(|| "IN".into());
    let currency = currency.unwrap_or_else(|| "INR".into());
    let subdomain = code.to_lowercase();

    let txn = db.begin().await?;

    let sql = format!(
        "CREATE SCHEMA IF NOT EXISTS {} AUTHORIZATION CURRENT_USER",
        schema_name
    );
    txn.execute_unprepared(&sql).await.map_err(|e| {
        KabiPayError::Internal(format!("create schema {schema_name}: {e}"))
    })?;

    let now = Utc::now();
    let tenant_am = tenant::ActiveModel {
        id: Set(tenant_id),
        name: Set(name.clone()),
        status: Set("PROVISIONING".into()),
        plan: Set(None),
        country: Set(Some(country.clone())),
        timezone: Set(None),
        currency: Set(Some(currency.clone())),
        gstin: Set(None),
        pan: Set(None),
        registered_address: Set(None),
        logo_url: Set(None),
        primary_color: Set(None),
        subdomain: Set(Some(subdomain.clone())),
        account_manager_id: Set(None),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    use sea_orm::sea_query::OnConflict;
    tenant::Entity::insert(tenant_am)
        .on_conflict(
            OnConflict::column(tenant::Column::Id)
                .update_columns([
                    tenant::Column::Name,
                    tenant::Column::Country,
                    tenant::Column::Currency,
                    tenant::Column::Subdomain,
                    tenant::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(&txn)
        .await?;

    let db_host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".into());
    let db_name = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "kabipay_dev".into());

    let tdb_am = tenant_database::ActiveModel {
        id: Set(tenant_db_row_id),
        tenant_id: Set(tenant_id),
        db_type: Set("POSTGRES".into()),
        db_host: Set(db_host),
        db_name: Set(db_name),
        schema_name: Set(schema_name.clone()),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    tenant_database::Entity::insert(tdb_am)
        .on_conflict(
            OnConflict::column(tenant_database::Column::Id)
                .update_columns([
                    tenant_database::Column::TenantId,
                    tenant_database::Column::DbHost,
                    tenant_database::Column::DbName,
                    tenant_database::Column::SchemaName,
                    tenant_database::Column::IsActive,
                    tenant_database::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(&txn)
        .await?;

    txn.commit().await?;

    cache.invalidate(tenant_id);

    let mut migrations_ran = false;
    let mut detail: Option<String> = None;

    if run_migrations {
        match invoke_tenant_liquibase(&schema_name).await {
            Ok(()) => {
                migrations_ran = true;
                let row = tenant::Entity::find_by_id(tenant_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| KabiPayError::Internal("tenant row missing".into()))?;
                let mut am: tenant::ActiveModel = row.into();
                am.status = Set("ACTIVE".into());
                am.updated_at = Set(Utc::now());
                am.update(db).await?;
            }
            Err(e) => {
                detail = Some(format!("{e}"));
            }
        }
    } else {
        detail = Some(
            "runMigrations was false; tenant remains PROVISIONING until migrations run".into(),
        );
    }

    let tenant_row = tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("tenant row missing after provision".into()))?;

    Ok(ProvisionOutcome {
        tenant: tenant_row,
        schema_name,
        migrations_ran,
        detail,
    })
}

/// Run Liquibase for an existing tenant (retry path).
pub async fn run_tenant_migrations(
    db: &DatabaseConnection,
    cache: &TenantDbCache,
    tenant_id: Uuid,
) -> KabiPayResult<ProvisionOutcome> {
    let t = tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant",
            id: tenant_id.to_string(),
        })?;
    if t.status == "TERMINATED" {
        return Err(KabiPayError::Conflict(
            "terminated tenant cannot run migrations".into(),
        ));
    }

    let db_row = tenant_database::Entity::find()
        .filter(tenant_database::Column::TenantId.eq(tenant_id))
        .filter(tenant_database::Column::IsActive.eq(true))
        .one(db)
        .await?
        .ok_or_else(|| {
            KabiPayError::NotFound {
                entity: "tenant_database",
                id: tenant_id.to_string(),
            }
        })?;

    let schema_name = db_row.schema_name.clone();
    invoke_tenant_liquibase(&schema_name).await?;

    let now = Utc::now();
    let mut active: tenant::ActiveModel = t.into();
    active.status = Set("ACTIVE".into());
    active.updated_at = Set(now);
    let tenant_row = active.update(db).await?;

    cache.invalidate(tenant_id);

    Ok(ProvisionOutcome {
        tenant: tenant_row,
        schema_name,
        migrations_ran: true,
        detail: None,
    })
}
