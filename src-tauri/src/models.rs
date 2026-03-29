use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatRoom {
    pub thread_id: String,
    pub subject: String,
    pub assigned_to: Option<String>,
    pub status: String,
    pub last_received_at: String,
    pub sender_name: String,
    pub sender_avatar: Option<String>,
    pub pinned: bool,
    pub important: bool,
    pub unread_count: i64,
    pub last_message_preview: Option<String>,
    pub is_briefing: bool,
    pub ai_tags: Option<String>,
    pub needs_action: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContactGroup {
    pub id: String,
    pub name: String,
    pub sync_source: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Contact {
    pub id: String,
    pub group_id: Option<String>,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub message_type: String,
    pub sender_identity: String,
    pub sender_name: Option<String>,
    pub body_summary: Option<String>,
    pub body_original: Option<String>,
    pub icon_type: Option<String>,
    pub emoji_tag: Option<String>,
    pub hashtags: Option<String>,
    pub created_at: String,
    pub is_outgoing: bool,
    pub ai_summary: Option<String>,
    pub ai_translation: Option<String>,
    pub ai_processed: Option<bool>,
    #[sqlx(skip)] // Handled separately
    pub attachments: Option<Vec<Attachment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TelegramLink {
    pub id: String,
    pub link_code: String,
    pub chat_id: Option<i64>,
    pub username: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailAccount {
    pub id: String,
    pub provider: String,
    pub email: String,
    pub display_name: Option<String>,
    pub imap_host: Option<String>,
    pub imap_port: Option<i32>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub username: Option<String>,
    pub password_encrypted: Option<String>,
    pub sync_mode: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub source_thread_id: Option<String>,
    pub source_msg_id: Option<String>,
    pub due_date: Option<String>,
    pub priority: String,
    pub completed: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub event_date: String,
    pub event_time: Option<String>,
    pub source_thread_id: Option<String>,
    pub source_msg_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MailGroup {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub member_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub id: String,
    pub group_id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ScheduledEmail {
    pub id: String,
    pub group_id: Option<String>,
    pub to_emails: String,
    pub subject: String,
    pub body: String,
    pub schedule_type: String,
    pub schedule_time: String,
    pub recurrence_rule: Option<String>,
    pub last_sent_at: Option<String>,
    pub next_run_at: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Attachment {
    pub id: String,
    pub message_id: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub local_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub main_type: String, // e.g. "image", "application", "video"
    pub created_at: String,
}
