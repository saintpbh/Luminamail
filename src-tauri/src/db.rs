use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;

use crate::models::*;

pub async fn init_db(app_data_dir: PathBuf) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&app_data_dir)?;
    let db_path = app_data_dir.join("lumina_mail.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // ── Core tables ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_rooms (
            thread_id TEXT PRIMARY KEY,
            subject TEXT NOT NULL,
            assigned_to TEXT,
            status TEXT NOT NULL DEFAULT 'open',
            last_received_at TEXT NOT NULL,
            sender_name TEXT NOT NULL,
            sender_avatar TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            important INTEGER NOT NULL DEFAULT 0,
            unread_count INTEGER NOT NULL DEFAULT 0,
            last_message_preview TEXT,
            is_briefing INTEGER NOT NULL DEFAULT 0
        )"
    ).execute(&pool).await?;

    // Attempt to add is_briefing to existing databases (ignores error if already exists)
    let _ = sqlx::query("ALTER TABLE chat_rooms ADD COLUMN is_briefing INTEGER NOT NULL DEFAULT 0")
        .execute(&pool).await;

    // Attempt to add receiver_account to existing databases
    let _ = sqlx::query("ALTER TABLE chat_rooms ADD COLUMN receiver_account TEXT")
        .execute(&pool).await;

    // Attempt to add hidden_until for snooze feature
    let _ = sqlx::query("ALTER TABLE chat_rooms ADD COLUMN hidden_until TEXT")
        .execute(&pool).await;

    // Attempt to add deleted_at for trash retention feature
    let _ = sqlx::query("ALTER TABLE chat_rooms ADD COLUMN deleted_at TEXT")
        .execute(&pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL,
            message_type TEXT NOT NULL DEFAULT 'email',
            sender_identity TEXT NOT NULL,
            sender_name TEXT,
            body_summary TEXT,
            body_original TEXT,
            icon_type TEXT,
            emoji_tag TEXT,
            hashtags TEXT,
            created_at TEXT NOT NULL,
            is_outgoing INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(thread_id) REFERENCES chat_rooms(thread_id)
        )"
    ).execute(&pool).await?;

    // ── Attachments ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS attachments (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            filename TEXT NOT NULL,
            content_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            local_path TEXT,
            thumbnail_path TEXT,
            main_type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(message_id) REFERENCES messages(id)
        )"
    ).execute(&pool).await?;

    // ── Telegram links ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS telegram_links (
            id TEXT PRIMARY KEY,
            link_code TEXT NOT NULL,
            chat_id INTEGER,
            username TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Email accounts ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS email_accounts (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            email TEXT NOT NULL,
            display_name TEXT,
            imap_host TEXT,
            imap_port INTEGER DEFAULT 993,
            smtp_host TEXT,
            smtp_port INTEGER DEFAULT 587,
            username TEXT,
            password_encrypted TEXT,
            sync_mode TEXT NOT NULL DEFAULT 'readonly',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Sync state columns for incremental IMAP ──
    let _ = sqlx::query("ALTER TABLE email_accounts ADD COLUMN last_uid INTEGER DEFAULT 0")
        .execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_accounts ADD COLUMN uid_validity INTEGER DEFAULT 0")
        .execute(&pool).await;

    // ── Todos ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS todos (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            source_thread_id TEXT,
            source_msg_id TEXT,
            due_date TEXT,
            priority TEXT NOT NULL DEFAULT 'normal',
            completed INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Calendar events ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS calendar_events (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            event_date TEXT NOT NULL,
            event_time TEXT,
            source_thread_id TEXT,
            source_msg_id TEXT,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Cloud tokens (Google Drive / OneDrive) ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cloud_tokens (
            provider TEXT PRIMARY KEY,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            expires_at TEXT,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Mail Groups ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS mail_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            color TEXT NOT NULL DEFAULT '#6366f1',
            member_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS group_members (
            id TEXT PRIMARY KEY,
            group_id TEXT NOT NULL,
            email TEXT NOT NULL,
            display_name TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (group_id) REFERENCES mail_groups(id) ON DELETE CASCADE
        )"
    ).execute(&pool).await?;

    // ── Scheduled / Recurring Emails ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scheduled_emails (
            id TEXT PRIMARY KEY,
            group_id TEXT,
            to_emails TEXT NOT NULL,
            subject TEXT NOT NULL,
            body TEXT NOT NULL,
            schedule_type TEXT NOT NULL DEFAULT 'once',
            schedule_time TEXT NOT NULL,
            recurrence_rule TEXT,
            last_sent_at TEXT,
            next_run_at TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Contact Groups ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS contact_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            sync_source TEXT,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Contacts ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS contacts (
            id TEXT PRIMARY KEY,
            group_id TEXT,
            name TEXT NOT NULL,
            email TEXT,
            phone TEXT,
            company TEXT,
            avatar_url TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(group_id) REFERENCES contact_groups(id) ON DELETE SET NULL
        )"
    ).execute(&pool).await?;

    // ── Thread Memos (private notes per thread) ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS thread_memos (
            id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── Email Signatures ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS email_signatures (
            id TEXT PRIMARY KEY,
            account_id TEXT,
            name TEXT NOT NULL,
            body_html TEXT NOT NULL DEFAULT '',
            is_default INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"
    ).execute(&pool).await?;

    // ── App Settings (key-value) ──
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )"
    ).execute(&pool).await?;

    // ── Phase 14: AI column migrations (idempotent) ──
    let ai_migrations = [
        "ALTER TABLE messages ADD COLUMN ai_summary TEXT",
        "ALTER TABLE messages ADD COLUMN ai_translation TEXT",
        "ALTER TABLE messages ADD COLUMN ai_processed INTEGER DEFAULT 0",
        "ALTER TABLE chat_rooms ADD COLUMN ai_tags TEXT",
        "ALTER TABLE chat_rooms ADD COLUMN needs_action INTEGER DEFAULT 0",
    ];
    for sql in &ai_migrations {
        let _ = sqlx::query(sql).execute(&pool).await; // ignore "duplicate column" errors
    }

    Ok(pool)
}

