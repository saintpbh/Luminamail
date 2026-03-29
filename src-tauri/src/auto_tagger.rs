// ═══════════════════════════════════════════════════
// Auto-Tagger Module
// Provides keyword-based and AI-powered automatic
// tagging for incoming emails
// ═══════════════════════════════════════════════════

use std::collections::HashSet;

// ╔═══════════════════════════════════════════════╗
// ║   KEYWORD-BASED AUTO TAGGER (FREE)            ║
// ╚═══════════════════════════════════════════════╝

/// Tag rules: each rule has a tag name, icon, and keyword patterns
struct TagRule {
    tag: &'static str,
    icon: &'static str,
    keywords: &'static [&'static str],
}

const TAG_RULES: &[TagRule] = &[
    // ── Business / Finance ──
    TagRule { tag: "결제", icon: "💰", keywords: &["결제", "입금", "송금", "이체", "금액", "원)", "₩", "payment", "invoice", "인보이스"] },
    TagRule { tag: "견적", icon: "📊", keywords: &["견적", "견적서", "quotation", "estimate", "예산"] },
    TagRule { tag: "계약", icon: "📝", keywords: &["계약", "계약서", "contract", "서명", "날인", "MOU"] },
    TagRule { tag: "세금", icon: "🏛️", keywords: &["세금", "세금계산서", "부가세", "VAT", "tax", "국세청", "홈택스"] },
    TagRule { tag: "보고서", icon: "📄", keywords: &["보고서", "리포트", "report", "월간보고", "주간보고", "실적"] },

    // ── Work / Collaboration ──
    TagRule { tag: "회의", icon: "📅", keywords: &["회의", "미팅", "meeting", "캘린더", "일정", "킥오프", "스탠드업", "참석", "zoom", "teams"] },
    TagRule { tag: "승인", icon: "✅", keywords: &["승인", "결재", "approval", "confirm", "확인 요청", "처리 요청", "검토 요청"] },
    TagRule { tag: "업무요청", icon: "📋", keywords: &["요청", "부탁", "처리해", "진행해", "작업", "task", "assign", "배정", "담당"] },
    TagRule { tag: "피드백", icon: "💬", keywords: &["피드백", "의견", "리뷰", "검토", "feedback", "review", "수정", "코멘트"] },
    TagRule { tag: "프로젝트", icon: "🗂️", keywords: &["프로젝트", "project", "마일스톤", "milestone", "진행률", "WBS", "간트"] },
    TagRule { tag: "채용", icon: "👔", keywords: &["채용", "면접", "이력서", "resume", "지원", "recruit", "hire", "합격", "불합격"] },

    // ── Urgent / Important ──
    TagRule { tag: "긴급", icon: "🚨", keywords: &["긴급", "urgent", "ASAP", "즉시", "비상", "emergency", "당장", "critical"] },
    TagRule { tag: "마감", icon: "⏰", keywords: &["마감", "deadline", "기한", "만료", "due date", "D-day", "D-"] },

    // ── Communication ──
    TagRule { tag: "감사", icon: "🙏", keywords: &["감사", "고맙", "thank", "수고", "축하", "congratulation", "환영"] },
    TagRule { tag: "사과", icon: "😔", keywords: &["죄송", "사과", "sorry", "실수", "착오", "apologize"] },
    TagRule { tag: "공지", icon: "📢", keywords: &["공지", "안내", "notice", "알림", "변경사항", "announcement", "공문"] },
    TagRule { tag: "초대", icon: "🎉", keywords: &["초대", "invite", "참여", "행사", "이벤트", "event", "세미나", "웨비나", "컨퍼런스"] },

    // ── Tech / System ──
    TagRule { tag: "서버", icon: "🖥️", keywords: &["서버", "server", "배포", "deploy", "장애", "downtime", "오류", "에러", "버그", "bug"] },
    TagRule { tag: "보안", icon: "🔒", keywords: &["보안", "security", "비밀번호", "password", "인증", "해킹", "취약점", "2FA"] },
    TagRule { tag: "업데이트", icon: "🔄", keywords: &["업데이트", "update", "패치", "버전", "version", "릴리즈", "release"] },

    // ── Marketing / Sales ──
    TagRule { tag: "마케팅", icon: "📣", keywords: &["마케팅", "marketing", "캠페인", "campaign", "광고", "프로모션", "promotion", "이벤트"] },
    TagRule { tag: "고객", icon: "👤", keywords: &["고객", "customer", "클레임", "문의", "CS", "VOC", "불만", "요구사항"] },
    TagRule { tag: "영업", icon: "💼", keywords: &["영업", "sales", "제안서", "proposal", "RFP", "입찰", "수주"] },

    // ── Personal / Misc ──
    TagRule { tag: "교회", icon: "⛪", keywords: &["교회", "예배", "기도", "설교", "선교", "찬양", "성경", "목사", "집사"] },
    TagRule { tag: "택배", icon: "📦", keywords: &["택배", "배송", "delivery", "shipping", "운송장", "tracking", "도착"] },
    TagRule { tag: "구독", icon: "📩", keywords: &["뉴스레터", "newsletter", "구독", "subscribe", "unsubscribe", "수신거부"] },
    TagRule { tag: "여행", icon: "✈️", keywords: &["여행", "항공", "호텔", "예약", "booking", "flight", "trip", "숙소"] },
    TagRule { tag: "교육", icon: "📚", keywords: &["교육", "강의", "수업", "학습", "course", "학원", "세미나", "워크숍"] },

    // ── Attachments ──
    TagRule { tag: "첨부", icon: "📎", keywords: &["첨부", "attachment", "파일", "다운로드", "첨부파일"] },
    TagRule { tag: "이미지", icon: "🖼️", keywords: &[".jpg", ".png", ".gif", ".jpeg", ".webp", "이미지", "사진", "스크린샷"] },
    TagRule { tag: "문서", icon: "📁", keywords: &[".pdf", ".doc", ".docx", ".xlsx", ".pptx", ".hwp", "문서"] },
];

