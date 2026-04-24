//! Tenant `attendance_punch_policy`: geofence + IP allowlist for `punchToday`.

use std::net::IpAddr;
use std::str::FromStr;

use chrono::Utc;
use ipnet::IpNet;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0032_attendance_punch_policy::attendance_punch_policy;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

pub async fn find_punch_policy(
    db: &DatabaseConnection,
    tenant_id: Uuid,
) -> KabiPayResult<Option<attendance_punch_policy::Model>> {
    attendance_punch_policy::Entity::find()
        .filter(attendance_punch_policy::Column::TenantId.eq(tenant_id))
        .one(db)
        .await
        .map_err(KabiPayError::from)
}

fn policy_geofence_active(p: &attendance_punch_policy::Model) -> bool {
    if !p.is_enforced {
        return false;
    }
    let Some(max) = p.max_distance_meters else {
        return false;
    };
    if max <= 0 {
        return false;
    }
    p.site_latitude.is_some() && p.site_longitude.is_some()
}

fn policy_ip_active(p: &attendance_punch_policy::Model) -> bool {
    if !p.is_enforced {
        return false;
    }
    p.ip_allowlist
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty())
}

pub fn validate_upsert_punch_policy_input(
    is_enforced: bool,
    site_latitude: Option<f64>,
    site_longitude: Option<f64>,
    max_distance_meters: Option<i32>,
    ip_allowlist: Option<&str>,
) -> KabiPayResult<()> {
    let parts = (
        site_latitude.is_some(),
        site_longitude.is_some(),
        max_distance_meters.is_some(),
    );
    let n = parts.0 as u8 + parts.1 as u8 + parts.2 as u8;
    if n > 0 && n < 3 {
        return Err(KabiPayError::Validation(
            "siteLatitude, siteLongitude, and maxDistanceMeters must all be set together".into(),
        ));
    }
    if let Some(m) = max_distance_meters {
        if m <= 0 {
            return Err(KabiPayError::Validation(
                "maxDistanceMeters must be positive when set".into(),
            ));
        }
    }
    if let (Some(lat), Some(lon)) = (site_latitude, site_longitude) {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(KabiPayError::Validation(
                "siteLatitude must be between -90 and 90".into(),
            ));
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(KabiPayError::Validation(
                "siteLongitude must be between -180 and 180".into(),
            ));
        }
    }
    if is_enforced {
        let geo_ok = site_latitude.is_some()
            && site_longitude.is_some()
            && max_distance_meters.is_some_and(|m| m > 0);
        let ip_ok = ip_allowlist.is_some_and(|s| !s.trim().is_empty());
        if !geo_ok && !ip_ok {
            return Err(KabiPayError::Validation(
                "when isEnforced is true, configure a geofence (site coordinates + max distance) and/or a non-empty ipAllowlist"
                    .into(),
            ));
        }
    }
    Ok(())
}

pub fn client_ip_allowed(allowlist: &str, client_ip: &str) -> bool {
    let Ok(ip) = client_ip.trim().parse::<IpAddr>() else {
        return false;
    };
    for raw in allowlist.split(',') {
        let part = raw.trim();
        if part.is_empty() {
            continue;
        }
        if let Ok(net) = part.parse::<IpNet>() {
            if net.contains(&ip) {
                return true;
            }
        } else if let Ok(addr) = part.parse::<IpAddr>() {
            if addr == ip {
                return true;
            }
        }
    }
    false
}

fn haversine_distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_M: f64 = 6_371_000.0;
    let phi1 = lat1.to_radians();
    let phi2 = lat2.to_radians();
    let dphi = (lat2 - lat1).to_radians();
    let dlambda = (lon2 - lon1).to_radians();
    let sin_dphi = (dphi / 2.0).sin();
    let sin_dlambda = (dlambda / 2.0).sin();
    let a = sin_dphi * sin_dphi + phi1.cos() * phi2.cos() * sin_dlambda * sin_dlambda;
    let c = 2.0 * a.sqrt().atan2((1.0_f64 - a).max(0.0).sqrt());
    EARTH_RADIUS_M * c
}