// ╔═══════════════════════════════════════════════╗
// ║        CHAT ROOMS & MESSAGES                  ║
// ╚═══════════════════════════════════════════════╝

pub async fn get_threads(pool: &SqlitePool, filter: &str) -> Result<Vec<ChatRoom>, sqlx::Error> {
    if filter == "trash" || filter == "spam" {
        let query = format!("
            SELECT thread_id, subject, assigned_to, status, last_received_at, sender_name, sender_avatar, pinned, important, unread_count, last_message_preview, is_briefing, receiver_account, ai_tags, needs_action
            FROM chat_rooms
            WHERE status = '{}'
            ORDER BY last_received_at DESC
        ", filter);
        return sqlx::query_as(&query).fetch_all(pool).await;
    }

    let now = chrono::Local::now().to_rfc3339();
    let mut query = "
        SELECT thread_id, subject, assigned_to, status, last_received_at, sender_name, sender_avatar, pinned, important, unread_count, last_message_preview, is_briefing, receiver_account, ai_tags, needs_action
        FROM chat_rooms 
        WHERE status NOT IN ('deleted', 'spam') 
          AND (hidden_until IS NULL OR hidden_until <= ?)
    ".to_string();

    if filter == "unread" { query.push_str(" AND unread_count > 0"); }
    else if filter == "pinned" { query.push_str(" AND pinned = 1"); }
    else if filter == "important" { query.push_str(" AND important = 1"); }
    else if filter == "needs_action" { query.push_str(" AND needs_action = 1"); }
    else if filter == "media" { query.push_str(" AND thread_id IN (SELECT thread_id FROM messages WHERE id IN (SELECT message_id FROM attachments WHERE main_type = 'image' OR main_type = 'video'))"); }

    query.push_str(" ORDER BY pinned DESC, last_received_at DESC");

    sqlx::query_as(&query).bind(now).fetch_all(pool).await
}

pub async fn get_messages(pool: &SqlitePool, thread_id: &str) -> Result<Vec<Message>, sqlx::Error> {
    let mut messages: Vec<Message> = sqlx::query_as(
        "SELECT id, thread_id, message_type, sender_identity, sender_name, body_summary, body_original, icon_type, emoji_tag, hashtags, created_at, is_outgoing, ai_summary, ai_translation, ai_processed
         FROM messages WHERE thread_id = ? ORDER BY created_at ASC"
    ).bind(thread_id).fetch_all(pool).await?;

    for msg in &mut messages {
        let attachments: Vec<Attachment> = sqlx::query_as(
            "SELECT id, message_id, filename, content_type, size_bytes, local_path, thumbnail_path, main_type, created_at
             FROM attachments WHERE message_id = ? ORDER BY created_at ASC"
        ).bind(&msg.id).fetch_all(pool).await?;
        
        if !attachments.is_empty() {
            msg.attachments = Some(attachments);
        }
    }

    Ok(messages)
}

pub async fn get_thread_by_id(pool: &SqlitePool, thread_id: &str) -> Result<Option<ChatRoom>, sqlx::Error> {
    sqlx::query_as(
        "SELECT thread_id, subject, assigned_to, status, last_received_at, sender_name, sender_avatar, pinned, important, unread_count, last_message_preview, is_briefing, receiver_account, ai_tags, needs_action
         FROM chat_rooms WHERE thread_id = ?"
    ).bind(thread_id).fetch_optional(pool).await
}

pub async fn create_message(pool: &SqlitePool, thread_id: &str, body_text: &str, body_html: Option<&str>, message_type: &str) -> Result<(), sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let msg_type = if message_type == "whisper" { "internal_comment" } else { "email" };

    sqlx::query(
        "INSERT INTO messages (id, thread_id, message_type, sender_identity, sender_name, body_summary, body_original, created_at, is_outgoing)
         VALUES (?, ?, ?, 'me@company.com', '나', ?, ?, ?, 1)"
    ).bind(&id).bind(thread_id).bind(msg_type).bind(body_text).bind(body_html).bind(&now)
    .execute(pool).await?;

    let preview = if body_text.is_empty() { "이미지/서식 메일" } else { body_text };
    sqlx::query("UPDATE chat_rooms SET last_message_preview = ?, last_received_at = ? WHERE thread_id = ?")
        .bind(preview).bind(&now).bind(thread_id).execute(pool).await?;

    Ok(())
}

