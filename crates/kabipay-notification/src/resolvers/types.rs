//! GraphQL DTOs for kabipay-notification.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::tenant::d0027_communication_audit::{announcement, notification};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Announcement")]
pub struct AnnouncementDto {
    pub id: ID,
    pub tenant_id: ID,
    pub created_by: Option<ID>,
    pub title: String,
    pub body: Option<String>,
    pub target_audience: Option<String>,
    pub publish_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<announcement::Model> for AnnouncementDto {
    fn from(m: announcement::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            created_by: m.created_by.map(|u| ID(u.to_string())),
            title: m.title,
            body: m.body,
            target_audience: m.target_audience,
            publish_at: m.publish_at,
            expires_at: m.expires_at,
            created_at: m.created_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Notification")]
pub struct NotificationDto {
    pub id: ID,
    pub tenant_id: ID,
    pub user_id: ID,
    #[graphql(name = "kind")]
    pub kind: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
    pub action_url: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<notification::Model> for NotificationDto {
    fn from(m: notification::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            user_id: ID(m.user_id.to_string()),
            kind: m.r#type,
            title: m.title,
            message: m.message,
            action_url: m.action_url,
            is_read: m.is_read,
            read_at: m.read_at,
            created_at: m.created_at,
        }
    }
}
