use sqlx::SqlitePool;

pub async fn seed_mock_data(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Check if data already exists
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chat_rooms")
        .fetch_one(pool)
        .await?;

    if count.0 > 0 {
        return Ok(());
    }

    // --- Chat Rooms ---
    seed_rooms(pool).await?;

    // --- Messages ---
    seed_thread_001(pool).await?;
    seed_thread_002(pool).await?;
    seed_thread_003(pool).await?;
    seed_thread_004(pool).await?;
    seed_thread_005(pool).await?;
    seed_thread_006(pool).await?;
    seed_thread_007(pool).await?;
    seed_thread_008(pool).await?;

    Ok(())
}

async fn insert_room(pool: &SqlitePool, id: &str, subject: &str, sender: &str, status: &str, time: &str, pinned: i32, unread: i32, preview: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO chat_rooms (thread_id, subject, status, last_received_at, sender_name, pinned, important, unread_count, last_message_preview)
         VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)"
    )
    .bind(id).bind(subject).bind(status).bind(time).bind(sender).bind(pinned).bind(unread).bind(preview)
    .execute(pool).await?;
    Ok(())
}

async fn insert_msg(
    pool: &SqlitePool, id: &str, thread_id: &str, msg_type: &str,
    sender_id: &str, sender_name: &str, summary: &str,
    original: Option<&str>, icon: Option<&str>, emoji: Option<&str>,
    outgoing: bool, time: &str,
) -> Result<(), sqlx::Error> {
    // Auto-generate tags from content
    let tags = crate::auto_tagger::auto_tag_basic(summary, original.unwrap_or(""));
    let tags_str = if tags.is_empty() { None } else { Some(tags.join(",")) };

    sqlx::query(
        "INSERT INTO messages (id, thread_id, message_type, sender_identity, sender_name, body_summary, body_original, icon_type, emoji_tag, hashtags, is_outgoing, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(thread_id).bind(msg_type).bind(sender_id).bind(sender_name)
    .bind(summary).bind(original).bind(icon).bind(emoji).bind(&tags_str).bind(outgoing).bind(time)
    .execute(pool).await?;
    Ok(())
}

async fn insert_attachment(
    pool: &SqlitePool, id: &str, message_id: &str, filename: &str,
    content_type: &str, size_bytes: i64, main_type: &str, time: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO attachments (id, message_id, filename, content_type, size_bytes, local_path, thumbnail_path, main_type, created_at)
         VALUES (?, ?, ?, ?, ?, NULL, NULL, ?, ?)"
    )
    .bind(id).bind(message_id).bind(filename).bind(content_type).bind(size_bytes).bind(main_type).bind(time)
    .execute(pool).await?;
    Ok(())
}

async fn seed_rooms(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_room(pool, "thread-001", "프로젝트 킥오프 미팅", "김선교 (더미)", "open", "2026-03-28T09:00:00", 1, 3, "다음 주 월요일 10시에 킥오프 미팅 진행하겠습니다.").await?;
    insert_room(pool, "thread-002", "3월 재정 보고서 검토 요청", "이재무 (더미)", "progress", "2026-03-27T15:30:00", 0, 1, "첨부된 보고서 검토 부탁드립니다.").await?;
    insert_room(pool, "thread-003", "선교 뉴스레터 Vol.42", "글로벌미션 (더미)", "open", "2026-03-27T11:00:00", 0, 0, "이번 달 선교 소식을 전해드립니다.").await?;
    insert_room(pool, "thread-004", "[긴급] 서버 점검 안내", "시스템관리자 (더미)", "open", "2026-03-28T07:15:00", 1, 2, "오늘 22시~24시 서버 점검이 예정되어 있습니다.").await?;
    insert_room(pool, "thread-005", "팀 회식 장소 투표", "박팀장 (더미)", "closed", "2026-03-26T18:00:00", 0, 0, "회식 장소가 강남역 근처로 결정되었습니다.").await?;
    insert_room(pool, "thread-006", "디자인 시안 피드백", "최디자인 (더미)", "progress", "2026-03-28T06:45:00", 0, 5, "v3 시안 첨부합니다. 피드백 부탁드려요.").await?;
    insert_room(pool, "thread-007", "기도 제목 나눔", "사랑의교회 (더미)", "open", "2026-03-27T20:00:00", 0, 0, "이번 주 기도 제목을 나눕니다.").await?;
    insert_room(pool, "thread-008", "인보이스 #2026-0342", "거래처A (더미)", "open", "2026-03-28T05:00:00", 0, 1, "3월분 인보이스를 보내드립니다.").await?;
    Ok(())
}