pub async fn add_thread_tag(pool: &SqlitePool, thread_id: &str, tag: &str) -> Result<(), sqlx::Error> {
    let current: Option<(Option<String>,)> = sqlx::query_as("SELECT ai_tags FROM chat_rooms WHERE thread_id = ?")
        .bind(thread_id).fetch_optional(pool).await?;

    if let Some((tags_opt,)) = current {
        let mut tags: Vec<String> = match tags_opt {
            Some(s) if !s.is_empty() => s.split(',').map(|s| s.trim().to_string()).collect(),
            _ => vec![],
        };

        let normalized_tag = tag.trim().to_string();
        if !tags.contains(&normalized_tag) && !normalized_tag.is_empty() {
            tags.push(normalized_tag);
            let new_tags_str = tags.join(",");
            sqlx::query("UPDATE chat_rooms SET ai_tags = ? WHERE thread_id = ?")
                .bind(new_tags_str).bind(thread_id).execute(pool).await?;
        }
    }
    Ok(())
}

pub async fn get_messages_by_date(pool: &SqlitePool, date: &str) -> Result<Vec<Message>, sqlx::Error> {
    let mut messages: Vec<Message> = sqlx::query_as(
        "SELECT id, thread_id, message_type, sender_identity, sender_name, body_summary, body_original, icon_type, emoji_tag, hashtags, created_at, is_outgoing, ai_summary, ai_translation, ai_processed
         FROM messages WHERE created_at LIKE ? ORDER BY created_at DESC"
    ).bind(format!("{}%", date)).fetch_all(pool).await?;

    for msg in &mut messages {
        let attachments: Vec<Attachment> = sqlx::query_as(
            "SELECT id, message_id, filename, content_type, size_bytes, local_path, thumbnail_path, main_type, created_at
             FROM attachments WHERE message_id = ? ORDER BY created_at ASC"
        ).bind(&msg.id).fetch_all(pool).await?;
        
        if !attachments.is_empty() {
            msg.attachments = Some(attachments);
        }
    }

    Ok(messages)
}

pub async fn insert_attachment(
    pool: &SqlitePool,
    id: &str,
    message_id: &str,
    filename: &str,
    content_type: &str,
    size_bytes: i64,
    local_path: Option<&str>,
    thumbnail_path: Option<&str>,
    main_type: &str,
    created_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO attachments (id, message_id, filename, content_type, size_bytes, local_path, thumbnail_path, main_type, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(message_id).bind(filename).bind(content_type).bind(size_bytes)
    .bind(local_path).bind(thumbnail_path).bind(main_type).bind(created_at)
    .execute(pool).await?;
    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║        TELEGRAM LINKS                         ║
// ╚═══════════════════════════════════════════════╝

pub async fn save_telegram_link(pool: &SqlitePool, code: &str) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    sqlx::query("INSERT INTO telegram_links (id, link_code, status, created_at) VALUES (?, ?, 'pending', ?)")
        .bind(&id).bind(code).bind(&now).execute(pool).await?;

    Ok(id)
}

pub async fn complete_telegram_link(pool: &SqlitePool, code: &str, chat_id: i64, username: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE telegram_links SET chat_id = ?, username = ?, status = 'linked' WHERE link_code = ?")
        .bind(chat_id).bind(username).bind(code).execute(pool).await?;
    Ok(())
}

pub async fn get_telegram_link(pool: &SqlitePool) -> Result<Option<TelegramLink>, sqlx::Error> {
    sqlx::query_as("SELECT id, link_code, chat_id, username, status, created_at FROM telegram_links WHERE status = 'linked' ORDER BY created_at DESC LIMIT 1")
        .fetch_optional(pool).await
}

pub async fn delete_telegram_link(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM telegram_links").execute(pool).await?;
    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║        EMAIL ACCOUNTS                         ║
// ╚═══════════════════════════════════════════════╝

pub async fn get_email_accounts(pool: &SqlitePool) -> Result<Vec<EmailAccount>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, provider, email, display_name, imap_host, imap_port, smtp_host, smtp_port, username, password_encrypted, sync_mode, enabled, created_at
         FROM email_accounts ORDER BY created_at ASC"
    ).fetch_all(pool).await
}

pub async fn add_email_account(pool: &SqlitePool, provider: &str, email: &str, display_name: &str, imap_host: &str, imap_port: i32, smtp_host: &str, smtp_port: i32, username: &str, password: &str, sync_mode: &str) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO email_accounts (id, provider, email, display_name, imap_host, imap_port, smtp_host, smtp_port, username, password_encrypted, sync_mode, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    ).bind(&id).bind(provider).bind(email).bind(display_name).bind(imap_host).bind(imap_port).bind(smtp_host).bind(smtp_port).bind(username).bind(password).bind(sync_mode).bind(&now)
    .execute(pool).await?;

    Ok(id)
}

