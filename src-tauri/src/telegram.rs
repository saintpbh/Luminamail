// ═══════════════════════════════════════════════════════════
// Lumina Mail - Telegram Bot Integration
// Real Bot API with personal linking via unique code
// ═══════════════════════════════════════════════════════════

use reqwest::Client;
use serde::{Deserialize, Serialize};

const BOT_TOKEN: &str = "7895607405:AAFx5UEB5jPs57F_eJo_uvCnyaWsdbs6Rfg";

fn api_url(method: &str) -> String {
    format!("https://api.telegram.org/bot{}/{}", BOT_TOKEN, method)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub chat: TelegramChat,
    pub text: Option<String>,
    pub from: Option<TelegramUser>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub data: Option<String>,
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetUpdatesResponse {
    ok: bool,
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendMessageResponse {
    ok: bool,
}

/// Generate a 6-digit linking code
pub fn generate_link_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(100000..999999))
}

/// Poll Telegram for new messages and check for linking code
pub async fn check_for_link_code(
    code: &str,
    last_update_id: i64,
) -> Result<Option<(i64, String, i64)>, String> {
    let client = Client::new();
    let url = api_url("getUpdates");

    let resp = client
        .get(&url)
        .query(&[
            ("offset", (last_update_id + 1).to_string()),
            ("timeout", "5".to_string()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let data: GetUpdatesResponse = resp.json().await.map_err(|e| e.to_string())?;

    for update in &data.result {
        if let Some(msg) = &update.message {
            if let Some(text) = &msg.text {
                let trimmed = text.trim();
                // Check if this is the linking code
                if trimmed == code {
                    let chat_id = msg.chat.id;
                    let username = msg.from.as_ref()
                        .map(|u| u.username.clone().unwrap_or_else(|| u.first_name.clone()))
                        .unwrap_or_else(|| "Unknown".to_string());

                    // Send confirmation
                    let _ = send_message(
                        chat_id,
                        "✅ Lumina Mail 연결 완료!\n\n이제부터 중요한 이메일 알림을 받으실 수 있습니다.",
                        None,
                    ).await;

                    return Ok(Some((chat_id, username, update.update_id)));
                }
                // Handle /start command
                else if trimmed == "/start" {
                    let _ = send_message(
                        msg.chat.id,
                        "👋 Lumina Mail Bot에 오신 것을 환영합니다!\n\n📱 Lumina Mail 앱에서 표시된 6자리 연결 코드를 입력해 주세요.",
                        None,
                    ).await;
                }
            }
        }
    }

    // Return the last update_id for offset tracking
    let new_offset = data.result.last().map(|u| u.update_id).unwrap_or(last_update_id);
    if new_offset > last_update_id {
        // We processed updates but didn't find the code
        Ok(None)
    } else {
        Ok(None)
    }
}

/// Send a text message to a Telegram chat
pub async fn send_message(
    chat_id: i64,
    text: &str,
    reply_markup: Option<serde_json::Value>,
) -> Result<(), String> {
    let client = Client::new();
    let url = api_url("sendMessage");

    let mut body = serde_json::json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML",
    });

    if let Some(markup) = reply_markup {
        body["reply_markup"] = markup;
    }

    client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Send mail notification to Telegram with action buttons
pub async fn send_mail_notification(
    chat_id: i64,
    subject: &str,
    sender: &str,
    preview: &str,
    thread_id: &str,
) -> Result<(), String> {
    let text = format!(
        "📧 <b>새 메일 도착</b>\n\n<b>보낸 사람:</b> {}\n<b>제목:</b> {}\n\n{}",
        sender, subject, preview
    );

    let markup = serde_json::json!({
        "inline_keyboard": [[
            {"text": "✅ 승인", "callback_data": format!("approve:{}", thread_id)},
            {"text": "💬 답장", "callback_data": format!("reply:{}", thread_id)},
            {"text": "📌 고정", "callback_data": format!("pin:{}", thread_id)},
        ]]
    });

    send_message(chat_id, &text, Some(markup)).await
}

/// Get latest update_id for offset tracking
pub async fn get_latest_update_id() -> Result<i64, String> {
    let client = Client::new();
    let url = api_url("getUpdates");

    let resp = client
        .get(&url)
        .query(&[("offset", "-1"), ("limit", "1")])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let data: GetUpdatesResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.result.last().map(|u| u.update_id).unwrap_or(0))
}