async fn seed_thread_001(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-001-01", "thread-001", "email", "kim@company.com", "김선교",
        "프로젝트 킥오프 미팅 일정을 공유드립니다. 다음 주 월요일 10시에 3층 회의실에서 진행합니다. 킥오프_발표자료.pptx 내용 확인 부탁드립니다.",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h2>프로젝트 킥오프 미팅</h2><p>안녕하세요 팀원 여러분,</p><p>다음 주 월요일(4/4) 오전 10시에 <b>3층 대회의실</b>에서 프로젝트 킥오프 미팅을 진행합니다.</p><h3>📋 안건</h3><ul><li>프로젝트 개요 및 목표 공유</li><li>역할 분담 논의</li><li>마일스톤 일정 확인</li><li>Q&A</li></ul><p>참석이 어려우신 분은 미리 말씀해 주세요.</p><p>감사합니다,<br/>김선교 드림</p></div>"),
        Some("document"), Some("📄"), false, "2026-03-27T14:00:00").await?;
        
    insert_attachment(pool, "att-001-01-1", "msg-001-01", "킥오프_발표자료.pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation", 8450123, "document", "2026-03-27T14:00:00").await?;

    insert_msg(pool, "msg-001-02", "thread-001", "email", "me@company.com", "나",
        "네, 확인했습니다. 참석하겠습니다!",
        Some("<p>네, 확인했습니다. 참석하겠습니다!</p>"),
        None, None, true, "2026-03-27T14:30:00").await?;

    insert_msg(pool, "msg-001-03", "thread-001", "internal_comment", "park@company.com", "박팀장",
        "김선교님 발표 자료 미리 받아둘 수 있을까요?",
        None, None, None, false, "2026-03-27T15:00:00").await?;

    insert_msg(pool, "msg-001-04", "thread-001", "email", "lee@company.com", "이재무",
        "혹시 예산 관련 논의도 포함되나요? 작년 대비 20% 증가분에 대한 승인이 필요합니다. 예산계획안.xlsx 첨부합니다.",
        Some("<div style='font-family: sans-serif; padding: 20px;'><p>김선교님,</p><p>킥오프 미팅에 예산 관련 논의도 포함되면 좋겠습니다.</p><p>작년 대비 <span style='color: red; font-weight: bold;'>20% 증가분</span>에 대한 경영진 승인이 필요한 상황입니다.</p><p>관련 자료는 첨부해 드리겠습니다.</p><p>이재무 드림</p></div>"),
        Some("document"), Some("💰"), false, "2026-03-27T16:00:00").await?;
        
    insert_attachment(pool, "att-001-04-1", "msg-001-04", "예산계획안.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", 1254300, "document", "2026-03-27T16:00:00").await?;

    insert_msg(pool, "msg-001-05", "thread-001", "email", "kim@company.com", "김선교",
        "네, 예산 논의도 안건에 추가하겠습니다. 다음 주 월요일 10시에 뵙겠습니다.",
        Some("<p>이재무님, 네 예산 논의도 안건에 추가하겠습니다. 다음 주 월요일 10시에 뵙겠습니다.</p>"),
        None, None, false, "2026-03-28T09:00:00").await?;
    Ok(())
}

async fn seed_thread_002(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-002-01", "thread-002", "email", "lee@company.com", "이재무",
        "3월 재정 보고서를 첨부합니다. 주요 변동사항 3건이 있으니 검토 부탁드립니다. 첨부: 26년_3월_결산보고.pdf",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h2>📊 3월 재정 보고서</h2><p>안녕하세요,</p><p>3월 재정 보고서를 첨부해 드립니다.</p><h3>주요 변동사항</h3><table border='1' cellpadding='8' cellspacing='0' style='border-collapse: collapse; width: 100%;'><tr style='background: #f0f0f0;'><th>항목</th><th>예산</th><th>실적</th><th>차이</th></tr><tr><td>인건비</td><td>5,000만원</td><td>4,800만원</td><td style='color: green;'>-200만원</td></tr><tr><td>운영비</td><td>2,000만원</td><td>2,350만원</td><td style='color: red;'>+350만원</td></tr><tr><td>마케팅</td><td>1,500만원</td><td>1,200만원</td><td style='color: green;'>-300만원</td></tr></table><p>운영비 초과 사유는 별도 보고서를 참고해 주세요.</p></div>"),
        Some("document"), Some("💰"), false, "2026-03-27T15:00:00").await?;
        
    insert_attachment(pool, "att-002-01-1", "msg-002-01", "26년_3월_결산보고.pdf", "application/pdf", 450123, "document", "2026-03-27T15:00:00").await?;

    insert_msg(pool, "msg-002-02", "thread-002", "internal_comment", "me@company.com", "나",
        "운영비 초과 건은 경영지원팀에 확인 필요",
        None, None, None, true, "2026-03-27T15:15:00").await?;

    insert_msg(pool, "msg-002-03", "thread-002", "email", "me@company.com", "나",
        "확인했습니다. 운영비 초과분에 대해 상세 내역 보내주실 수 있나요?",
        Some("<p>이재무님, 확인했습니다. 운영비 초과분에 대해 상세 내역 보내주실 수 있나요?</p>"),
        None, None, true, "2026-03-27T15:30:00").await?;
    Ok(())
}

