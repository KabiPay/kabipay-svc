//! Primary bank account + statutory identity rows (`employee_bank`, `employee_pan`, `employee_aadhaar`).

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};
use uuid::Uuid;

use crate::entities::d0007_employee_core::{employee_aadhaar, employee_bank, employee_pan};

pub async fn find_primary_bank(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<Option<employee_bank::Model>> {
    employee_bank::Entity::find()
        .filter(employee_bank::Column::TenantId.eq(tenant_id))
        .filter(employee_bank::Column::EmployeeId.eq(employee_id))
        .filter(employee_bank::Column::IsPrimary.eq(true))
        .one(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn find_primary_pan(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<Option<employee_pan::Model>> {
    employee_pan::Entity::find()
        .filter(employee_pan::Column::TenantId.eq(tenant_id))
        .filter(employee_pan::Column::EmployeeId.eq(employee_id))
        .filter(employee_pan::Column::IsPrimary.eq(true))
        .one(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn find_primary_aadhaar(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<Option<employee_aadhaar::Model>> {
    employee_aadhaar::Entity::find()
        .filter(employee_aadhaar::Column::TenantId.eq(tenant_id))
        .filter(employee_aadhaar::Column::EmployeeId.eq(employee_id))
        .filter(employee_aadhaar::Column::IsPrimary.eq(true))
        .one(db)
        .await
        .map_err(KabiPayError::from)
}

/// Replace primary bank details for an employee (exactly one `is_primary` row).
pub async fn upsert_primary_bank(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    bank_name: String,
    account_number: String,
    ifsc_code: String,
    account_type: Option<String>,
) -> KabiPayResult<employee_bank::Model> {
    let ifsc_owned = ifsc_code.trim().to_uppercase();
    let acct_owned = account_number.trim().to_string();
    let bank_owned = bank_name.trim().to_string();
    let acct_type = account_type.filter(|s| !s.trim().is_empty());

    if acct_owned.is_empty() || ifsc_owned.is_empty() || bank_owned.is_empty() {
        return Err(KabiPayError::Validation(
            "bankName, accountNumber, and ifscCode are required".into(),
        ));
    }

    let txn = db.begin().await.map_err(KabiPayError::from)?;

    let rows: Vec<employee_bank::Model> = employee_bank::Entity::find()
        .filter(employee_bank::Column::TenantId.eq(tenant_id))
        .filter(employee_bank::Column::EmployeeId.eq(employee_id))
        .all(&txn)
        .await
        .map_err(KabiPayError::from)?;

    let now = Utc::now();

    if rows.is_empty() {
        let id = Uuid::new_v4();
        let am = employee_bank::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            account_number: Set(acct_owned.clone()),
            ifsc_code: Set(ifsc_owned.clone()),
            bank_name: Set(bank_owned.clone()),
            account_type: Set(acct_type.clone()),
            is_primary: Set(true),
            is_verified: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(&txn).await.map_err(KabiPayError::from)?;
        let out = employee_bank::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(KabiPayError::from)?
            .ok_or_else(|| KabiPayError::Internal("inserted bank row missing".into()))?;
        txn.commit().await.map_err(KabiPayError::from)?;
        return Ok(out);
    }

    let key = rows
        .iter()
        .find(|r| r.is_primary)
        .map(|r| r.id)
        .unwrap_or(rows[0].id);

    let mut out: Option<employee_bank::Model> = None;
    for m in rows {
        let is_target = m.id == key;
        let mut am: employee_bank::ActiveModel = m.into();
        if is_target {
            am.bank_name = Set(bank_owned.clone());
            am.account_number = Set(acct_owned.clone());
            am.ifsc_code = Set(ifsc_owned.clone());
            am.account_type = Set(acct_type.clone());
            am.is_primary = Set(true);
            am.is_verified = Set(false);
        } else {
            am.is_primary = Set(false);
        }
        am.updated_at = Set(now);
        am.update(&txn).await.map_err(KabiPayError::from)?;
        if is_target {
            out = employee_bank::Entity::find_by_id(key)
                .one(&txn)
                .await
                .map_err(KabiPayError::from)?;
        }
    }

    let out = out.ok_or_else(|| KabiPayError::Internal("updated bank row missing".into()))?;
    txn.commit().await.map_err(KabiPayError::from)?;
    Ok(out)
}

fn validate_pan(pan: &str) -> KabiPayResult<String> {
    let t = pan.trim().to_uppercase().replace(' ', "");
    if t.len() != 10 {
        return Err(KabiPayError::Validation("PAN must be 10 characters".into()));
    }
    let b = t.as_bytes();
    for i in 0..5 {
        if !matches!(b[i], b'A'..=b'Z') {
            return Err(KabiPayError::Validation(
                "PAN: first 5 characters must be letters".into(),
            ));
        }
    }
    for i in 5..9 {
        if !matches!(b[i], b'0'..=b'9') {
            return Err(KabiPayError::Validation(
                "PAN: characters 6–9 must be digits".into(),
            ));
        }
    }
    if !matches!(b[9], b'A'..=b'Z') {
        return Err(KabiPayError::Validation(
            "PAN: last character must be a letter".into(),
        ));
    }
    Ok(t)
}

fn normalize_aadhaar_last4(raw: &str) -> KabiPayResult<String> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let last4 = if digits.len() == 12 {
        digits[8..12].to_string()
    } else if digits.len() == 4 {
        digits
    } else {
        return Err(KabiPayError::Validation(
            "Aadhaar: enter 12 digits or the last 4 digits".into(),
        ));
    };
    Ok(last4)
}

/// Replace primary PAN for an employee (exactly one `is_primary` row). Resets verification.
pub async fn upsert_primary_pan(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    pan_number: String,
) -> KabiPayResult<employee_pan::Model> {
    let pan_owned = validate_pan(&pan_number)?;

    let txn = db.begin().await.map_err(KabiPayError::from)?;

    let rows: Vec<employee_pan::Model> = employee_pan::Entity::find()
        .filter(employee_pan::Column::TenantId.eq(tenant_id))
        .filter(employee_pan::Column::EmployeeId.eq(employee_id))
        .all(&txn)
        .await
        .map_err(KabiPayError::from)?;

    let now = Utc::now();

    if rows.is_empty() {
        let id = Uuid::new_v4();
        let am = employee_pan::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            pan_number: Set(pan_owned),
            is_primary: Set(true),
            is_verified: Set(false),
            verified_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(&txn).await.map_err(KabiPayError::from)?;
        let out = employee_pan::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(KabiPayError::from)?
            .ok_or_else(|| KabiPayError::Internal("inserted PAN row missing".into()))?;
        txn.commit().await.map_err(KabiPayError::from)?;
        return Ok(out);
    }

    let key = rows
        .iter()
        .find(|r| r.is_primary)
        .map(|r| r.id)
        .unwrap_or(rows[0].id);

    let mut out: Option<employee_pan::Model> = None;
    for m in rows {
        let is_target = m.id == key;
        let mut am: employee_pan::ActiveModel = m.into();
        if is_target {
            am.pan_number = Set(pan_owned.clone());
            am.is_primary = Set(true);
            am.is_verified = Set(false);
            am.verified_at = Set(None);
        } else {
            am.is_primary = Set(false);
        }
        am.updated_at = Set(now);
        am.update(&txn).await.map_err(KabiPayError::from)?;
        if is_target {
            out = employee_pan::Entity::find_by_id(key)
                .one(&txn)
                .await
                .map_err(KabiPayError::from)?;
        }
    }

    let out = out.ok_or_else(|| KabiPayError::Internal("updated PAN row missing".into()))?;
    txn.commit().await.map_err(KabiPayError::from)?;
    Ok(out)
}

/// Replace primary Aadhaar last-4 for an employee. Resets verification.
pub async fn upsert_primary_aadhaar(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    aadhaar_number: String,
) -> KabiPayResult<employee_aadhaar::Model> {
    let last4 = normalize_aadhaar_last4(&aadhaar_number)?;

    let txn = db.begin().await.map_err(KabiPayError::from)?;

    let rows: Vec<employee_aadhaar::Model> = employee_aadhaar::Entity::find()
        .filter(employee_aadhaar::Column::TenantId.eq(tenant_id))
        .filter(employee_aadhaar::Column::EmployeeId.eq(employee_id))
        .all(&txn)
        .await
        .map_err(KabiPayError::from)?;

    let now = Utc::now();

    if rows.is_empty() {
        let id = Uuid::new_v4();
        let am = employee_aadhaar::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            aadhaar_last4: Set(last4),
            is_primary: Set(true),
            is_verified: Set(false),
            verified_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(&txn).await.map_err(KabiPayError::from)?;
        let out = employee_aadhaar::Entity::find_by_id(id)
            .one(&txn)
            .await
            .map_err(KabiPayError::from)?
            .ok_or_else(|| KabiPayError::Internal("inserted Aadhaar row missing".into()))?;
        txn.commit().await.map_err(KabiPayError::from)?;
        return Ok(out);
    }

    let key = rows
        .iter()
        .find(|r| r.is_primary)
        .map(|r| r.id)
        .unwrap_or(rows[0].id);

    let mut out: Option<employee_aadhaar::Model> = None;
    for m in rows {
        let is_target = m.id == key;
        let mut am: employee_aadhaar::ActiveModel = m.into();
        if is_target {
            am.aadhaar_last4 = Set(last4.clone());
            am.is_primary = Set(true);
            am.is_verified = Set(false);
            am.verified_at = Set(None);
        } else {
            am.is_primary = Set(false);
        }
        am.updated_at = Set(now);
        am.update(&txn).await.map_err(KabiPayError::from)?;
        if is_target {
            out = employee_aadhaar::Entity::find_by_id(key)
                .one(&txn)
                .await
                .map_err(KabiPayError::from)?;
        }
    }

    let out = out.ok_or_else(|| KabiPayError::Internal("updated Aadhaar row missing".into()))?;
    txn.commit().await.map_err(KabiPayError::from)?;
    Ok(out)
}