/// Generate tags from email subject + body using keyword matching
pub fn auto_tag_basic(subject: &str, body: &str) -> Vec<String> {
    let mut tags = HashSet::new();
    let combined = format!("{} {}", subject, body).to_lowercase();

    for rule in TAG_RULES {
        for keyword in rule.keywords {
            if combined.contains(&keyword.to_lowercase()) {
                tags.insert(format!("{}{}", rule.icon, rule.tag));
                break; // One match per rule is enough
            }
        }
    }

    // Limit to max 5 tags to keep it clean
    let mut result: Vec<String> = tags.into_iter().collect();
    result.sort();
    result.truncate(5);
    result
}

// ╔═══════════════════════════════════════════════╗
// ║   AI-POWERED AUTO TAGGER (GEMINI)             ║
// ╚═══════════════════════════════════════════════╝

/// Generate tags using Google Gemini API (requires API key)
pub async fn auto_tag_ai(subject: &str, body: &str, api_key: &str) -> Result<Vec<String>, String> {
    let prompt = format!(
        "다음 이메일의 내용을 분석해서 적절한 태그를 2~5개 생성해줘.\n\
         태그는 한국어로, 각 태그 앞에 적절한 이모지를 붙여줘.\n\
         태그만 쉼표로 구분해서 응답해. 다른 설명은 하지마.\n\
         태그 예시: 💰결제, 📅회의, 🚨긴급, ✅승인, 📎첨부\n\n\
         제목: {}\n본문: {}",
        subject,
        &body.chars().take(500).collect::<String>() // Safe char-level truncation
    );

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    let response = client.post(&url)
        .json(&serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {
                "temperature": 0.3,
                "maxOutputTokens": 100
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Gemini API error: {}", e))?;

    let json: serde_json::Value = response.json().await.map_err(|e| format!("Parse error: {}", e))?;

    // Extract text from Gemini response
    let text = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    // Parse comma-separated tags
    let tags: Vec<String> = text.split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty() && t.len() > 1)
        .take(5)
        .collect();

    if tags.is_empty() {
        // Fallback to basic if AI returns nothing useful
        Ok(auto_tag_basic(subject, body))
    } else {
        Ok(tags)
    }
}

/// Auto-tag a message, choosing AI or basic mode based on API key availability
pub async fn auto_tag(subject: &str, body: &str, ai_api_key: Option<&str>) -> Vec<String> {
    if let Some(key) = ai_api_key {
        if !key.is_empty() {
            match auto_tag_ai(subject, body, key).await {
                Ok(tags) => return tags,
                Err(e) => {
                    eprintln!("AI tagging failed, falling back to basic: {}", e);
                }
            }
        }
    }
    auto_tag_basic(subject, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tagging() {
        let tags = auto_tag_basic("3월 인보이스 발송", "총 금액: ₩15,400,000 결제 기한: 4월 15일");
        assert!(tags.iter().any(|t| t.contains("결제")));

        let tags2 = auto_tag_basic("[긴급] 서버 점검 안내", "오늘 22시~24시 서버 점검이 예정되어 있습니다");
        assert!(tags2.iter().any(|t| t.contains("긴급")));
        assert!(tags2.iter().any(|t| t.contains("서버")));
    }
}