async fn seed_thread_003(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-003-01", "thread-003", "email", "newsletter@globalmission.org", "글로벌미션",
        "이번 달 선교 소식: 동남아시아 3개국 새 사역지 오픈, 아프리카 식수 프로젝트 완료 보고",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h1 style='color: #2c5282;'>🌐 글로벌미션 뉴스레터 Vol.42</h1><h2>이번 달 주요 소식</h2><h3>🇹🇭 동남아시아 새 사역지 오픈</h3><p>태국, 캄보디아, 미얀마에 새로운 사역지가 오픈되었습니다.</p><h3>🇰🇪 아프리카 식수 프로젝트 완료</h3><p>케냐 나이로비 외곽 3개 마을에 깨끗한 식수 시설이 완공되었습니다.</p><h3>🙏 기도 제목</h3><ul><li>새 사역지 현지 사역자들의 건강과 안전</li><li>식수 시설 유지보수를 위한 현지 인력 양성</li></ul></div>"),
        Some("globe"), Some("🙏"), false, "2026-03-27T11:00:00").await?;
    Ok(())
}

async fn seed_thread_004(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-004-01", "thread-004", "email", "admin@company.com", "시스템관리자",
        "🚨 오늘 22시~24시 정기 서버 점검이 예정되어 있습니다. 중요 작업은 미리 저장해 주세요.",
        Some("<div style='font-family: sans-serif; padding: 20px; border-left: 4px solid #ff3b30;'><h2>🚨 [긴급] 서버 점검 안내</h2><p>안녕하세요, 시스템관리팀입니다.</p><p><b>오늘(3/28) 22:00 ~ 24:00</b> 동안 정기 서버 점검이 진행됩니다.</p><h3>영향 범위</h3><ul><li>사내 메일 시스템 (약 30분 다운타임)</li><li>파일 서버 (약 1시간 다운타임)</li><li>VPN 접속 (일시적 불안정)</li></ul><p style='color: red; font-weight: bold;'>⚠️ 중요 작업은 21:30까지 저장해 주세요.</p><p>문의: 시스템관리팀 내선 1234</p></div>"),
        Some("document"), Some("🚨"), false, "2026-03-28T07:00:00").await?;

    insert_msg(pool, "msg-004-02", "thread-004", "email", "me@company.com", "나",
        "확인했습니다. 감사합니다.",
        Some("<p>확인했습니다. 감사합니다.</p>"),
        None, None, true, "2026-03-28T07:15:00").await?;
    Ok(())
}

async fn seed_thread_005(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-005-01", "thread-005", "email", "park@company.com", "박팀장",
        "다음 주 금요일 팀 회식 장소를 투표로 정하겠습니다. 1번: 강남역 한식 2번: 역삼역 이탈리안 3번: 선릉역 일식",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h2>🍽️ 팀 회식 장소 투표</h2><p>다음 주 금요일 저녁 회식 장소를 정하겠습니다!</p><ol><li>강남역 한식 - 고기리막국수</li><li>역삼역 이탈리안 - 라 피아짜</li><li>선릉역 일식 - 스시히로바</li></ol><p>금요일까지 투표해 주세요! 🗳️</p></div>"),
        Some("letter"), None, false, "2026-03-26T14:00:00").await?;

    insert_msg(pool, "msg-005-02", "thread-005", "email", "me@company.com", "나",
        "1번 강남역 한식에 한 표!",
        Some("<p>1번 강남역 한식에 한 표! 🙋‍♂️</p>"),
        None, None, true, "2026-03-26T15:00:00").await?;

    insert_msg(pool, "msg-005-03", "thread-005", "email", "park@company.com", "박팀장",
        "투표 결과: 1번 강남역 한식으로 결정! 다음 주 금요일 18:30에 만나요.",
        Some("<p>투표 결과 1번 강남역 한식이 가장 많은 표를 받았습니다! 🎉<br/>다음 주 금요일 18:30에 강남역 2번 출구에서 만나요.</p>"),
        None, None, false, "2026-03-26T18:00:00").await?;
    Ok(())
}

