// ═══════════════════════════════════════════════════════════
// Lumina Mail - Cloud Upload (Google Drive / OneDrive)
// Upload large files to cloud storage and return share links
// ═══════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudToken {
    pub provider: String, // "gdrive" or "onedrive"
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadResult {
    pub provider: String,
    pub file_name: String,
    pub file_size: u64,
    pub share_link: String,
    pub cloud_file_id: String,
}

// ── OAuth2 Configuration ──
// NOTE: For production, register your own OAuth apps.
// These are placeholder client IDs for development.

const GDRIVE_CLIENT_ID: &str = "YOUR_GOOGLE_CLIENT_ID";
const GDRIVE_CLIENT_SECRET: &str = "YOUR_GOOGLE_CLIENT_SECRET";
const GDRIVE_REDIRECT_URI: &str = "http://localhost:17239/callback";

const ONEDRIVE_CLIENT_ID: &str = "YOUR_ONEDRIVE_CLIENT_ID";
const ONEDRIVE_REDIRECT_URI: &str = "http://localhost:17239/callback";

// ╔═══════════════════════════════════════════════╗
// ║   GOOGLE DRIVE UPLOAD                         ║
// ╚═══════════════════════════════════════════════╝

/// Generate Google Drive OAuth2 URL for user to authorize
pub fn gdrive_auth_url() -> String {
    format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
        client_id={}&\
        redirect_uri={}&\
        response_type=code&\
        scope=https://www.googleapis.com/auth/drive.file&\
        access_type=offline&\
        prompt=consent",
        GDRIVE_CLIENT_ID,
        urlencoding(GDRIVE_REDIRECT_URI)
    )
}

/// Exchange auth code for token
pub async fn gdrive_exchange_code(code: &str) -> Result<CloudToken, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", GDRIVE_CLIENT_ID),
            ("client_secret", GDRIVE_CLIENT_SECRET),
            ("redirect_uri", GDRIVE_REDIRECT_URI),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("토큰 교환 실패: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("응답 파싱 실패: {}", e))?;

    if let Some(error) = json.get("error") {
        return Err(format!("Google OAuth 오류: {}", error));
    }

    Ok(CloudToken {
        provider: "gdrive".to_string(),
        access_token: json["access_token"].as_str().unwrap_or("").to_string(),
        refresh_token: json.get("refresh_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
        expires_at: None,
    })
}

/// Upload file to Google Drive and return share link
pub async fn gdrive_upload(token: &str, file_path: &str) -> Result<UploadResult, String> {
    let path = PathBuf::from(file_path);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let file_bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("파일 읽기 실패: {}", e))?;
    let file_size = file_bytes.len() as u64;

    let client = reqwest::Client::new();

    // Step 1: Upload file using simple upload
    let metadata = serde_json::json!({
        "name": file_name,
        "parents": ["root"]
    });

    // Use multipart upload
    let boundary = "lumina_mail_boundary_12345";
    let mut body = Vec::new();

    // Metadata part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(metadata.to_string().as_bytes());
    body.extend_from_slice(b"\r\n");

    // File content part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(format!("Content-Type: application/octet-stream\r\n\r\n").as_bytes());
    body.extend_from_slice(&file_bytes);
    body.extend_from_slice(format!("\r\n--{}--", boundary).as_bytes());

    let upload_resp = client
        .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,webViewLink")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", format!("multipart/related; boundary={}", boundary))
        .body(body)
        .send()
        .await
        .map_err(|e| format!("업로드 실패: {}", e))?;

    let upload_json: serde_json::Value = upload_resp.json()
        .await
        .map_err(|e| format!("업로드 응답 파싱 실패: {}", e))?;

    if let Some(error) = upload_json.get("error") {
        return Err(format!("Google Drive 업로드 오류: {}", error));
    }

    let file_id = upload_json["id"].as_str().unwrap_or("").to_string();

    // Step 2: Create a public sharing permission
    let _perm_resp = client
        .post(format!("https://www.googleapis.com/drive/v3/files/{}/permissions", file_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "type": "anyone",
            "role": "reader"
        }))
        .send()
        .await
        .map_err(|e| format!("공유 설정 실패: {}", e))?;

    let share_link = upload_json["webViewLink"]
        .as_str()
        .unwrap_or(&format!("https://drive.google.com/file/d/{}/view?usp=sharing", file_id))
        .to_string();

    Ok(UploadResult {
        provider: "gdrive".to_string(),
        file_name,
        file_size,
        share_link,
        cloud_file_id: file_id,
    })
}

// ╔═══════════════════════════════════════════════╗
// ║   ONEDRIVE UPLOAD                             ║
// ╚═══════════════════════════════════════════════╝

/// Generate OneDrive OAuth2 URL
pub fn onedrive_auth_url() -> String {
    format!(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?\
        client_id={}&\
        redirect_uri={}&\
        response_type=code&\
        scope=Files.ReadWrite.All offline_access&\
        response_mode=query",
        ONEDRIVE_CLIENT_ID,
        urlencoding(ONEDRIVE_REDIRECT_URI)
    )
}

