mod db;
mod models;
mod commands;
mod seed;
mod telegram;
mod imap_client;
mod cloud_upload;
mod auto_tagger;
mod smtp_client;
mod ai_engine;

use commands::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                let app_data_dir = dirs::data_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join("com.bongpark.lumina-mail");

                let pool = db::init_db(app_data_dir)
                    .await
                    .expect("Failed to initialize database");

                seed::seed_mock_data(&pool)
                    .await
                    .expect("Failed to seed mock data");

                let pool_clone = pool.clone();
                app_handle.manage(AppState {
                    db: Arc::new(Mutex::new(pool)),
                });

                // Run heavy migration in background (non-blocking)
                tauri::async_runtime::spawn(async move {
                    // Check if migration already ran via a simple flag
                    let already_ran: Result<(i64,), _> = sqlx::query_as(
                        "SELECT COUNT(*) FROM app_settings WHERE key = 'css_cleanup_v2_done'"
                    ).fetch_one(&pool_clone).await;
                    
                    if let Ok((count,)) = already_ran {
                        if count == 0 {
                            eprintln!("[Startup] Running one-time body_summary cleanup...");
                            let _ = db::cleanup_css_previews(&pool_clone).await;
                            let _ = sqlx::query(
                                "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('css_cleanup_v2_done', '1')"
                            ).execute(&pool_clone).await;
                            eprintln!("[Startup] Cleanup complete.");
                        }
                    }
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core
            commands::get_threads,
            commands::get_messages,
            commands::get_thread_detail,
            commands::mark_thread_read,
            commands::send_reply,
            commands::get_messages_by_date,
            // Telegram
            commands::telegram_start_linking,
            commands::telegram_poll_link,
            commands::telegram_get_status,
            commands::telegram_disconnect,
            commands::telegram_send_test,
            // Email accounts
            commands::get_email_accounts,
            commands::add_email_account,
            commands::update_email_account_details,
            commands::update_email_sync_mode,
            commands::delete_email_account,
            // Todos
            commands::get_todos,
            commands::add_todo,
            commands::toggle_todo,
            commands::delete_todo,
            // Calendar
            commands::get_calendar_events,
            commands::add_calendar_event,
            commands::delete_calendar_event,
            // Email sync (IMAP)
            commands::test_email_connection,
            commands::sync_email_account,
            commands::sync_all_accounts,
            // Cloud upload
            commands::cloud_get_auth_url,
            commands::cloud_start_auth,
            commands::cloud_upload_file,
            commands::cloud_get_status,
            commands::cloud_disconnect,
            // Thread actions (context menu)
            commands::toggle_thread_pin,
            commands::toggle_thread_important,
            commands::toggle_thread_unread,
            commands::delete_thread_cmd,
            commands::restore_thread_cmd,
            commands::auto_empty_trash_cmd,
            commands::spam_thread_cmd,
            commands::snooze_thread_cmd,
            // Mail groups
            commands::create_mail_group,
            commands::get_mail_groups,
            commands::update_mail_group,
            commands::delete_mail_group,
            commands::add_group_member_cmd,
            commands::remove_group_member_cmd,
            commands::get_group_members_cmd,
            // Scheduled emails
            commands::create_scheduled_email_cmd,
            commands::get_scheduled_emails_cmd,
            commands::delete_scheduled_email_cmd,
            commands::toggle_scheduled_email_cmd,
            // Auto-tagging
            commands::retag_message_ai,
            commands::retag_all_basic,
            commands::get_gemini_api_key,
            commands::save_gemini_api_key,
            commands::get_app_setting,
            commands::save_app_setting,
            // Attachments
            commands::open_attachment_cmd,
            commands::save_attachment_cmd,
            commands::drag_attachment_cmd,
            // Contacts & Briefing
            commands::toggle_briefing,
            commands::get_contact_groups,
            commands::get_contacts,
            commands::sync_mock_contacts,
            // Phase 13: New features
            commands::send_email_cmd,
            commands::get_memo_cmd,
            commands::save_memo_cmd,
            commands::get_signatures_cmd,
            commands::save_signature_cmd,
            commands::delete_signature_cmd,
            commands::sync_contacts_from_mail_cmd,
            // Phase 14: AI Intelligence
            commands::process_email_ai_cmd,
            commands::translate_email_cmd,
            commands::add_thread_tag_cmd,
            commands::test_ai_connection_cmd,
            commands::get_available_models_cmd,
        ])
        .menu(|app_handle| {
            use tauri::menu::*;
            let menu = MenuBuilder::new(app_handle);
            
            // App submenu
            let app_menu = SubmenuBuilder::new(app_handle, "Lumina Mail")
                .item(&MenuItem::with_id(app_handle, "settings", "설정...", true, Some("CmdOrCtrl+,"))?)
                .separator()
                .item(&MenuItem::with_id(app_handle, "quit", "종료", true, Some("CmdOrCtrl+Q"))?)
                .build()?;
            
            // Mail submenu
            let mail_menu = SubmenuBuilder::new(app_handle, "메일")
                .item(&MenuItem::with_id(app_handle, "compose", "새 메일 작성", true, Some("CmdOrCtrl+N"))?)
                .item(&MenuItem::with_id(app_handle, "reply", "답장", true, Some("CmdOrCtrl+R"))?)
                .item(&MenuItem::with_id(app_handle, "forward", "전달", true, Some("CmdOrCtrl+Shift+F"))?)
                .separator()
                .item(&MenuItem::with_id(app_handle, "sync", "모든 계정 동기화", true, Some("CmdOrCtrl+Shift+S"))?)
                .separator()
                .item(&MenuItem::with_id(app_handle, "toggle_unread", "읽음/안읽음 표시", true, Some("CmdOrCtrl+Shift+U"))?)
                .item(&MenuItem::with_id(app_handle, "toggle_important", "중요 표시", true, Some("CmdOrCtrl+Shift+I"))?)
                .item(&MenuItem::with_id(app_handle, "toggle_pin", "핀 고정/해제", true, Some("CmdOrCtrl+Shift+P"))?)
                .separator()
                .item(&MenuItem::with_id(app_handle, "archive", "보관", true, Some("CmdOrCtrl+E"))?)
                .build()?;
            
            // Edit submenu
            let edit_menu = SubmenuBuilder::new(app_handle, "편집")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;
            
            // View submenu
            let view_menu = SubmenuBuilder::new(app_handle, "보기")
                .item(&MenuItem::with_id(app_handle, "filter_all", "전체 메일", true, Some("CmdOrCtrl+1"))?)
                .item(&MenuItem::with_id(app_handle, "filter_unread", "안 읽음", true, Some("CmdOrCtrl+2"))?)
                .item(&MenuItem::with_id(app_handle, "filter_pinned", "고정됨", true, Some("CmdOrCtrl+3"))?)
                .item(&MenuItem::with_id(app_handle, "filter_trash", "휴지통", true, Some("CmdOrCtrl+4"))?)
                .item(&MenuItem::with_id(app_handle, "filter_spam", "스팸", true, Some("CmdOrCtrl+5"))?)
                .separator()
                .item(&MenuItem::with_id(app_handle, "contacts", "주소록", true, Some("CmdOrCtrl+6"))?)
                .build()?;
            
            // Help submenu
            let help_menu = SubmenuBuilder::new(app_handle, "도움말")
                .item(&MenuItem::with_id(app_handle, "help_guide", "📖 사용 설명서", true, Some("CmdOrCtrl+Shift+/"))?)
                .separator()
                .item(&MenuItem::with_id(app_handle, "shortcut_guide", "단축키 안내", true, Some("CmdOrCtrl+/"))?)
                .build()?;
            
            menu.item(&app_menu)
                .item(&mail_menu)
                .item(&edit_menu)
                .item(&view_menu)
                .item(&help_menu)
                .build()
        })
        .on_menu_event(|app_handle, event| {
            use tauri::Emitter;
            let id = event.id().as_ref();
            match id {
                "quit" => std::process::exit(0),
                _ => {
                    let _ = app_handle.emit("menu-action", id);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
