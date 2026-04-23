//! GraphQL DTOs for kabipay-assets.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::tenant::d0022_assets::{asset, asset_category};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "AssetCategory")]
pub struct AssetCategoryDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<asset_category::Model> for AssetCategoryDto {
    fn from(m: asset_category::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Asset")]
pub struct AssetDto {
    pub id: ID,
    pub tenant_id: ID,
    pub asset_category_id: ID,
    pub name: String,
    pub serial_number: Option<String>,
    pub asset_tag: Option<String>,
    pub purchase_value: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub status: String,
}

impl From<asset::Model> for AssetDto {
    fn from(m: asset::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            asset_category_id: ID(m.asset_category_id.to_string()),
            name: m.name,
            serial_number: m.serial_number,
            asset_tag: m.asset_tag,
            purchase_value: m.purchase_value.map(|d| d.to_string()),
            purchase_date: m.purchase_date,
            status: m.status,
        }
    }
}