pub async fn update_email_account_details(pool: &SqlitePool, id: &str, display_name: &str, imap_host: &str, imap_port: i32, smtp_host: &str, smtp_port: i32, password: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE email_accounts SET display_name = ?, imap_host = ?, imap_port = ?, smtp_host = ?, smtp_port = ?, password_encrypted = ? WHERE id = ?"
    ).bind(display_name).bind(imap_host).bind(imap_port).bind(smtp_host).bind(smtp_port).bind(password).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn update_email_sync_mode(pool: &SqlitePool, id: &str, sync_mode: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_accounts SET sync_mode = ? WHERE id = ?")
        .bind(sync_mode).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn delete_email_account(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_accounts WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn update_sync_state(pool: &SqlitePool, account_id: &str, last_uid: u32, uid_validity: u32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_accounts SET last_uid = ?, uid_validity = ? WHERE id = ?")
        .bind(last_uid as i64).bind(uid_validity as i64).bind(account_id).execute(pool).await?;
    Ok(())
}

pub async fn get_sync_state(pool: &SqlitePool, account_id: &str) -> Result<(u32, u32), sqlx::Error> {
    let row: (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(last_uid, 0), COALESCE(uid_validity, 0) FROM email_accounts WHERE id = ?"
    ).bind(account_id).fetch_one(pool).await?;
    Ok((row.0 as u32, row.1 as u32))
}

// ╔═══════════════════════════════════════════════╗
// ║        TODOS                                  ║
// ╚═══════════════════════════════════════════════╝

pub async fn get_todos(pool: &SqlitePool) -> Result<Vec<Todo>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, description, source_thread_id, source_msg_id, due_date, priority, completed, created_at
         FROM todos ORDER BY completed ASC, due_date ASC, created_at DESC"
    ).fetch_all(pool).await
}

pub async fn add_todo(pool: &SqlitePool, title: &str, description: &str, source_thread_id: Option<&str>, source_msg_id: Option<&str>, due_date: Option<&str>, priority: &str) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO todos (id, title, description, source_thread_id, source_msg_id, due_date, priority, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    ).bind(&id).bind(title).bind(description).bind(source_thread_id).bind(source_msg_id).bind(due_date).bind(priority).bind(&now)
    .execute(pool).await?;

    Ok(id)
}

pub async fn toggle_todo(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE todos SET completed = CASE WHEN completed = 0 THEN 1 ELSE 0 END WHERE id = ?")
        .bind(id).execute(pool).await?;
    Ok(())
}

pub async fn delete_todo(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM todos WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║        CALENDAR EVENTS                        ║
// ╚═══════════════════════════════════════════════╝

pub async fn get_calendar_events(pool: &SqlitePool, month: &str) -> Result<Vec<CalendarEvent>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, title, description, event_date, event_time, source_thread_id, source_msg_id, created_at
         FROM calendar_events WHERE event_date LIKE ? ORDER BY event_date ASC, event_time ASC"
    ).bind(format!("{}%", month)).fetch_all(pool).await
}

pub async fn add_calendar_event(pool: &SqlitePool, title: &str, description: &str, event_date: &str, event_time: Option<&str>, source_thread_id: Option<&str>, source_msg_id: Option<&str>) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    sqlx::query(
        "INSERT INTO calendar_events (id, title, description, event_date, event_time, source_thread_id, source_msg_id, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    ).bind(&id).bind(title).bind(description).bind(event_date).bind(event_time).bind(source_thread_id).bind(source_msg_id).bind(&now)
    .execute(pool).await?;

    Ok(id)
}

