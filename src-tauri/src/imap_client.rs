// ═══════════════════════════════════════════════════════════
// Lumina Mail - Real IMAP Email Client
// Connects to IMAP servers and fetches actual emails
// ═══════════════════════════════════════════════════════════

use mailparse::*;
use futures::TryStreamExt;
use encoding_rs::EUC_KR;

use crate::models::EmailAccount;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FetchedAttachment {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FetchedEmail {
    pub message_id: String,
    pub from_address: String,
    pub from_name: String,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<FetchedAttachment>,
    pub date: String,
    pub is_read: bool,
}

async fn connect_imap(account: &EmailAccount) -> Result<async_imap::Session<async_native_tls::TlsStream<impl futures::io::AsyncRead + futures::io::AsyncWrite + Unpin + std::fmt::Debug + Send>>, String> {
    let host = account.imap_host.as_deref().unwrap_or("imap.gmail.com");
    let port = account.imap_port.unwrap_or(993) as u16;
    let username = account.username.as_deref().unwrap_or(&account.email);
    let password = account.password_encrypted.as_deref().unwrap_or("");

    let tls = async_native_tls::TlsConnector::new();
    let addr = format!("{}:{}", host, port);

    let tcp = async_std_tcp_connect(&addr).await?;
    let tls_stream = tls.connect(host, tcp)
        .await
        .map_err(|e| format!("TLS 연결 실패: {}", e))?;

    let client = async_imap::Client::new(tls_stream);

    let session = client
        .login(username, password)
        .await
        .map_err(|e| format!("로그인 실패: {} (비밀번호 또는 앱 비밀번호를 확인하세요)", e.0))?;

    Ok(session)
}

pub async fn fetch_inbox_initial(account: &EmailAccount, max_count: u32) -> Result<(Vec<FetchedEmail>, u32), String> {
    let mut session = connect_imap(account).await?;
    let mailbox = session.select("INBOX").await.map_err(|e| e.to_string())?;
    
    let total = mailbox.exists;
    if total == 0 {
        let _ = session.logout().await;
        return Ok((vec![], 0));
    }

    let start = if total > max_count { total - max_count + 1 } else { 1 };
    let range = format!("{}:{}", start, total);

    let emails = fetch_and_parse_range(&mut session, &range).await?;
    let _ = session.logout().await;
    
    Ok((emails, total))
}

/// Incremental UID-based sync: only fetch emails newer than last_uid.
/// Returns (new_emails, new_max_uid, uid_validity, is_full_reset).
pub async fn fetch_new_emails_incremental(
    account: &EmailAccount,
    stored_last_uid: u32,
    stored_uid_validity: u32,
) -> Result<(Vec<FetchedEmail>, u32, u32), String> {
    let mut session = connect_imap(account).await?;
    let mailbox = session.select("INBOX").await.map_err(|e| e.to_string())?;
    
    let total = mailbox.exists;
    let uid_validity = mailbox.uid_validity.unwrap_or(0);

    if total == 0 {
        let _ = session.logout().await;
        return Ok((vec![], 0, uid_validity));
    }

    // If uid_validity changed, the mailbox was reset → full re-sync needed
    if stored_uid_validity > 0 && uid_validity != stored_uid_validity {
        eprintln!("UID validity changed ({} → {}), doing full sync", stored_uid_validity, uid_validity);
        let start = if total > 50 { total - 50 + 1 } else { 1 };
        let range = format!("{}:{}", start, total);
        let emails = fetch_and_parse_range(&mut session, &range).await?;
        // Find max UID from fetched emails
        let max_uid = get_max_uid_from_session(&mut session, &format!("{}:{}", start, total)).await.unwrap_or(total);
        let _ = session.logout().await;
        return Ok((emails, max_uid, uid_validity));
    }

    // If we have a stored UID, only fetch newer messages
    if stored_last_uid > 0 {
        let uid_range = format!("{}:*", stored_last_uid + 1);
        
        // First: fetch only headers (ENVELOPE + FLAGS) to detect what's new
        let header_messages: Vec<async_imap::types::Fetch> = session
            .uid_fetch(&uid_range, "(UID FLAGS ENVELOPE)")
            .await
            .map_err(|e| format!("UID header fetch failed: {}", e))?
            .try_collect()
            .await
            .map_err(|e| format!("UID header stream failed: {}", e))?;

        // Filter: only UIDs strictly greater than stored_last_uid
        let new_uids: Vec<u32> = header_messages.iter()
            .filter_map(|m| m.uid)
            .filter(|&uid| uid > stored_last_uid)
            .collect();

        if new_uids.is_empty() {
            let _ = session.logout().await;
            return Ok((vec![], stored_last_uid, uid_validity));
        }

        // Second: fetch full body ONLY for genuinely new UIDs
        let uid_set: String = new_uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        let emails = uid_fetch_and_parse(&mut session, &uid_set).await?;
        let max_uid = new_uids.iter().copied().max().unwrap_or(stored_last_uid);

        let _ = session.logout().await;
        return Ok((emails, max_uid, uid_validity));
    }

    // First-time sync: fetch latest 50 with UIDs
    let start = if total > 50 { total - 50 + 1 } else { 1 };
    let range = format!("{}:{}", start, total);
    let emails = fetch_and_parse_range(&mut session, &range).await?;
    let max_uid = get_max_uid_from_session(&mut session, &format!("{}:{}", start, total)).await.unwrap_or(total);
    let _ = session.logout().await;
    
    Ok((emails, max_uid, uid_validity))
}

/// Fetch and parse emails by UID set (comma-separated UIDs)
async fn uid_fetch_and_parse<T: futures::io::AsyncRead + futures::io::AsyncWrite + Unpin + Send + std::fmt::Debug>(
    session: &mut async_imap::Session<async_native_tls::TlsStream<T>>,
    uid_set: &str,
) -> Result<Vec<FetchedEmail>, String> {
    let messages: Vec<async_imap::types::Fetch> = session
        .uid_fetch(uid_set, "(FLAGS ENVELOPE BODY[] INTERNALDATE)")
        .await
        .map_err(|e| format!("UID fetch failed: {}", e))?
        .try_collect()
        .await
        .map_err(|e| format!("UID stream collect failed: {}", e))?;

    parse_fetched_messages(messages)
}

/// Get max UID from a sequence range
async fn get_max_uid_from_session<T: futures::io::AsyncRead + futures::io::AsyncWrite + Unpin + Send + std::fmt::Debug>(
    session: &mut async_imap::Session<async_native_tls::TlsStream<T>>,
    range: &str,
) -> Result<u32, String> {
    let uid_msgs: Vec<async_imap::types::Fetch> = session
        .fetch(range, "(UID)")
        .await
        .map_err(|e| format!("UID query failed: {}", e))?
        .try_collect()
        .await
        .map_err(|e| format!("UID stream failed: {}", e))?;
    
    Ok(uid_msgs.iter().filter_map(|m| m.uid).max().unwrap_or(0))
}

async fn fetch_and_parse_range<T: futures::io::AsyncRead + futures::io::AsyncWrite + Unpin + Send + std::fmt::Debug>(
    session: &mut async_imap::Session<async_native_tls::TlsStream<T>>,
    range: &str
) -> Result<Vec<FetchedEmail>, String> {
    let messages: Vec<async_imap::types::Fetch> = session
        .fetch(range, "(FLAGS ENVELOPE BODY[] INTERNALDATE)")
        .await
        .map_err(|e| format!("메일 가져오기 실패: {}", e))?
        .try_collect()
        .await
        .map_err(|e| format!("메일 스트림 수집 실패: {}", e))?;

    parse_fetched_messages(messages)
}

/// Shared parsing logic for IMAP Fetch results
fn parse_fetched_messages(messages: Vec<async_imap::types::Fetch>) -> Result<Vec<FetchedEmail>, String> {
    // Helper for fixing EUC-KR Mojibake
    fn decode_legacy_korean(input: &str) -> String {
        if input.is_ascii() { return input.to_string(); }
        let is_latin1 = input.chars().all(|c| c as u32 <= 255);
        if is_latin1 {
            let bytes: Vec<u8> = input.chars().map(|c| c as u8).collect();
            let (decoded, _, had_errors) = EUC_KR.decode(&bytes);
            if !had_errors && !decoded.as_ref().contains('\u{FFFD}') {
                return decoded.into_owned();
            }
        }
        input.to_string()
    }

    let mut emails = Vec::new();
    for msg in &messages {
        let body_raw = msg.body().unwrap_or_default();
        let mut flags = msg.flags();
        let is_read = flags.any(|f| matches!(f, async_imap::types::Flag::Seen));

        if let Ok(parsed) = parse_mail(body_raw) {
            let mut subject = parsed.headers.iter().find(|h| h.get_key_ref() == "Subject").map(|h| h.get_value()).unwrap_or_else(|| "(제목 없음)".to_string());
            subject = decode_legacy_korean(&subject);
            
            let mut from_raw = parsed.headers.iter().find(|h| h.get_key_ref() == "From").map(|h| h.get_value()).unwrap_or_default();
            from_raw = decode_legacy_korean(&from_raw);
            
            let (from_name, from_address) = parse_from_header(&from_raw);
            let date = parsed.headers.iter().find(|h| h.get_key_ref() == "Date").map(|h| h.get_value()).unwrap_or_default();
            let message_id = parsed.headers.iter().find(|h| h.get_key_ref() == "Message-ID" || h.get_key_ref() == "Message-Id").map(|h| h.get_value()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let (body_text, body_html) = extract_body(&parsed);
            let mut attachments = Vec::new();
            extract_attachments(&parsed, &mut attachments);

            emails.push(FetchedEmail {
                message_id: message_id.trim_matches(|c| c == '<' || c == '>').to_string(),
                from_address, from_name, subject, body_text, body_html, attachments,
                date: parse_email_date(&date), is_read,
            });
        }
    }
    emails.reverse();
    Ok(emails)
}

#[derive(Clone, serde::Serialize)]
pub struct SyncProgressPayload {
    pub account_id: String,
    pub current: u32,
    pub total: u32,
}

pub async fn sync_remaining_background(
    app: tauri::AppHandle,
    account: EmailAccount,
    db: std::sync::Arc<tokio::sync::Mutex<sqlx::SqlitePool>>,
    total_exists: u32,
    already_fetched: u32,
) -> Result<(), String> {
    use tauri::Emitter;
    
    if total_exists <= already_fetched { return Ok(()); }
    
    let mut current_end = total_exists - already_fetched;
    let batch_size = 50;
    
    let mut session = connect_imap(&account).await?;
    let _ = session.select("INBOX").await;
    
    let _total_to_fetch = current_end;
    let mut fetched_so_far = 0;

    while current_end > 0 {
        let current_start = if current_end > batch_size { current_end - batch_size + 1 } else { 1 };
        let range = format!("{}:{}", current_start, current_end);
        
        if let Ok(emails) = fetch_and_parse_range(&mut session, &range).await {
            let pool = db.lock().await;
            let receiver_account = account.display_name.as_deref().unwrap_or(&account.email);
            for email in &emails {
                let _ = crate::db::import_fetched_email(
                    &pool, &email.from_address, &email.from_name, &email.subject,
                    &email.body_text, email.body_html.as_deref(), &email.date, &email.message_id,
                    email.is_read, receiver_account, email.attachments.clone(),
                ).await;
            }
            drop(pool);
            
            fetched_so_far += emails.len() as u32;
            let _ = app.emit("sync-progress", SyncProgressPayload {
                account_id: account.id.clone(),
                current: fetched_so_far + already_fetched,
                total: total_exists,
            });
        } else {
            // Reconnect logic on failure
            let _ = session.logout().await;
            if let Ok(mut new_sess) = connect_imap(&account).await {
                if new_sess.select("INBOX").await.is_ok() {
                    session = new_sess;
                }
            }
        }
        
        if current_start == 1 { break; }
        current_end = current_start - 1;
    }
    
    let _ = session.logout().await;
    Ok(())
}

/// Test IMAP connection without fetching mail
pub async fn test_connection(account: &EmailAccount) -> Result<String, String> {
    let host = account.imap_host.as_deref().unwrap_or("imap.gmail.com");
    let port = account.imap_port.unwrap_or(993) as u16;
    let username = account.username.as_deref().unwrap_or(&account.email);
    let password = account.password_encrypted.as_deref().unwrap_or("");

    let tls = async_native_tls::TlsConnector::new();
    let addr = format!("{}:{}", host, port);

    let tcp = async_std_tcp_connect(&addr).await?;
    let tls_stream = tls.connect(host, tcp)
        .await
        .map_err(|e| format!("TLS 연결 실패: {}", e))?;

    let client = async_imap::Client::new(tls_stream);

    let mut session = client
        .login(username, password)
        .await
        .map_err(|e| format!("로그인 실패: {} — 앱 비밀번호를 사용해야 합니다", e.0))?;

    let mailbox = session
        .select("INBOX")
        .await
        .map_err(|e| format!("INBOX 접근 실패: {}", e))?;

    let total = mailbox.exists;
    let _ = session.logout().await;

    Ok(format!("✅ 연결 성공! INBOX에 {}통의 메일이 있습니다.", total))
}

// ── TCP Connect using futures-io compatible async I/O ──
// We use a tokio TcpStream wrapped with `Compat` for futures-io compatibility
async fn async_std_tcp_connect(addr: &str) -> Result<impl futures::io::AsyncRead + futures::io::AsyncWrite + Unpin + std::fmt::Debug + Send, String> {
    use tokio::net::TcpStream;
    use tokio_util::compat::TokioAsyncReadCompatExt;
    
    let tcp = TcpStream::connect(addr)
        .await
        .map_err(|e| format!("TCP 연결 실패: {} (서버: {})", e, addr))?;
    
    Ok(tcp.compat())
}

// ── Helpers ──

fn parse_from_header(from: &str) -> (String, String) {
    // "Name <email@example.com>" or just "email@example.com"
    if let Some(pos) = from.find('<') {
        let name = from[..pos].trim().trim_matches('"').to_string();
        let email = from[pos+1..].trim_matches('>').trim().to_string();
        let display_name = if name.is_empty() { email.clone() } else { name };
        (display_name, email)
    } else {
        (from.trim().to_string(), from.trim().to_string())
    }
}

fn extract_body(parsed: &ParsedMail) -> (String, Option<String>) {
    let mut text_body = String::new();
    let mut html_body: Option<String> = None;

    if parsed.subparts.is_empty() {
        // Single part email
        let content_type = parsed.ctype.mimetype.as_str();
        if let Ok(body) = parsed.get_body() {
            if content_type.starts_with("text/html") {
                html_body = Some(body.clone());
                // Strip HTML tags for text summary
                text_body = strip_html_tags(&body);
            } else {
                text_body = body;
            }
        }
    } else {
        // Multipart email
        for part in &parsed.subparts {
            extract_body_recursive(part, &mut text_body, &mut html_body);
        }
    }

    // Truncate for summary
    if text_body.chars().count() > 500 {
        text_body = text_body.chars().take(500).collect();
    }

    (text_body.trim().to_string(), html_body)
}

fn extract_body_recursive(part: &ParsedMail, text: &mut String, html: &mut Option<String>) {
    let content_type = part.ctype.mimetype.as_str();

    if !part.subparts.is_empty() {
        for sub in &part.subparts {
            extract_body_recursive(sub, text, html);
        }
    } else if content_type == "text/plain" && text.is_empty() {
        if let Ok(body) = part.get_body() {
            *text = body;
        }
    } else if content_type == "text/html" && html.is_none() {
        if let Ok(body) = part.get_body() {
            *html = Some(body.clone());
            if text.is_empty() {
                *text = strip_html_tags(&body);
            }
        }
    }
}

fn extract_attachments(parsed: &ParsedMail, attachments: &mut Vec<FetchedAttachment>) {
    let disposition = parsed.get_content_disposition();
    let is_attachment = disposition.disposition == DispositionType::Attachment;
    let has_filename = disposition.params.get("filename").is_some() || parsed.ctype.params.get("name").is_some();
    
    // Some clients send attachments as inline without disposition header but with a filename.
    let content_type = parsed.ctype.mimetype.as_str();
    let is_body_text = content_type == "text/plain" || content_type == "text/html";

    if (is_attachment || has_filename) && !is_body_text {
        let filename = disposition.params.get("filename")
            .or_else(|| parsed.ctype.params.get("name"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown_file".to_string());
            
        if let Ok(data) = parsed.get_body_raw() {
            attachments.push(FetchedAttachment {
                filename,
                content_type: content_type.to_string(),
                size_bytes: data.len() as i64,
                data,
            });
        }
    }

    // Process subparts recursively
    for sub in &parsed.subparts {
        extract_attachments(sub, attachments);
    }
}

pub fn strip_html_tags(html: &str) -> String {
    let re_style = regex::Regex::new(r"(?is)<style.*?>.*?</style>").unwrap();
    let re_script = regex::Regex::new(r"(?is)<script.*?>.*?</script>").unwrap();
    let re_tags = regex::Regex::new(r"(?is)<.*?>").unwrap();

    let no_style = re_style.replace_all(html, " ");
    let no_script = re_script.replace_all(&no_style, " ");
    let no_tags = re_tags.replace_all(&no_script, " ");

    // Decode ALL HTML entities
    let decoded = decode_html_entities(&no_tags);

    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decode HTML entities: numeric (&#NNN; &#xHH;) and common named entities
fn decode_html_entities(input: &str) -> String {
    let re = regex::Regex::new(r"&(#x?[0-9a-fA-F]+|[a-zA-Z]+);").unwrap();
    
    re.replace_all(input, |caps: &regex::Captures| {
        let entity = &caps[1];
        
        if let Some(hex) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
            // Hex numeric entity: &#xHH;
            if let Ok(code) = u32::from_str_radix(hex, 16) {
                return char_or_strip(code);
            }
        } else if let Some(dec) = entity.strip_prefix('#') {
            // Decimal numeric entity: &#NNN;
            if let Ok(code) = dec.parse::<u32>() {
                return char_or_strip(code);
            }
        } else {
            // Named entity
            return match entity {
                "nbsp" | "ensp" | "emsp" | "thinsp" => " ".to_string(),
                "amp" => "&".to_string(),
                "lt" => "<".to_string(),
                "gt" => ">".to_string(),
                "quot" => "\"".to_string(),
                "apos" => "'".to_string(),
                "shy" | "zwj" | "zwnj" | "lrm" | "rlm" => String::new(), // invisible chars
                "ndash" => "–".to_string(),
                "mdash" => "—".to_string(),
                "bull" | "middot" => "·".to_string(),
                "hellip" => "…".to_string(),
                "laquo" => "«".to_string(),
                "raquo" => "»".to_string(),
                "ldquo" => "\u{201C}".to_string(),
                "rdquo" => "\u{201D}".to_string(),
                "lsquo" => "\u{2018}".to_string(),
                "rsquo" | "rsquor" => "\u{2019}".to_string(),
                "copy" => "©".to_string(),
                "reg" => "®".to_string(),
                "trade" => "™".to_string(),
                "times" => "×".to_string(),
                "divide" => "÷".to_string(),
                "euro" => "€".to_string(),
                "pound" => "£".to_string(),
                "yen" => "¥".to_string(),
                "cent" => "¢".to_string(),
                _ => caps[0].to_string(), // Unknown entity: keep as-is
            };
        }
        caps[0].to_string()
    }).to_string()
}

/// Convert a Unicode codepoint to a character, or strip if it's invisible/zero-width
fn char_or_strip(code: u32) -> String {
    match code {
        // Invisible / zero-width / formatting characters — strip them
        0x00AD | // soft hyphen (&shy;)
        0x034F | // combining grapheme joiner (&#847;)
        0x200B | // zero-width space
        0x200C | // zero-width non-joiner
        0x200D | // zero-width joiner
        0x200E | // left-to-right mark
        0x200F | // right-to-left mark
        0x2028 | // line separator
        0x2029 | // paragraph separator
        0x202A..=0x202E | // bidi formatting
        0x2060..=0x2064 | // invisible operators
        0xFE00..=0xFE0F | // variation selectors
        0xFEFF   // BOM / zero-width no-break space
            => String::new(),
        // Various spaces → single space
        0x2007 | // figure space (&#8199;)
        0x2002 | // en space
        0x2003 | // em space
        0x2004..=0x200A | // various thin/hair spaces
        0x00A0   // non-breaking space
            => " ".to_string(),
        _ => {
            if let Some(c) = char::from_u32(code) {
                c.to_string()
            } else {
                String::new()
            }
        }
    }
}

fn parse_email_date(date_str: &str) -> String {
    // Try to parse RFC2822 date and convert to ISO with timezone offset
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(date_str) {
        dt.to_rfc3339()
    } else {
        // Fallback: use current time in UTC
        chrono::Utc::now().to_rfc3339()
    }
}
