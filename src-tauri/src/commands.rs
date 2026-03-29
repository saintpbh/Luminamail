use sqlx::SqlitePool;
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db;
use crate::models::*;
use crate::telegram;

pub struct AppState {
    pub db: Arc<Mutex<SqlitePool>>,
}

// ╔═══════════════════════════════════════════════╗
// ║   THREADS & MESSAGES                          ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn get_threads(state: State<'_, AppState>, filter: String) -> Result<Vec<ChatRoom>, String> {
    let pool = state.db.lock().await;
    db::get_threads(&pool, &filter).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_messages(state: State<'_, AppState>, thread_id: String) -> Result<Vec<Message>, String> {
    let pool = state.db.lock().await;
    db::get_messages(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_thread_detail(state: State<'_, AppState>, thread_id: String) -> Result<Option<ChatRoom>, String> {
    let pool = state.db.lock().await;
    db::get_thread_by_id(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_thread_read(state: State<'_, AppState>, thread_id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    sqlx::query("UPDATE chat_rooms SET unread_count = 0 WHERE thread_id = ?")
        .bind(&thread_id).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn send_reply(state: State<'_, AppState>, thread_id: String, body_text: String, body_html: Option<String>, message_type: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::create_message(&pool, &thread_id, &body_text, body_html.as_deref(), &message_type).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_messages_by_date(state: State<'_, AppState>, date: String) -> Result<Vec<Message>, String> {
    let pool = state.db.lock().await;
    db::get_messages_by_date(&pool, &date).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   TELEGRAM                                    ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn telegram_start_linking(state: State<'_, AppState>) -> Result<String, String> {
    let pool = state.db.lock().await;
    // Delete any previous pending links
    db::delete_telegram_link(&pool).await.map_err(|e| e.to_string())?;
    // Generate new code
    let code = telegram::generate_link_code();
    db::save_telegram_link(&pool, &code).await.map_err(|e| e.to_string())?;
    Ok(code)
}

#[tauri::command]
pub async fn telegram_poll_link(state: State<'_, AppState>, code: String) -> Result<Option<String>, String> {
    // Get latest offset, then poll for code match
    let offset = telegram::get_latest_update_id().await.unwrap_or(0);
    let result = telegram::check_for_link_code(&code, offset - 1).await?;

    if let Some((chat_id, username, _update_id)) = result {
        let pool = state.db.lock().await;
        db::complete_telegram_link(&pool, &code, chat_id, &username).await.map_err(|e| e.to_string())?;
        Ok(Some(username))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn telegram_get_status(state: State<'_, AppState>) -> Result<Option<TelegramLink>, String> {
    let pool = state.db.lock().await;
    db::get_telegram_link(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn telegram_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_telegram_link(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn telegram_send_test(state: State<'_, AppState>) -> Result<(), String> {
    let pool = state.db.lock().await;
    let link = db::get_telegram_link(&pool).await.map_err(|e| e.to_string())?;
    if let Some(link) = link {
        if let Some(chat_id) = link.chat_id {
            telegram::send_message(chat_id, "🔔 <b>Lumina Mail 테스트</b>\n\n테스트 알림이 성공적으로 전송되었습니다!", None).await?;
            Ok(())
        } else {
            Err("Telegram이 연결되지 않았습니다.".into())
        }
    } else {
        Err("Telegram이 연결되지 않았습니다.".into())
    }
}

// ╔═══════════════════════════════════════════════╗
// ║   EMAIL ACCOUNTS                              ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn get_email_accounts(state: State<'_, AppState>) -> Result<Vec<EmailAccount>, String> {
    let pool = state.db.lock().await;
    db::get_email_accounts(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_email_account(
    state: State<'_, AppState>,
    provider: String, email: String, display_name: String,
    imap_host: String, imap_port: i32, smtp_host: String, smtp_port: i32,
    username: String, password: String, sync_mode: String,
) -> Result<String, String> {
    let pool = state.db.lock().await;
    db::add_email_account(&pool, &provider, &email, &display_name, &imap_host, imap_port, &smtp_host, smtp_port, &username, &password, &sync_mode)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_email_account_details(
    state: State<'_, AppState>,
    id: String, display_name: String,
    imap_host: String, imap_port: i32, smtp_host: String, smtp_port: i32,
    password: String,
) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::update_email_account_details(&pool, &id, &display_name, &imap_host, imap_port, &smtp_host, smtp_port, &password)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_email_sync_mode(state: State<'_, AppState>, id: String, sync_mode: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::update_email_sync_mode(&pool, &id, &sync_mode).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_email_account(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_email_account(&pool, &id).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   TODOS                                       ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn get_todos(state: State<'_, AppState>) -> Result<Vec<Todo>, String> {
    let pool = state.db.lock().await;
    db::get_todos(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_todo(
    state: State<'_, AppState>,
    title: String, description: String,
    source_thread_id: Option<String>, source_msg_id: Option<String>,
    due_date: Option<String>, priority: String,
) -> Result<String, String> {
    let pool = state.db.lock().await;
    db::add_todo(&pool, &title, &description, source_thread_id.as_deref(), source_msg_id.as_deref(), due_date.as_deref(), &priority)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_todo(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::toggle_todo(&pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_todo(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_todo(&pool, &id).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   CALENDAR EVENTS                             ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn get_calendar_events(state: State<'_, AppState>, month: String) -> Result<Vec<CalendarEvent>, String> {
    let pool = state.db.lock().await;
    db::get_calendar_events(&pool, &month).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_calendar_event(
    state: State<'_, AppState>,
    title: String, description: String, event_date: String,
    event_time: Option<String>, source_thread_id: Option<String>, source_msg_id: Option<String>,
) -> Result<String, String> {
    let pool = state.db.lock().await;
    db::add_calendar_event(&pool, &title, &description, &event_date, event_time.as_deref(), source_thread_id.as_deref(), source_msg_id.as_deref())
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_calendar_event(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_calendar_event(&pool, &id).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   EMAIL SYNC (IMAP)                           ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn test_email_connection(state: State<'_, AppState>, account_id: String) -> Result<String, String> {
    let pool = state.db.lock().await;
    let account = db::get_email_account_by_id(&pool, &account_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "계정을 찾을 수 없습니다".to_string())?;
    drop(pool); // Release lock before network call

    crate::imap_client::test_connection(&account).await
}

#[tauri::command]
pub async fn sync_email_account(app_handle: tauri::AppHandle, state: State<'_, AppState>, account_id: String) -> Result<String, String> {
    let pool = state.db.lock().await;
    let account = db::get_email_account_by_id(&pool, &account_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "계정을 찾을 수 없습니다".to_string())?;

    // Get stored sync state
    let (stored_last_uid, stored_uid_validity) = db::get_sync_state(&pool, &account_id)
        .await
        .unwrap_or((0, 0));
    drop(pool); // Release lock before network call

    // Use incremental sync if we have a last_uid, otherwise do initial full fetch
    if stored_last_uid > 0 {
        // ── Incremental sync: UID-based, header-first ──
        let (emails, new_max_uid, uid_validity) = crate::imap_client::fetch_new_emails_incremental(
            &account, stored_last_uid, stored_uid_validity
        ).await?;

        let new_count = emails.len();

        if new_count > 0 {
            let pool = state.db.lock().await;
            let receiver_account = account.display_name.as_deref().unwrap_or(&account.email);
            for email in &emails {
                let _ = db::import_fetched_email(
                    &pool, &email.from_address, &email.from_name, &email.subject,
                    &email.body_text, email.body_html.as_deref(), &email.date, &email.message_id,
                    email.is_read, receiver_account, email.attachments.clone(),
                ).await;
            }
            // Update sync state
            let _ = db::update_sync_state(&pool, &account_id, new_max_uid, uid_validity).await;
            drop(pool);
            Ok(format!("📬 새 메일 {}통을 가져왔습니다!", new_count))
        } else {
            // No new mail, just update uid_validity if needed
            if uid_validity != stored_uid_validity {
                let pool = state.db.lock().await;
                let _ = db::update_sync_state(&pool, &account_id, new_max_uid, uid_validity).await;
            }
            Ok("✓ 새 메일 없음".to_string())
        }
    } else {
        // ── First-time sync: full fetch ──
        let (emails, total_exists) = crate::imap_client::fetch_inbox_initial(&account, 50).await?;

        let pool = state.db.lock().await;
        let mut _imported_count = 0;

        for email in &emails {
            let receiver_account = account.display_name.as_deref().unwrap_or(&account.email);
            let result = db::import_fetched_email(
                &pool, &email.from_address, &email.from_name, &email.subject,
                &email.body_text, email.body_html.as_deref(), &email.date, &email.message_id,
                email.is_read, receiver_account, email.attachments.clone(),
            ).await;

            match result {
                Ok(true) => _imported_count += 1,
                Ok(false) => {}
                Err(e) => eprintln!("Import error: {}", e),
            }
        }

        // Get UID info for future incremental sync via a quick connection
        let (_, max_uid, uid_validity) = crate::imap_client::fetch_new_emails_incremental(
            &account, 0, 0
        ).await.unwrap_or((vec![], 0, 0));
        let _ = db::update_sync_state(&pool, &account_id, max_uid, uid_validity).await;

        drop(pool);

        // Spawn background task for the rest
        if total_exists > 50 {
            let db_clone = state.db.clone();
            let acc_clone = account.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::imap_client::sync_remaining_background(
                    app_handle,
                    acc_clone,
                    db_clone,
                    total_exists,
                    50
                ).await;
            });
            Ok(format!("✅ 연결 성공! INBOX에 {}통이 있습니다. (이전 메일 계속 가져오는 중...)", total_exists))
        } else {
            Ok(format!("✅ 연결 성공! 모든 메일({}통)을 동기화했습니다.", total_exists))
        }
    }
}

#[tauri::command]
pub async fn sync_all_accounts(app_handle: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let pool = state.db.lock().await;
    let accounts = db::get_email_accounts(&pool).await.map_err(|e| e.to_string())?;
    drop(pool);

    let mut total_imported = 0;
    let mut errors = Vec::new();

    for account in &accounts {
        if !account.enabled { continue; }
        match crate::imap_client::fetch_inbox_initial(account, 50).await {
            Ok((emails, total_exists)) => {
                let pool = state.db.lock().await;
                let receiver_account = account.display_name.as_deref().unwrap_or(&account.email);
                for email in &emails {
                    if let Ok(true) = db::import_fetched_email(
                        &pool, &email.from_address, &email.from_name, &email.subject,
                        &email.body_text, email.body_html.as_deref(), &email.date,
                        &email.message_id, email.is_read, receiver_account, email.attachments.clone(),
                    ).await {
                        total_imported += 1;
                    }
                }
                drop(pool);

                if total_exists > 50 {
                    let db_clone = state.db.clone();
                    let acc_clone = account.clone();
                    let app_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = crate::imap_client::sync_remaining_background(
                            app_clone, acc_clone, db_clone, total_exists, 50
                        ).await;
                    });
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", account.email, e));
            }
        }
    }

    if errors.is_empty() {
        Ok(format!("✅ 전체 초기 동기화 완료: {}통 새 메일 (나머지 백그라운드 진행 중)", total_imported))
    } else {
        Ok(format!("⚠️ {}통 가져옴, 오류: {}", total_imported, errors.join("; ")))
    }
}

// ╔═══════════════════════════════════════════════╗
// ║   CLOUD UPLOAD (Google Drive / OneDrive)      ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn cloud_get_auth_url(provider: String) -> Result<String, String> {
    match provider.as_str() {
        "gdrive" => Ok(crate::cloud_upload::gdrive_auth_url()),
        "onedrive" => Ok(crate::cloud_upload::onedrive_auth_url()),
        _ => Err("지원하지 않는 클라우드 저장소입니다".to_string()),
    }
}

#[tauri::command]
pub async fn cloud_start_auth(state: State<'_, AppState>, provider: String) -> Result<String, String> {
    // Open browser for OAuth
    let auth_url = match provider.as_str() {
        "gdrive" => crate::cloud_upload::gdrive_auth_url(),
        "onedrive" => crate::cloud_upload::onedrive_auth_url(),
        _ => return Err("지원하지 않는 클라우드입니다".to_string()),
    };

    // Open URL in default browser
    let _ = open::that(&auth_url);

    // Wait for OAuth callback
    let code = crate::cloud_upload::wait_for_oauth_callback().await?;

    // Exchange code for token
    let token = match provider.as_str() {
        "gdrive" => crate::cloud_upload::gdrive_exchange_code(&code).await?,
        "onedrive" => crate::cloud_upload::onedrive_exchange_code(&code).await?,
        _ => return Err("지원하지 않는 클라우드입니다".to_string()),
    };

    // Save token to DB
    let pool = state.db.lock().await;
    db::save_cloud_token(&pool, &provider, &token.access_token, token.refresh_token.as_deref())
        .await
        .map_err(|e| format!("토큰 저장 실패: {}", e))?;

    Ok(format!("✅ {} 연결 완료!", if provider == "gdrive" { "Google Drive" } else { "OneDrive" }))
}

#[tauri::command]
pub async fn cloud_upload_file(state: State<'_, AppState>, provider: String, file_path: String) -> Result<crate::cloud_upload::UploadResult, String> {
    let pool = state.db.lock().await;
    let token_data = db::get_cloud_token(&pool, &provider)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{} 계정이 연결되어 있지 않습니다. 먼저 설정에서 연결하세요.", 
            if provider == "gdrive" { "Google Drive" } else { "OneDrive" }))?;
    drop(pool);

    let token = &token_data.0;

    match provider.as_str() {
        "gdrive" => crate::cloud_upload::gdrive_upload(token, &file_path).await,
        "onedrive" => crate::cloud_upload::onedrive_upload(token, &file_path).await,
        _ => Err("지원하지 않는 클라우드입니다".to_string()),
    }
}

#[tauri::command]
pub async fn cloud_get_status(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let pool = state.db.lock().await;
    db::get_all_cloud_providers(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cloud_disconnect(state: State<'_, AppState>, provider: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_cloud_token(&pool, &provider).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   THREAD ACTIONS (Context Menu)               ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn toggle_thread_pin(state: State<'_, AppState>, thread_id: String) -> Result<bool, String> {
    let pool = state.db.lock().await;
    db::toggle_pin(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_thread_important(state: State<'_, AppState>, thread_id: String) -> Result<bool, String> {
    let pool = state.db.lock().await;
    db::toggle_important(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_thread_unread(state: State<'_, AppState>, thread_id: String) -> Result<bool, String> {
    let pool = state.db.lock().await;
    db::toggle_unread(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_thread_cmd(state: State<'_, AppState>, thread_id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_thread(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_thread_cmd(state: State<'_, AppState>, thread_id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::restore_thread(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn auto_empty_trash_cmd(state: State<'_, AppState>) -> Result<u64, String> {
    let pool = state.db.lock().await;
    db::auto_empty_trash(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn spam_thread_cmd(state: State<'_, AppState>, thread_id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::spam_thread(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn snooze_thread_cmd(state: State<'_, AppState>, thread_id: String, months: i64) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::snooze_thread(&pool, &thread_id, months).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   MAIL GROUPS                                 ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn create_mail_group(state: State<'_, AppState>, name: String, description: Option<String>, color: String) -> Result<crate::models::MailGroup, String> {
    let pool = state.db.lock().await;
    db::create_group(&pool, &name, description.as_deref(), &color).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mail_groups(state: State<'_, AppState>) -> Result<Vec<crate::models::MailGroup>, String> {
    let pool = state.db.lock().await;
    db::get_groups(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_mail_group(state: State<'_, AppState>, id: String, name: String, description: Option<String>, color: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::update_group(&pool, &id, &name, description.as_deref(), &color).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mail_group(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_group(&pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_group_member_cmd(state: State<'_, AppState>, group_id: String, email: String, display_name: Option<String>) -> Result<crate::models::GroupMember, String> {
    let pool = state.db.lock().await;
    db::add_group_member(&pool, &group_id, &email, display_name.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_group_member_cmd(state: State<'_, AppState>, member_id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::remove_group_member(&pool, &member_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_group_members_cmd(state: State<'_, AppState>, group_id: String) -> Result<Vec<crate::models::GroupMember>, String> {
    let pool = state.db.lock().await;
    db::get_group_members(&pool, &group_id).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   SCHEDULED EMAILS                            ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn create_scheduled_email_cmd(
    state: State<'_, AppState>, group_id: Option<String>, to_emails: String, subject: String, body: String,
    schedule_type: String, schedule_time: String, recurrence_rule: Option<String>
) -> Result<crate::models::ScheduledEmail, String> {
    let pool = state.db.lock().await;
    db::create_scheduled_email(&pool, group_id.as_deref(), &to_emails, &subject, &body, &schedule_type, &schedule_time, recurrence_rule.as_deref())
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_scheduled_emails_cmd(state: State<'_, AppState>) -> Result<Vec<crate::models::ScheduledEmail>, String> {
    let pool = state.db.lock().await;
    db::get_scheduled_emails(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_scheduled_email_cmd(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_scheduled_email(&pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_scheduled_email_cmd(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let pool = state.db.lock().await;
    db::toggle_scheduled_email(&pool, &id).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║   AUTO-TAGGING                                ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn retag_message_ai(state: State<'_, AppState>, msg_id: String, api_key: String) -> Result<Vec<String>, String> {
    let pool = state.db.lock().await;
    // Get message content
    let msg: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT body_summary, body_original FROM messages WHERE id = ?"
    ).bind(&msg_id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;

    let (summary, original) = msg.ok_or("Message not found")?;
    let body = original.unwrap_or_default();

    let tags = crate::auto_tagger::auto_tag(&summary, &body, Some(&api_key)).await;
    let tags_str = tags.join(",");

    sqlx::query("UPDATE messages SET hashtags = ? WHERE id = ?")
        .bind(&tags_str).bind(&msg_id).execute(&*pool).await.map_err(|e| e.to_string())?;

    Ok(tags)
}

#[tauri::command]
pub async fn retag_all_basic(state: State<'_, AppState>) -> Result<i64, String> {
    let pool = state.db.lock().await;
    let messages: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, body_summary, body_original FROM messages"
    ).fetch_all(&*pool).await.map_err(|e| e.to_string())?;

    let mut count = 0i64;
    for (id, summary, original) in &messages {
        let tags = crate::auto_tagger::auto_tag_basic(summary, &original.clone().unwrap_or_default());
        if !tags.is_empty() {
            let tags_str = tags.join(",");
            sqlx::query("UPDATE messages SET hashtags = ? WHERE id = ?")
                .bind(&tags_str).bind(id).execute(&*pool).await.map_err(|e| e.to_string())?;
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub async fn get_gemini_api_key(state: State<'_, AppState>) -> Result<String, String> {
    let pool = state.db.lock().await;
    // Store in a simple key-value settings table
    sqlx::query("CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    let val: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    Ok(val.map(|v| v.0).unwrap_or_default())
}

#[tauri::command]
pub async fn save_gemini_api_key(state: State<'_, AppState>, api_key: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    sqlx::query("CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT OR REPLACE INTO app_settings (key, value) VALUES ('gemini_api_key', ?)")
        .bind(&api_key).execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_app_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let pool = state.db.lock().await;
    sqlx::query("CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    let val: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
        .bind(&key)
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    Ok(val.map(|v| v.0))
}

#[tauri::command]
pub async fn save_app_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    sqlx::query("CREATE TABLE IF NOT EXISTS app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    sqlx::query("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?, ?)")
        .bind(&key).bind(&value)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn open_attachment_cmd(path: String) -> Result<(), String> {
    if std::path::Path::new(&path).exists() {
        open::that(&path).map_err(|e| format!("파일 열기 실패: {}", e))?;
        Ok(())
    } else {
        Err("파일을 찾을 수 없습니다.".to_string())
    }
}

#[tauri::command]
pub async fn save_attachment_cmd(source: String, destination: String) -> Result<(), String> {
    if std::path::Path::new(&source).exists() {
        std::fs::copy(&source, &destination).map_err(|e| format!("파일 복사 실패: {}", e))?;
        Ok(())
    } else {
        Err("원본 파일을 찾을 수 없습니다.".to_string())
    }
}

#[tauri::command]
pub async fn drag_attachment_cmd(path: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&path);
    if !src.exists() {
        return Err("파일을 찾을 수 없습니다.".to_string());
    }
    // Copy file to Desktop as a drag-out fallback
    let desktop = dirs::desktop_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("Desktop"));
    let filename = src.file_name().unwrap_or_default();
    let dest = desktop.join(filename);
    std::fs::copy(&src, &dest).map_err(|e| format!("파일 복사 실패: {}", e))?;
    Ok(dest.to_string_lossy().to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║      CONTACTS & ADDRESS BOOK                  ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn toggle_briefing(state: State<'_, AppState>, thread_id: String, is_briefing: bool) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::toggle_briefing(&pool, &thread_id, is_briefing).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_contact_groups(state: State<'_, AppState>) -> Result<Vec<ContactGroup>, String> {
    let pool = state.db.lock().await;
    db::get_contact_groups(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_contacts(state: State<'_, AppState>) -> Result<Vec<Contact>, String> {
    let pool = state.db.lock().await;
    db::get_contacts(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_mock_contacts(state: State<'_, AppState>, provider: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::sync_mock_contacts(&pool, &provider).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║      SEND EMAIL (SMTP)                        ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn send_email_cmd(
    state: State<'_, AppState>,
    to: String, cc: String, bcc: String, subject: String, body_html: String, attachments: Vec<String>, account_id: Option<String>,
) -> Result<(), String> {
    let pool = state.db.lock().await;
    
    let account = if let Some(aid) = &account_id {
        db::get_email_account_by_id(&pool, aid).await.map_err(|e| e.to_string())?
    } else {
        db::get_default_email_account(&pool).await.map_err(|e| e.to_string())?
    };
    
    let acct = account.ok_or("이메일 계정이 설정되지 않았습니다. 설정에서 계정을 추가해주세요.".to_string())?;
    
    let smtp_host = acct.smtp_host.as_deref().unwrap_or("smtp.gmail.com");
    let smtp_port = acct.smtp_port.unwrap_or(587) as u16;
    let username = acct.username.as_deref().unwrap_or(&acct.email);
    let password = acct.password_encrypted.as_deref().unwrap_or("");
    let display_name = acct.display_name.as_deref().unwrap_or(&acct.email);
    
    // Get default signature
    let sigs = db::get_signatures(&pool).await.map_err(|e| e.to_string())?;
    let default_sig = sigs.iter().find(|s| s.is_default == 1).map(|s| s.body_html.as_str());
    
    let cc_opt = if cc.trim().is_empty() { None } else { Some(cc.as_str()) };
    let bcc_opt = if bcc.trim().is_empty() { None } else { Some(bcc.as_str()) };

    crate::smtp_client::send_email(
        smtp_host, smtp_port, username, password,
        display_name, &acct.email, &to, cc_opt, bcc_opt, &subject, &body_html, default_sig, attachments,
    )?;

    // Store sent message in DB
    let thread_id = format!("sent-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();
    let preview = crate::imap_client::strip_html_tags(&body_html);

    let _ = sqlx::query(
        "INSERT INTO chat_rooms (thread_id, subject, status, last_received_at, sender_name, pinned, important, unread_count, last_message_preview)
         VALUES (?, ?, 'open', ?, ?, 0, 0, 0, ?)"
    ).bind(&thread_id).bind(&subject).bind(&now).bind(&to).bind(&preview.chars().take(100).collect::<String>())
    .execute(&*pool).await;

    let msg_id = uuid::Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO messages (id, thread_id, message_type, sender_identity, sender_name, body_summary, body_original, created_at, is_outgoing)
         VALUES (?, ?, 'email', ?, ?, ?, ?, ?, 1)"
    ).bind(&msg_id).bind(&thread_id).bind(&acct.email).bind(display_name).bind(&preview.chars().take(300).collect::<String>()).bind(&body_html).bind(&now)
    .execute(&*pool).await;

    Ok(())
}

// ╔═══════════════════════════════════════════════╗
// ║      MEMOS (Private Notes)                    ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn get_memo_cmd(state: State<'_, AppState>, thread_id: String) -> Result<Option<String>, String> {
    let pool = state.db.lock().await;
    db::get_memo(&pool, &thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_memo_cmd(state: State<'_, AppState>, thread_id: String, content: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::save_memo(&pool, &thread_id, &content).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║      SIGNATURES                               ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn get_signatures_cmd(state: State<'_, AppState>) -> Result<Vec<db::EmailSignature>, String> {
    let pool = state.db.lock().await;
    db::get_signatures(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_signature_cmd(state: State<'_, AppState>, id: Option<String>, name: String, body_html: String, is_default: bool) -> Result<String, String> {
    let pool = state.db.lock().await;
    db::save_signature(&pool, id.as_deref(), &name, &body_html, is_default).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_signature_cmd(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.db.lock().await;
    db::delete_signature(&pool, &id).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║      CONTACT SYNC                             ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn sync_contacts_from_mail_cmd(state: State<'_, AppState>) -> Result<i64, String> {
    let pool = state.db.lock().await;
    db::sync_contacts_from_messages(&pool).await.map_err(|e| e.to_string())
}

// ╔═══════════════════════════════════════════════╗
// ║      AI INTELLIGENCE                          ║
// ╚═══════════════════════════════════════════════╝

#[tauri::command]
pub async fn process_email_ai_cmd(state: State<'_, AppState>, thread_id: String) -> Result<serde_json::Value, String> {
    let pool = state.db.lock().await;

    // Get API key
    let api_key: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let key = api_key.map(|(v,)| v).unwrap_or_default();
    if key.is_empty() {
        return Err("Gemini API 키가 설정되지 않았습니다. 설정에서 API 키를 입력해주세요.".into());
    }

    // Get unprocessed messages for this thread
    let messages: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, thread_id, body_summary, body_original FROM messages 
         WHERE thread_id = ? AND (ai_processed IS NULL OR ai_processed = 0) 
         ORDER BY created_at DESC LIMIT 3"
    ).bind(&thread_id).fetch_all(&*pool).await.map_err(|e| e.to_string())?;

    if messages.is_empty() {
        // Already processed, return existing data
        let existing: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT m.ai_summary, m.ai_translation, c.ai_tags FROM messages m 
             JOIN chat_rooms c ON m.thread_id = c.thread_id 
             WHERE m.thread_id = ? AND m.ai_processed = 1 ORDER BY m.created_at DESC LIMIT 1"
        ).bind(&thread_id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;

        if let Some((summary, translation, tags)) = existing {
            return Ok(serde_json::json!({
                "status": "already_processed",
                "summary": summary,
                "translation": translation,
                "tags": tags.and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok()).unwrap_or_default()
            }));
        }
        return Ok(serde_json::json!({"status": "no_messages"}));
    }

    // Get thread subject
    let subject: (String,) = sqlx::query_as("SELECT subject FROM chat_rooms WHERE thread_id = ?")
        .bind(&thread_id).fetch_one(&*pool).await.map_err(|e| e.to_string())?;

    // Combine bodies for AI
    let combined_body: String = messages.iter()
        .map(|(_, _, summary, original)| {
            original.as_deref()
                .map(|o| crate::imap_client::strip_html_tags(o))
                .unwrap_or_else(|| summary.as_deref().unwrap_or("").to_string())
        })
        .collect::<Vec<_>>()
        .join("\n---\n");

    // Get model setting
    let model_opt: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_model'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let model = model_opt.map(|(v,)| v).unwrap_or_else(|| "gemini-2.0-flash".to_string());

    // Process with AI engine
    let result = crate::ai_engine::process_email(
        &subject.0, &combined_body, &key, &model, true,
    ).await?;

    // Save AI results to DB
    let tags_json = serde_json::to_string(&result.tags).unwrap_or_default();

    for (msg_id, _, _, _) in &messages {
        sqlx::query(
            "UPDATE messages SET ai_summary = ?, ai_translation = ?, ai_processed = 1 WHERE id = ?"
        ).bind(&result.summary).bind(&result.translation).bind(msg_id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;
    }

    // Update chat_room with tags and needs_action
    sqlx::query(
        "UPDATE chat_rooms SET ai_tags = ?, needs_action = ? WHERE thread_id = ?"
    ).bind(&tags_json).bind(result.needs_action as i32).bind(&thread_id)
    .execute(&*pool).await.map_err(|e| e.to_string())?;

    // Also mark as important if AI says so
    if result.important {
        sqlx::query("UPDATE chat_rooms SET important = 1 WHERE thread_id = ?")
            .bind(&thread_id).execute(&*pool).await.map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({
        "status": "processed",
        "summary": result.summary,
        "translation": result.translation,
        "tags": result.tags,
        "important": result.important,
        "needs_action": result.needs_action
    }))
}

#[tauri::command]
pub async fn translate_email_cmd(state: State<'_, AppState>, message_id: String) -> Result<String, String> {
    let pool = state.db.lock().await;

    // Get API key
    let api_key: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let key = api_key.map(|(v,)| v).unwrap_or_default();
    if key.is_empty() {
        return Err("Gemini API 키가 설정되지 않았습니다.".into());
    }

    // Check if already translated
    let existing: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT ai_translation FROM messages WHERE id = ?"
    ).bind(&message_id).fetch_optional(&*pool).await.map_err(|e| e.to_string())?;

    if let Some((Some(translation),)) = &existing {
        if !translation.is_empty() {
            return Ok(translation.clone());
        }
    }

    // Get message body
    let body: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT body_original, body_summary FROM messages WHERE id = ?"
    ).bind(&message_id).fetch_one(&*pool).await.map_err(|e| e.to_string())?;

    let text = body.0.as_deref()
        .map(|o| crate::imap_client::strip_html_tags(o))
        .unwrap_or_else(|| body.1.unwrap_or_default());

    // Get model setting
    let model_opt: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_model'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let model = model_opt.map(|(v,)| v).unwrap_or_else(|| "gemini-2.0-flash".to_string());

    let translation = crate::ai_engine::translate_email(&text, &key, &model).await?;

    // Save translation
    sqlx::query("UPDATE messages SET ai_translation = ? WHERE id = ?")
        .bind(&translation).bind(&message_id)
        .execute(&*pool).await.map_err(|e| e.to_string())?;

    Ok(translation)
}

#[tauri::command]
pub async fn test_ai_connection_cmd(state: State<'_, AppState>) -> Result<String, String> {
    let pool = state.db.lock().await;
    let api_key: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let key = api_key.map(|(v,)| v).unwrap_or_default();
    if key.is_empty() {
        return Err("API 키가 비어있습니다.".into());
    }

    let model_opt: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_model'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let model = model_opt.map(|(v,)| v).unwrap_or_else(|| "gemini-2.0-flash".to_string());

    // Simple test prompt
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, key
    );
    let response = client.post(&url)
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": "Hello, respond with just 'OK'"}]}],
            "generationConfig": {"maxOutputTokens": 5}
        }))
        .send().await.map_err(|e| format!("연결 실패: {}", e))?;

    if response.status().is_success() {
        Ok("✅ AI 연결 성공! Gemini API가 정상 작동합니다.".into())
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(format!("❌ API 오류: {}", body))
    }
}

#[tauri::command]
pub async fn get_available_models_cmd(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    let pool = state.db.lock().await;
    let api_key: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = 'gemini_api_key'")
        .fetch_optional(&*pool).await.map_err(|e| e.to_string())?;
    let key = api_key.map(|(v,)| v).unwrap_or_default();
    if key.is_empty() {
        return Err("API 키가 설정되지 않았습니다.".into());
    }

    let client = reqwest::Client::new();
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", key);
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let models = json["models"].as_array().cloned().unwrap_or_default();
        
        // Filter out only supported generateContent models
        let mut supported = Vec::new();
        for model in models {
            if let Some(methods) = model["supportedGenerationMethods"].as_array() {
                if methods.iter().any(|m| m.as_str() == Some("generateContent")) {
                    supported.push(model);
                }
            }
        }
        Ok(supported)
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(format!("모델 목록 가져오기 실패: {}", body))
    }
}