pub async fn delete_calendar_event(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM calendar_events WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║        EMAIL IMPORT (IMAP → threads)          ║
// ╚═══════════════════════════════════════════════╝

fn truncate_to_chars(text: &str, length: usize) -> String {
    text.chars().take(length).collect()
}

pub async fn import_fetched_email(
    pool: &SqlitePool,
    from_address: &str,
    from_name: &str,
    subject: &str,
    body_text: &str,
    body_html: Option<&str>,
    date: &str,
    message_id: &str,
    is_read: bool,
    receiver_account: &str,
    attachments: Vec<crate::imap_client::FetchedAttachment>,
) -> Result<bool, sqlx::Error> {
    // Check duplicate by message_id
    let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE id = ?")
        .bind(message_id)
        .fetch_one(pool)
        .await?;

    if exists.0 > 0 {
        return Ok(false); // Already imported
    }

    // Group by sender → thread_id
    // Use a hash of the from_address as thread_id for grouping
    let thread_id = format!("imap-{}", simple_hash(from_address));

    // Create or update chat_room
    let room_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chat_rooms WHERE thread_id = ?")
        .bind(&thread_id)
        .fetch_one(pool)
        .await?;

    if room_exists.0 == 0 {
        sqlx::query(
            "INSERT INTO chat_rooms (thread_id, subject, status, last_received_at, sender_name, pinned, important, unread_count, last_message_preview, receiver_account)
             VALUES (?, ?, 'open', ?, ?, 0, 0, 1, ?, ?)"
        )
        .bind(&thread_id)
        .bind(subject)
        .bind(date)
        .bind(from_name)
        .bind(&truncate_to_chars(body_text, 100))
        .bind(receiver_account)
        .execute(pool)
        .await?;
    } else {
        // Update thread with latest message, and unhide it if new message arrives
        let unread_add = if is_read { 0 } else { 1 };
        sqlx::query(
            "UPDATE chat_rooms 
             SET subject = ?, 
                 last_received_at = MAX(last_received_at, ?), 
                 last_message_preview = ?, 
                 unread_count = unread_count + ?,
                 hidden_until = NULL
             WHERE thread_id = ?"
        )
        .bind(subject)
        .bind(date)
        .bind(&truncate_to_chars(body_text, 100))
        .bind(unread_add)
        .bind(&thread_id)
        .execute(pool)
        .await?;
    }

    // Insert message with auto-generated tags
    let tags = crate::auto_tagger::auto_tag_basic(subject, body_text);
    let tags_str = if tags.is_empty() { None } else { Some(tags.join(",")) };

    sqlx::query(
        "INSERT INTO messages (id, thread_id, message_type, sender_identity, sender_name, body_summary, body_original, hashtags, created_at, is_outgoing)
         VALUES (?, ?, 'email', ?, ?, ?, ?, ?, ?, 0)"
    )
    .bind(message_id)
    .bind(&thread_id)
    .bind(from_address)
    .bind(from_name)
    .bind(&truncate_to_chars(body_text, 300))
    .bind(body_html)
    .bind(&tags_str)
    .bind(date)
    .execute(pool)
    .await?;

    // Save attachments to local disk and DB
    if !attachments.is_empty() {
        let base_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("com.bongpark.lumina-mail")
            .join("attachments");

        let _ = std::fs::create_dir_all(&base_dir);

        for att in attachments {
            let att_id = uuid::Uuid::new_v4().to_string();
            let mut ext = std::path::Path::new(&att.filename)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            
            if ext.is_empty() {
                if att.content_type == "image/jpeg" { ext = "jpg"; }
                else if att.content_type == "image/png" { ext = "png"; }
                else if att.content_type == "application/pdf" { ext = "pdf"; }
            }
            
            let safe_filename = format!("{}.{}", att_id, ext);
            let file_path = base_dir.join(&safe_filename);

            let _ = std::fs::write(&file_path, &att.data);

            let mut thumb_path = None;
            let mut main_type = "file".to_string();

            if att.content_type.starts_with("image/") {
                main_type = "image".to_string();
                if let Ok(img) = image::load_from_memory(&att.data) {
                    let thumb = img.thumbnail(200, 200);
                    let t_name = format!("{}_thumb.jpg", att_id);
                    let t_path = base_dir.join(&t_name);
                    if thumb.save(&t_path).is_ok() {
                        thumb_path = Some(t_path.to_string_lossy().to_string());
                    }
                }
            }

            let _ = insert_attachment(
                pool,
                &att_id,
                message_id,
                &att.filename,
                &att.content_type,
                att.size_bytes,
                Some(&file_path.to_string_lossy()),
                thumb_path.as_deref(),
                &main_type,
                date
            ).await;
        }
    }

    Ok(true) // Successfully imported
}

fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub async fn get_email_account_by_id(pool: &SqlitePool, id: &str) -> Result<Option<EmailAccount>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, provider, email, display_name, imap_host, imap_port, smtp_host, smtp_port, username, password_encrypted, sync_mode, enabled, created_at
         FROM email_accounts WHERE id = ?"
    ).bind(id).fetch_optional(pool).await
}

// ╔═══════════════════════════════════════════════╗
// ║        CLOUD TOKENS                           ║
// ╚═══════════════════════════════════════════════╝

pub async fn save_cloud_token(pool: &SqlitePool, provider: &str, access_token: &str, refresh_token: Option<&str>) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    sqlx::query(
        "INSERT OR REPLACE INTO cloud_tokens (provider, access_token, refresh_token, created_at) VALUES (?, ?, ?, ?)"
    ).bind(provider).bind(access_token).bind(refresh_token).bind(&now)
    .execute(pool).await?;
    Ok(())
}

pub async fn get_cloud_token(pool: &SqlitePool, provider: &str) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    let result: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT access_token, refresh_token FROM cloud_tokens WHERE provider = ?"
    ).bind(provider).fetch_optional(pool).await?;
    Ok(result)
}

pub async fn delete_cloud_token(pool: &SqlitePool, provider: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM cloud_tokens WHERE provider = ?").bind(provider).execute(pool).await?;
    Ok(())
}