/// Exchange OneDrive auth code for token
pub async fn onedrive_exchange_code(code: &str) -> Result<CloudToken, String> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://login.microsoftonline.com/common/oauth2/v2.0/token")
        .form(&[
            ("code", code),
            ("client_id", ONEDRIVE_CLIENT_ID),
            ("redirect_uri", ONEDRIVE_REDIRECT_URI),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("토큰 교환 실패: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("응답 파싱 실패: {}", e))?;

    if let Some(error) = json.get("error") {
        return Err(format!("Microsoft OAuth 오류: {}", error));
    }

    Ok(CloudToken {
        provider: "onedrive".to_string(),
        access_token: json["access_token"].as_str().unwrap_or("").to_string(),
        refresh_token: json.get("refresh_token").and_then(|v| v.as_str()).map(|s| s.to_string()),
        expires_at: None,
    })
}

/// Upload file to OneDrive and return share link
pub async fn onedrive_upload(token: &str, file_path: &str) -> Result<UploadResult, String> {
    let path = PathBuf::from(file_path);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();

    let file_bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("파일 읽기 실패: {}", e))?;
    let file_size = file_bytes.len() as u64;

    let client = reqwest::Client::new();

    // Upload file (< 4MB: simple upload, >= 4MB: would need upload session — simplified here)
    let upload_url = format!(
        "https://graph.microsoft.com/v1.0/me/drive/root:/LuminaMail/{}:/content",
        file_name
    );

    let upload_resp = client
        .put(&upload_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/octet-stream")
        .body(file_bytes)
        .send()
        .await
        .map_err(|e| format!("OneDrive 업로드 실패: {}", e))?;

    let upload_json: serde_json::Value = upload_resp.json()
        .await
        .map_err(|e| format!("업로드 응답 파싱 실패: {}", e))?;

    if let Some(error) = upload_json.get("error") {
        return Err(format!("OneDrive 오류: {}", error));
    }

    let file_id = upload_json["id"].as_str().unwrap_or("").to_string();

    // Create sharing link
    let share_resp = client
        .post(format!(
            "https://graph.microsoft.com/v1.0/me/drive/items/{}/createLink",
            file_id
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "type": "view",
            "scope": "anonymous"
        }))
        .send()
        .await
        .map_err(|e| format!("공유 링크 생성 실패: {}", e))?;

    let share_json: serde_json::Value = share_resp.json()
        .await
        .map_err(|e| format!("공유 응답 파싱 실패: {}", e))?;

    let share_link = share_json["link"]["webUrl"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(UploadResult {
        provider: "onedrive".to_string(),
        file_name,
        file_size,
        share_link,
        cloud_file_id: file_id,
    })
}

// ╔═══════════════════════════════════════════════╗
// ║   LOCAL OAUTH CALLBACK SERVER                 ║
// ╚═══════════════════════════════════════════════╝

/// Start a temporary HTTP server to receive OAuth callback
pub async fn wait_for_oauth_callback() -> Result<String, String> {
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = TcpListener::bind("127.0.0.1:17239")
        .await
        .map_err(|e| format!("콜백 서버 시작 실패: {}", e))?;

    // Wait for the callback (with timeout)
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(300), // 5 minute timeout
        async {
            let (mut socket, _) = listener.accept()
                .await
                .map_err(|e| format!("콜백 연결 실패: {}", e))?;

            let mut buf = vec![0u8; 4096];
            let n = socket.read(&mut buf)
                .await
                .map_err(|e| format!("콜백 읽기 실패: {}", e))?;

            let request = String::from_utf8_lossy(&buf[..n]).to_string();

            // Extract auth code from URL: GET /callback?code=xxx&...
            let code = request
                .lines()
                .next()
                .and_then(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let path = parts[1];
                        if let Some(query) = path.split('?').nth(1) {
                            for param in query.split('&') {
                                let kv: Vec<&str> = param.split('=').collect();
                                if kv.len() == 2 && kv[0] == "code" {
                                    return Some(kv[1].to_string());
                                }
                            }
                        }
                    }
                    None
                })
                .ok_or_else(|| "인증 코드를 받지 못했습니다".to_string())?;

            // Send success response to browser
            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                <html><body style='font-family:sans-serif;text-align:center;margin-top:60px;background:#1a1a2e;color:white'>\
                <h2>✅ Lumina Mail 인증 완료!</h2>\
                <p>이 창을 닫고 Lumina Mail로 돌아가세요.</p>\
                <script>setTimeout(()=>window.close(),2000)</script>\
                </body></html>";

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.flush().await;

            Ok::<String, String>(code)
        }
    ).await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err("인증 시간 초과 (5분)".to_string()),
    }
}

// ── URL encoding helper ──
fn urlencoding(s: &str) -> String {
    s.replace(':', "%3A").replace('/', "%2F").replace(' ', "%20")
}
