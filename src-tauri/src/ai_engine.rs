// ═══════════════════════════════════════════════════
// AI Engine Module
// Unified Gemini AI processing for email intelligence:
// summarize, tag, classify importance, translate
// ═══════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiResult {
    pub summary: String,
    pub tags: Vec<String>,
    pub important: bool,
    pub needs_action: bool,
    pub translation: Option<String>,
}

/// Process a single email through the unified AI prompt.
/// Returns summary, tags, importance, and optional translation in one API call.
pub async fn process_email(
    subject: &str,
    body: &str,
    api_key: &str,
    model: &str,
    translate: bool,
) -> Result<AiResult, String> {
    // Truncate body to ~1500 bytes safely at a char boundary
    let truncate_at = body.char_indices()
        .take_while(|(i, _)| *i < 1500)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(body.len().min(1500));
    let body_truncated = &body[..truncate_at];

    let translate_instruction = if translate {
        "  \"translation\": \"본문의 핵심 내용을 한국어로 번역 (이미 한국어면 영어로 번역)\","
    } else {
        "  \"translation\": null,"
    };

    let prompt = format!(
        r#"당신은 이메일 분석 AI입니다. 다음 이메일을 분석하여 정확히 아래 JSON 형식으로만 응답하세요.
다른 텍스트나 마크다운 없이 순수 JSON만 출력하세요.

{{
  "summary": "메일 핵심 내용 1~2문장 요약",
  "tags": ["이모지+태그1", "이모지+태그2", ...최대5개],
{translate_instruction}
  "important": true 또는 false (공문, 계약, 결제, 승인요청 등이면 true),
  "needs_action": true 또는 false (회신, 처리, 확인 등 후속 조치가 필요하면 true)
}}

태그 예시: "💰결제", "📅회의", "🚨긴급", "✅승인", "📎첨부", "📢공지"

이메일:
제목: {subject}
본문: {body_truncated}"#
    );

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = client.post(&url)
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": 0.2,
                "maxOutputTokens": 300,
                "responseMimeType": "application/json"
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Gemini API 요청 실패: {}", e))?;

    let json: serde_json::Value = response.json().await
        .map_err(|e| format!("응답 파싱 실패: {}", e))?;

    // Extract text from Gemini response
    let text = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("{}")
        .trim();

    // Parse the JSON response
    let result: AiResult = serde_json::from_str(text).unwrap_or_else(|_| {
        // Try to extract from markdown code block if wrapped
        let cleaned = text.trim_start_matches("```json").trim_start_matches("```")
            .trim_end_matches("```").trim();
        serde_json::from_str(cleaned).unwrap_or_default()
    });

    Ok(result)
}

/// Translate a single email body
pub async fn translate_email(
    body: &str,
    api_key: &str,
    model: &str,
) -> Result<String, String> {
    let body_truncated: String = body.chars().take(2000).collect();

    let prompt = format!(
        "다음 이메일 본문을 번역해주세요. 한국어면 영어로, 영어면 한국어로 번역하세요. 번역문만 출력하세요.\n\n{}",
        body_truncated
    );

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = client.post(&url)
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 500
            }
        }))
        .send()
        .await
        .map_err(|e| format!("번역 API 요청 실패: {}", e))?;

    let json: serde_json::Value = response.json().await
        .map_err(|e| format!("번역 응답 파싱 실패: {}", e))?;

    let text = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    Ok(text)
}