pub async fn get_all_cloud_providers(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT provider FROM cloud_tokens").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

// ╔═══════════════════════════════════════════════╗
// ║        THREAD ACTIONS (Context Menu)           ║
// ╚═══════════════════════════════════════════════╝

pub async fn toggle_pin(pool: &SqlitePool, thread_id: &str) -> Result<bool, sqlx::Error> {
    let current: Option<(bool,)> = sqlx::query_as("SELECT pinned FROM chat_rooms WHERE thread_id = ?")
        .bind(thread_id).fetch_optional(pool).await?;
    let new_val = !current.map(|c| c.0).unwrap_or(false);
    sqlx::query("UPDATE chat_rooms SET pinned = ? WHERE thread_id = ?")
        .bind(new_val).bind(thread_id).execute(pool).await?;
    Ok(new_val)
}

pub async fn toggle_important(pool: &SqlitePool, thread_id: &str) -> Result<bool, sqlx::Error> {
    let current: Option<(bool,)> = sqlx::query_as("SELECT important FROM chat_rooms WHERE thread_id = ?")
        .bind(thread_id).fetch_optional(pool).await?;
    let new_val = !current.map(|c| c.0).unwrap_or(false);
    sqlx::query("UPDATE chat_rooms SET important = ? WHERE thread_id = ?")
        .bind(new_val).bind(thread_id).execute(pool).await?;
    Ok(new_val)
}

pub async fn toggle_unread(pool: &SqlitePool, thread_id: &str) -> Result<bool, sqlx::Error> {
    let current: Option<(i64,)> = sqlx::query_as("SELECT unread_count FROM chat_rooms WHERE thread_id = ?")
        .bind(thread_id).fetch_optional(pool).await?;
    let is_unread = current.map(|c| c.0 > 0).unwrap_or(false);
    let new_val = if is_unread { 0 } else { 1 };
    sqlx::query("UPDATE chat_rooms SET unread_count = ? WHERE thread_id = ?")
        .bind(new_val).bind(thread_id).execute(pool).await?;
    Ok(!is_unread)
}

pub async fn delete_thread(pool: &SqlitePool, thread_id: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    // Soft delete to prevent IMAP from re-fetching it
    sqlx::query("UPDATE chat_rooms SET status = 'deleted', deleted_at = ? WHERE thread_id = ?")
        .bind(now).bind(thread_id).execute(pool).await?;
    Ok(())
}

pub async fn restore_thread(pool: &SqlitePool, thread_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_rooms SET status = 'open', deleted_at = NULL WHERE thread_id = ?")
        .bind(thread_id).execute(pool).await?;
    Ok(())
}

pub async fn spam_thread(pool: &SqlitePool, thread_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_rooms SET status = 'spam' WHERE thread_id = ?").bind(thread_id).execute(pool).await?;
    Ok(())
}

pub async fn snooze_thread(pool: &SqlitePool, thread_id: &str, months: i64) -> Result<(), sqlx::Error> {
    use chrono::Utc;
    // Calculate future date roughly based on 30.44 days per month
    let future_date = Utc::now() + chrono::Duration::days(30 * months);
    let date_str = future_date.to_rfc3339();
    sqlx::query("UPDATE chat_rooms SET hidden_until = ? WHERE thread_id = ?")
        .bind(date_str).bind(thread_id).execute(pool).await?;
    Ok(())
}

pub async fn auto_empty_trash(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    // Manually delete related attachments and messages due to missing PRAGMA ON DELETE CASCADE
    sqlx::query("DELETE FROM attachments WHERE message_id IN (
        SELECT id FROM messages WHERE thread_id IN (
            SELECT thread_id FROM chat_rooms WHERE status = 'deleted' AND datetime(deleted_at) < datetime('now', '-3 months')
        )
    )").execute(pool).await?;

    sqlx::query("DELETE FROM messages WHERE thread_id IN (
        SELECT thread_id FROM chat_rooms WHERE status = 'deleted' AND datetime(deleted_at) < datetime('now', '-3 months')
    )").execute(pool).await?;

    let res = sqlx::query("DELETE FROM chat_rooms WHERE status = 'deleted' AND datetime(deleted_at) < datetime('now', '-3 months')")
        .execute(pool).await?;
        
    Ok(res.rows_affected())
}

// ╔═══════════════════════════════════════════════╗
// ║        MAIL GROUPS                            ║
// ╚═══════════════════════════════════════════════╝

use crate::models::{MailGroup, GroupMember, ScheduledEmail};

pub async fn create_group(pool: &SqlitePool, name: &str, description: Option<&str>, color: &str) -> Result<MailGroup, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO mail_groups (id, name, description, color, member_count, created_at) VALUES (?,?,?,?,0,?)"
    ).bind(&id).bind(name).bind(description).bind(color).bind(&now).execute(pool).await?;
    Ok(MailGroup { id, name: name.to_string(), description: description.map(|s| s.to_string()), color: color.to_string(), member_count: 0, created_at: now })
}

pub async fn get_groups(pool: &SqlitePool) -> Result<Vec<MailGroup>, sqlx::Error> {
    sqlx::query_as("SELECT id, name, description, color, member_count, created_at FROM mail_groups ORDER BY name").fetch_all(pool).await
}