/// Validates a live punch against the tenant policy row, if any.
pub fn validate_live_punch_for_policy(
    policy: Option<&attendance_punch_policy::Model>,
    punch_lat_lng: Option<(Decimal, Decimal)>,
    client_ip: Option<&str>,
) -> KabiPayResult<()> {
    let Some(p) = policy else {
        return Ok(());
    };
    if !p.is_enforced {
        return Ok(());
    }
    let geo_on = policy_geofence_active(p);
    let ip_on = policy_ip_active(p);
    if !geo_on && !ip_on {
        return Ok(());
    }

    if geo_on {
        let Some((plat, plng)) = punch_lat_lng else {
            return Err(KabiPayError::Validation(
                "attendance punch policy requires GPS coordinates for this punch; none were supplied — event rejected (audit)"
                    .into(),
            ));
        };
        let site_lat = p
            .site_latitude
            .as_ref()
            .and_then(|d| d.to_f64())
            .ok_or_else(|| KabiPayError::Internal("punch policy site latitude missing".into()))?;
        let site_lon = p
            .site_longitude
            .as_ref()
            .and_then(|d| d.to_f64())
            .ok_or_else(|| KabiPayError::Internal("punch policy site longitude missing".into()))?;
        let max_m = p.max_distance_meters.unwrap_or(0) as f64;
        let p_lat = plat.to_f64().unwrap_or(f64::NAN);
        let p_lng = plng.to_f64().unwrap_or(f64::NAN);
        let dist = haversine_distance_m(site_lat, site_lon, p_lat, p_lng);
        if dist > max_m {
            return Err(KabiPayError::Validation(format!(
                "punch location is {:.0} m from the configured site (limit {:.0} m) — attendance punch policy enforcement (audit)",
                dist, max_m
            )));
        }
    }

    if ip_on {
        let list = p.ip_allowlist.as_deref().unwrap_or("");
        let Some(ip) = client_ip.filter(|s| !s.trim().is_empty()) else {
            return Err(KabiPayError::Validation(
                "attendance punch policy uses an IP allowlist but the request had no trusted client IP (configure X-Forwarded-For / X-Real-IP) — event rejected (audit)"
                    .into(),
            ));
        };
        if !client_ip_allowed(list, ip) {
            return Err(KabiPayError::Validation(format!(
                "client IP {ip} is not permitted by the attendance punch IP allowlist — event rejected (audit)"
            )));
        }
    }

    Ok(())
}

pub async fn upsert_punch_policy(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    is_enforced: bool,
    site_latitude: Option<f64>,
    site_longitude: Option<f64>,
    max_distance_meters: Option<i32>,
    ip_allowlist: Option<String>,
) -> KabiPayResult<attendance_punch_policy::Model> {
    validate_upsert_punch_policy_input(
        is_enforced,
        site_latitude,
        site_longitude,
        max_distance_meters,
        ip_allowlist.as_deref(),
    )?;

    let site_lat_d = site_latitude
        .map(|f| Decimal::from_str(&format!("{f:.7}")))
        .transpose()
        .map_err(|_| KabiPayError::Validation("invalid site latitude".into()))?;
    let site_lon_d = site_longitude
        .map(|f| Decimal::from_str(&format!("{f:.7}")))
        .transpose()
        .map_err(|_| KabiPayError::Validation("invalid site longitude".into()))?;

    let ip_stored = ip_allowlist.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });

    let now = Utc::now();
    if let Some(existing) = find_punch_policy(db, tenant_id).await? {
        let id = existing.id;
        let mut am: attendance_punch_policy::ActiveModel = existing.into();
        am.is_enforced = Set(is_enforced);
        am.site_latitude = Set(site_lat_d);
        am.site_longitude = Set(site_lon_d);
        am.max_distance_meters = Set(max_distance_meters);
        am.ip_allowlist = Set(ip_stored);
        am.updated_at = Set(now);
        am.update(db).await?;
        return attendance_punch_policy::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::Internal("punch policy row missing after update".into()));
    }

    let id = Uuid::new_v4();
    let am = attendance_punch_policy::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        is_enforced: Set(is_enforced),
        site_latitude: Set(site_lat_d),
        site_longitude: Set(site_lon_d),
        max_distance_meters: Set(max_distance_meters),
        ip_allowlist: Set(ip_stored),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    attendance_punch_policy::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("punch policy row missing after insert".into()))
}