async fn seed_thread_006(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-006-01", "thread-006", "email", "choi@company.com", "최디자인",
        "v3 디자인 시안을 공유합니다. 주요 변경: 메인 컬러 변경, 레이아웃 개선, 모바일 반응형 추가. 첨부: v3_메인화면_시안.png",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h2>🎨 디자인 시안 v3</h2><p>안녕하세요, v3 시안을 공유합니다.</p><h3>주요 변경사항</h3><ol><li>메인 컬러: #007AFF → #5856D6 (보라색 계열)</li><li>대시보드 레이아웃 전면 개편</li><li>모바일 반응형 디자인 추가</li></ol><p>피드백은 금요일까지 부탁드립니다.</p></div>"),
        Some("document"), Some("📄"), false, "2026-03-28T06:00:00").await?;
        
    insert_attachment(pool, "att-006-01-1", "msg-006-01", "v3_메인화면_시안.png", "image/png", 3450123, "image", "2026-03-28T06:00:00").await?;

    insert_msg(pool, "msg-006-02", "thread-006", "internal_comment", "park@company.com", "박팀장",
        "보라색이 브랜드 가이드에 맞는지 마케팅팀 확인 필요",
        None, None, None, false, "2026-03-28T06:15:00").await?;

    insert_msg(pool, "msg-006-03", "thread-006", "email", "me@company.com", "나",
        "전체적으로 좋습니다! 메인 컬러만 기존 파란색 유지하면 어떨까요?",
        Some("<p>최디자인님, 전체적으로 매우 좋습니다! 다만 메인 컬러는 기존 파란색(#007AFF)을 유지하면 어떨까요? 브랜드 일관성 측면에서요.</p>"),
        None, None, true, "2026-03-28T06:30:00").await?;

    insert_msg(pool, "msg-006-04", "thread-006", "email", "choi@company.com", "최디자인",
        "네, 파란색으로 수정하겠습니다. 내일까지 v3.1 보내드리겠습니다.",
        Some("<p>네, 파란색으로 수정하겠습니다. 내일까지 v3.1 보내드리겠습니다. 😊</p>"),
        None, None, false, "2026-03-28T06:45:00").await?;
    Ok(())
}

async fn seed_thread_007(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-007-01", "thread-007", "email", "church@love.org", "사랑의교회",
        "이번 주 기도제목: 1) 새 성전 건축 기금 2) 선교사님들 건강 3) 청년부 부흥",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h2>🙏 이번 주 기도 제목</h2><ol><li>새 성전 건축 기금을 위해</li><li>해외 파송 선교사님들의 건강을 위해</li><li>청년부 부흥과 성장을 위해</li></ol><p>함께 기도해 주세요. 🙏</p></div>"),
        Some("letter"), Some("🙏"), false, "2026-03-27T20:00:00").await?;
    Ok(())
}

async fn seed_thread_008(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    insert_msg(pool, "msg-008-01", "thread-008", "email", "billing@partner-a.com", "거래처A",
        "3월분 인보이스 #2026-0342를 발송합니다. 총 금액: ₩15,400,000 / 결제 기한: 4월 15일. 첨부: Invoice_2026_0342.pdf",
        Some("<div style='font-family: sans-serif; padding: 20px;'><h2>인보이스 #2026-0342</h2><table border='1' cellpadding='8' cellspacing='0' style='border-collapse: collapse; width: 100%;'><tr style='background: #f0f0f0;'><th>항목</th><th>수량</th><th>단가</th><th>금액</th></tr><tr><td>컨설팅 서비스</td><td>40시간</td><td>₩250,000</td><td>₩10,000,000</td></tr><tr><td>기술 지원</td><td>20시간</td><td>₩200,000</td><td>₩4,000,000</td></tr><tr><td>기타 경비</td><td>1</td><td>₩1,400,000</td><td>₩1,400,000</td></tr></table><p style='font-size: 18px; font-weight: bold; margin-top: 16px;'>총 금액: ₩15,400,000</p><p>결제 기한: 2026년 4월 15일</p></div>"),
        Some("document"), Some("💰"), false, "2026-03-28T05:00:00").await?;
        
    insert_attachment(pool, "att-008-01-1", "msg-008-01", "Invoice_2026_0342.pdf", "application/pdf", 102450, "document", "2026-03-28T05:00:00").await?;
    Ok(())
}