pub async fn update_group(pool: &SqlitePool, id: &str, name: &str, description: Option<&str>, color: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE mail_groups SET name=?, description=?, color=? WHERE id=?")
        .bind(name).bind(description).bind(color).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn delete_group(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM group_members WHERE group_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM scheduled_emails WHERE group_id = ?").bind(id).execute(pool).await?;
    sqlx::query("DELETE FROM mail_groups WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn add_group_member(pool: &SqlitePool, group_id: &str, email: &str, display_name: Option<&str>) -> Result<GroupMember, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    sqlx::query("INSERT INTO group_members (id, group_id, email, display_name, created_at) VALUES (?,?,?,?,?)")
        .bind(&id).bind(group_id).bind(email).bind(display_name).bind(&now).execute(pool).await?;
    sqlx::query("UPDATE mail_groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = ?) WHERE id = ?")
        .bind(group_id).bind(group_id).execute(pool).await?;
    Ok(GroupMember { id, group_id: group_id.to_string(), email: email.to_string(), display_name: display_name.map(|s| s.to_string()), created_at: now })
}

pub async fn remove_group_member(pool: &SqlitePool, member_id: &str) -> Result<(), sqlx::Error> {
    let member: Option<(String,)> = sqlx::query_as("SELECT group_id FROM group_members WHERE id = ?").bind(member_id).fetch_optional(pool).await?;
    sqlx::query("DELETE FROM group_members WHERE id = ?").bind(member_id).execute(pool).await?;
    if let Some((gid,)) = member {
        sqlx::query("UPDATE mail_groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = ?) WHERE id = ?")
            .bind(&gid).bind(&gid).execute(pool).await?;
    }
    Ok(())
}

pub async fn get_group_members(pool: &SqlitePool, group_id: &str) -> Result<Vec<GroupMember>, sqlx::Error> {
    sqlx::query_as("SELECT id, group_id, email, display_name, created_at FROM group_members WHERE group_id = ? ORDER BY display_name, email")
        .bind(group_id).fetch_all(pool).await
}

// ╔═══════════════════════════════════════════════╗
// ║        SCHEDULED EMAILS                       ║
// ╚═══════════════════════════════════════════════╝

pub async fn create_scheduled_email(
    pool: &SqlitePool, group_id: Option<&str>, to_emails: &str, subject: &str, body: &str,
    schedule_type: &str, schedule_time: &str, recurrence_rule: Option<&str>
) -> Result<ScheduledEmail, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO scheduled_emails (id, group_id, to_emails, subject, body, schedule_type, schedule_time, recurrence_rule, next_run_at, enabled, created_at) VALUES (?,?,?,?,?,?,?,?,?,1,?)"
    ).bind(&id).bind(group_id).bind(to_emails).bind(subject).bind(body)
     .bind(schedule_type).bind(schedule_time).bind(recurrence_rule).bind(schedule_time).bind(&now)
     .execute(pool).await?;
    Ok(ScheduledEmail {
        id, group_id: group_id.map(|s| s.to_string()), to_emails: to_emails.to_string(),
        subject: subject.to_string(), body: body.to_string(), schedule_type: schedule_type.to_string(),
        schedule_time: schedule_time.to_string(), recurrence_rule: recurrence_rule.map(|s| s.to_string()),
        last_sent_at: None, next_run_at: Some(schedule_time.to_string()), enabled: true, created_at: now,
    })
}

pub async fn get_scheduled_emails(pool: &SqlitePool) -> Result<Vec<ScheduledEmail>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, group_id, to_emails, subject, body, schedule_type, schedule_time, recurrence_rule, last_sent_at, next_run_at, enabled, created_at FROM scheduled_emails ORDER BY next_run_at"
    ).fetch_all(pool).await
}

pub async fn delete_scheduled_email(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM scheduled_emails WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn toggle_scheduled_email(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
    let current: Option<(bool,)> = sqlx::query_as("SELECT enabled FROM scheduled_emails WHERE id = ?")
        .bind(id).fetch_optional(pool).await?;
    let new_val = !current.map(|c| c.0).unwrap_or(false);
    sqlx::query("UPDATE scheduled_emails SET enabled = ? WHERE id = ?").bind(new_val).bind(id).execute(pool).await?;
    Ok(new_val)
}

// ╔═══════════════════════════════════════════════╗
// ║        CONTACTS & ADDRESS BOOK                ║
// ╚═══════════════════════════════════════════════╝

pub async fn toggle_briefing(pool: &SqlitePool, thread_id: &str, is_briefing: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE chat_rooms SET is_briefing = ? WHERE thread_id = ?")
        .bind(is_briefing as i32)
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_contact_groups(pool: &SqlitePool) -> Result<Vec<ContactGroup>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM contact_groups ORDER BY name ASC")
        .fetch_all(pool)
        .await
}

pub async fn get_contacts(pool: &SqlitePool) -> Result<Vec<Contact>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM contacts ORDER BY name ASC")
        .fetch_all(pool)
        .await
}

pub async fn sync_mock_contacts(pool: &SqlitePool, provider: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let group_id = uuid::Uuid::new_v4().to_string();
    let group_name = if provider == "google" { "Google Contacts" } else { "Apple iCloud Contacts" };

    // Create a group
    sqlx::query("INSERT OR REPLACE INTO contact_groups (id, name, sync_source, created_at) VALUES (?, ?, ?, ?)")
        .bind(&group_id).bind(group_name).bind(provider).bind(&now)
        .execute(pool).await?;

    // Create some mock contacts
    let mocks = if provider == "google" {
        vec![
            ("이재무", "재무팀", "jaemu@example.com", "010-1234-5678"),
            ("김선교", "기획팀", "sunkyo@example.com", "010-2345-6789"),
            ("박영업", "영업팀", "youngup@example.com", "010-3456-7890"),
        ]
    } else {
        vec![
            ("최디자인", "디자인팀", "choi@example.com", "010-9876-5432"),
            ("정개발", "개발팀", "jung@example.com", "010-8765-4321"),
            ("홍마케팅", "마케팅팀", "hong@example.com", "010-1111-2222"),
        ]
    };

    for (name, company, email, phone) in mocks {
        let contact_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT OR REPLACE INTO contacts (id, group_id, name, email, phone, company, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&contact_id).bind(&group_id).bind(name).bind(email).bind(phone).bind(company).bind(&now)
            .execute(pool).await?;
    }
    Ok(())
}

/// One-time migration: re-strip body_summary from body_original using the fixed regex HTML stripper.
/// This cleans up CSS code that leaked into previews due to the old character-based parser.
pub async fn cleanup_css_previews(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 1. Fix messages: re-compute body_summary from body_original
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, body_original FROM messages WHERE body_original IS NOT NULL AND body_original != ''"
    ).fetch_all(pool).await?;

    for (msg_id, html) in &rows {
        let clean = crate::imap_client::strip_html_tags(html);
        let truncated = truncate_to_chars(&clean, 300);
        sqlx::query("UPDATE messages SET body_summary = ? WHERE id = ?")
            .bind(&truncated)
            .bind(msg_id)
            .execute(pool)
            .await?;
    }

    // 2. Fix chat_rooms: update last_message_preview from the latest message's clean body_summary
    let threads: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT thread_id FROM chat_rooms"
    ).fetch_all(pool).await?;

    for (tid,) in &threads {
        let latest: Option<(String,)> = sqlx::query_as(
            "SELECT body_summary FROM messages WHERE thread_id = ? ORDER BY created_at DESC LIMIT 1"
        ).bind(tid).fetch_optional(pool).await?;

        if let Some((summary,)) = latest {
            let preview = truncate_to_chars(&summary, 100);
            sqlx::query("UPDATE chat_rooms SET last_message_preview = ? WHERE thread_id = ?")
                .bind(&preview)
                .bind(tid)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║     THREAD MEMOS (private notes)              ║
// ╚═══════════════════════════════════════════════╝

pub async fn get_memo(pool: &SqlitePool, thread_id: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT content FROM thread_memos WHERE thread_id = ?"
    ).bind(thread_id).fetch_optional(pool).await?;
    Ok(row.map(|(c,)| c))
}

pub async fn save_memo(pool: &SqlitePool, thread_id: &str, content: &str) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO thread_memos (id, thread_id, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(thread_id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at"
    ).bind(&id).bind(thread_id).bind(content).bind(&now).bind(&now)
    .execute(pool).await?;
    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║     EMAIL SIGNATURES                          ║
// ╚═══════════════════════════════════════════════╝

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct EmailSignature {
    pub id: String,
    pub account_id: Option<String>,
    pub name: String,
    pub body_html: String,
    pub is_default: i32,
    pub created_at: String,
}

pub async fn get_signatures(pool: &SqlitePool) -> Result<Vec<EmailSignature>, sqlx::Error> {
    sqlx::query_as::<_, EmailSignature>(
        "SELECT id, account_id, name, body_html, is_default, created_at FROM email_signatures ORDER BY is_default DESC, created_at DESC"
    ).fetch_all(pool).await
}

pub async fn save_signature(pool: &SqlitePool, id: Option<&str>, name: &str, body_html: &str, is_default: bool) -> Result<String, sqlx::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let sig_id = id.map(|s| s.to_string()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    if is_default {
        sqlx::query("UPDATE email_signatures SET is_default = 0").execute(pool).await?;
    }
    
    sqlx::query(
        "INSERT INTO email_signatures (id, name, body_html, is_default, created_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, body_html = excluded.body_html, is_default = excluded.is_default"
    ).bind(&sig_id).bind(name).bind(body_html).bind(is_default as i32).bind(&now)
    .execute(pool).await?;
    Ok(sig_id)
}

pub async fn delete_signature(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM email_signatures WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║     CONTACT SYNC FROM MESSAGES                ║
// ╚═══════════════════════════════════════════════╝

pub async fn sync_contacts_from_messages(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    
    // Create or get the "메일 연락처" group
    let group_id: String = {
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM contact_groups WHERE name = '메일 연락처'"
        ).fetch_optional(pool).await?;
        if let Some((id,)) = existing {
            id
        } else {
            let gid = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO contact_groups (id, name, sync_source, created_at) VALUES (?, '메일 연락처', 'email', ?)")
                .bind(&gid).bind(&now).execute(pool).await?;
            gid
        }
    };

    // Get all unique sender emails from messages (excluding outgoing)
    let senders: Vec<(String, String)> = sqlx::query_as(
        "SELECT DISTINCT sender_identity, sender_name FROM messages WHERE is_outgoing = 0 AND sender_identity IS NOT NULL AND sender_identity != ''"
    ).fetch_all(pool).await?;

    let mut count: i64 = 0;
    for (email, name) in &senders {
        let exists: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM contacts WHERE email = ?"
        ).bind(email).fetch_one(pool).await?;
        
        if exists.0 == 0 {
            let cid = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO contacts (id, group_id, name, email, created_at) VALUES (?, ?, ?, ?, ?)"
            ).bind(&cid).bind(&group_id).bind(name).bind(email).bind(&now)
            .execute(pool).await?;
            count += 1;
        }
    }
    Ok(count)
}

pub async fn get_default_email_account(pool: &SqlitePool) -> Result<Option<EmailAccount>, sqlx::Error> {
    sqlx::query_as::<_, EmailAccount>(
        "SELECT id, provider, email, display_name, imap_host, imap_port, smtp_host, smtp_port, username, password_encrypted, sync_mode, enabled, created_at
         FROM email_accounts WHERE enabled = 1 ORDER BY created_at ASC LIMIT 1"
    ).fetch_all(pool).await.map(|v| v.into_iter().next())
}

