// ═══════════════════════════════════════════════════════════
// Lumina Mail - Component Renderers (v2: Full Features)
// Chat Bubbles, Thread Items, Media Cards, Briefings, Wiki
// ═══════════════════════════════════════════════════════════

const AVATAR_COLORS = ['color-0','color-1','color-2','color-3','color-4','color-5','color-6','color-7'];

function hashCode(str) {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash) + str.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash);
}

export function getAvatarColor(name) {
  return AVATAR_COLORS[hashCode(name) % AVATAR_COLORS.length];
}

function getInitial(name) {
  return name ? name.charAt(0).toUpperCase() : '?';
}

function formatTime(dateStr) {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now - date;
  const dayMs = 86400000;
  if (diff < dayMs && date.getDate() === now.getDate()) {
    return date.toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit', hour12: false });
  } else if (diff < 2 * dayMs) return '어제';
  else if (diff < 7 * dayMs) {
    return ['일','월','화','수','목','금','토'][date.getDay()] + '요일';
  } else return `${date.getMonth()+1}/${date.getDate()}`;
}

function formatFullDate(dateStr) {
  if (!dateStr) return '';
  return new Date(dateStr).toLocaleDateString('ko-KR', {
    year:'numeric', month:'long', day:'numeric', hour:'2-digit', minute:'2-digit', hour12:false,
  });
}

function formatDateLabel(dateStr) {
  if (!dateStr) return '';
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now - date;
  const dayMs = 86400000;
  if (diff < dayMs && date.getDate() === now.getDate()) return '오늘';
  else if (diff < 2 * dayMs) return '어제';
  else return date.toLocaleDateString('ko-KR', { month:'long', day:'numeric' });
}

// ─── 5. Custom Taxonomy Icons ───
function getCategoryIcon(thread) {
  const s = (thread.subject || '') + (thread.last_message_preview || '');
  if (/선교|기도|글로벌|mission/i.test(s)) return '🌐';
  if (/공문서|결재|인보이스|계약|보고서/i.test(s)) return '📄';
  if (/뉴스레터|소식|구독/i.test(s)) return '✉️';
  return '';
}

// Dynamic emoji based on content sentiment
function getDynamicEmoji(thread) {
  const s = (thread.subject || '') + (thread.last_message_preview || '');
  if (/긴급|점검|ASAP|즉시/i.test(s)) return '🚨';
  if (/축하|환영|감사|기쁘|성공/i.test(s)) return '🎉';
  if (/기도|위로|안타|슬프/i.test(s)) return '🙏';
  if (/승인|결재|확인 요청/i.test(s)) return '📋';
  if (/회의|미팅|킥오프/i.test(s)) return '📅';
  if (/디자인|시안|피드백/i.test(s)) return '🎨';
  return '';
}

// ─── Thread List Item ───
export function renderThreadItem(thread, isActive) {
  const colorClass = getAvatarColor(thread.sender_name);
  const initial = getInitial(thread.sender_name);
  const timeStr = formatTime(thread.last_received_at);
  const unread = thread.unread_count > 0;
  const pinned = thread.pinned;
  const important = thread.important;
  const categoryIcon = getCategoryIcon(thread);
  const dynamicEmoji = getDynamicEmoji(thread);

  const classes = [
    'thread-item',
    isActive ? 'active' : '',
    unread ? 'unread' : '',
    pinned ? 'pinned' : '',
    important ? 'important' : '',
  ].filter(Boolean).join(' ');

  // Store thread context data in a global map (avoids HTML encoding issues)
  if (!window._threadContextMap) window._threadContextMap = {};
  window._threadContextMap[thread.thread_id] = {
    thread_id: thread.thread_id,
    subject: thread.subject,
    pinned: thread.pinned,
    important: thread.important || false,
    unread_count: thread.unread_count,
    sender_name: thread.sender_name,
    is_briefing: thread.is_briefing || false,
  };

  return `
    <div class="${classes}" data-thread-id="${thread.thread_id}">
      <div class="thread-avatar ${colorClass}">
        <span class="avatar-letter">${initial}</span>
        ${dynamicEmoji ? `<span class="dynamic-emoji">${dynamicEmoji}</span>` : ''}
      </div>
      <div class="thread-info">
        <div class="thread-top-row">
          <span class="thread-sender">
            ${pinned ? '<span class="thread-pin-badge" title="핀 고정">📌</span>' : ''}${important ? '<span class="thread-important-badge" title="중요">⭐</span>' : ''}${categoryIcon ? `<span class="thread-category-icon">${categoryIcon}</span>` : ''}${escapeHtml(thread.sender_name)}
            ${thread.receiver_account ? `<span style="color:var(--text-muted); opacity: 0.7; font-size:11px; margin-left:6px; font-weight:normal;">(${escapeHtml(thread.receiver_account)})</span>` : ''}
          </span>
          <span class="thread-time">${timeStr}</span>
        </div>
        <div class="thread-subject">${escapeHtml(thread.subject)}</div>
        <div class="thread-preview">${escapeHtml(thread.last_message_preview || '')}</div>
        <div class="thread-badge-row">
          ${unread ? `<span class="unread-badge">${thread.unread_count}</span>` : ''}
          ${thread.status !== 'open' ? `<span class="thread-status-dot ${thread.status}"></span>` : ''}
        </div>
      </div>
    </div>
  `;
}

// ─── Chat Bubble ───
export function renderBubble(msg, index) {
  const isWhisper = msg.message_type === 'internal_comment';
  const isOutgoing = msg.is_outgoing;
  const hasOriginal = msg.body_original && msg.message_type === 'email';
  const colorClass = getAvatarColor(msg.sender_name || msg.sender_identity);
  const initial = getInitial(msg.sender_name || msg.sender_identity);

  const direction = isOutgoing ? 'outgoing' : 'incoming';
  const bubbleClass = isWhisper ? 'whisper' : direction;
  const hasOriginalClass = hasOriginal ? 'has-original' : '';
  const wrapperClass = isWhisper ? 'incoming' : direction;

  // Detect media content for Media Snapshot feature
  const mediaCard = detectMediaContent(msg.body_summary || '');

  // Detect tracking pixels for anti-tracking badge
  const hasTracker = msg.body_original && /1x1|pixel|track|beacon|\.gif\?/i.test(msg.body_original);

  // Encode msg data for context menu (right-click)
  const msgData = encodeURIComponent(JSON.stringify({
    id: msg.id,
    thread_id: msg.thread_id,
    body_summary: msg.body_summary || '',
    sender_name: msg.sender_name || '',
  }));

  return `
    <div class="message-wrapper ${wrapperClass}" style="animation-delay: ${Math.min(index * 0.03, 0.3)}s"
         data-msg-context='${msgData}'
         oncontextmenu="showBubbleContextMenu(event, JSON.parse(decodeURIComponent(this.dataset.msgContext)))">
      ${!isOutgoing ? `
        <div class="bubble-avatar ${colorClass}">
          ${initial}
        </div>
      ` : ''}
      <div class="bubble-content">
        ${!isOutgoing ? `<span class="bubble-sender-name">${escapeHtml(msg.sender_name || msg.sender_identity)}</span>` : ''}
        <div class="bubble ${bubbleClass} ${hasOriginalClass}"
             ${hasOriginal ? `data-msg-id="${msg.id}"` : ''}>
          ${msg.subject ? `<div class="bubble-subject">${escapeHtml(msg.subject)}</div>` : ''}
          ${escapeHtml(msg.body_summary || '')}
          ${hasTracker ? '<span class="anti-tracking-inline" title="읽음 픽셀 차단됨">🛡️</span>' : ''}
          ${msg.hashtags ? `
            <div class="bubble-hashtags">
              ${msg.hashtags.split(',').map(tag => `<span class="hashtag-item">${escapeHtml(tag.trim())}</span>`).join('')}
            </div>
          ` : ''}
          ${msg.attachments && Array.isArray(msg.attachments) && msg.attachments.length > 0 ? `
            <div class="bubble-attachments" style="display:flex; flex-direction:column; gap:4px; margin-top:8px;">
              ${msg.attachments.map(a => {
                const safePath = (a.local_path || '').replace(/\\/g, '\\\\').replace(/'/g, "\\'");
                const safeName = escapeHtml(a.filename || '첨부파일');
                return `
                  <div class="attachment-chip" 
                       draggable="true"
                       ondragstart="window.dragAttachment(event, '${safePath}')"
                       onclick="window.__TAURI__.core.invoke('open_attachment_cmd', { path: '${safePath}' })" 
                       style="cursor:pointer; padding:6px 10px; background:rgba(255,255,255,0.08); border-radius:6px; font-size:12px; display:flex; align-items:center; justify-content:space-between; transition:background 0.2s;">
                    <div style="display:flex; align-items:center; gap:6px; overflow:hidden; padding-right:8px;">
                      <span>📎</span>
                      <span style="white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title="${safeName}">${safeName}</span>
                      <small style="color:var(--text-muted); opacity: 0.8;">${Math.round((a.size_bytes || 0)/1024)}KB</small>
                    </div>
                    <div title="다운로드"
                         onclick="event.stopPropagation(); window.downloadAttachment('${safePath}', '${safeName.replace(/'/g, "\\'")}')" 
                         style="padding:4px; border-radius:4px; font-size:14px; opacity:0.7; transition:opacity 0.2s; display:flex; align-items:center;">
                      💾
                    </div>
                  </div>
                `;
              }).join('')}
            </div>
          ` : ''}
        </div>
        ${mediaCard}
        <div class="bubble-meta" style="justify-content: ${isOutgoing ? 'flex-end' : 'flex-start'};">
          ${msg.icon_type ? `<span class="bubble-icon-type">${getIconEmoji(msg.icon_type)}</span>` : ''}
          ${msg.emoji_tag ? `<span class="bubble-emoji-badge">${msg.emoji_tag}</span>` : ''}
          <span class="bubble-time">${formatTime(msg.created_at)}</span>
        </div>
      </div>
    </div>
  `;
}

// ─── Media Snapshot Detection ───
function detectMediaContent(text) {
  // YouTube
  const ytMatch = text.match(/(?:https?:\/\/)?(?:www\.)?(?:youtube\.com\/watch\?v=|youtu\.be\/)([\w-]+)/);
  if (ytMatch) {
    return `
      <div class="media-card" data-url="https://youtube.com/watch?v=${ytMatch[1]}">
        <div class="media-card-preview">
          <img src="https://img.youtube.com/vi/${ytMatch[1]}/mqdefault.jpg" alt="video" />
          <div class="media-card-play">▶</div>
        </div>
        <div class="media-card-info">
          <div class="media-card-title">YouTube 동영상</div>
          <div class="media-card-domain">youtube.com</div>
        </div>
      </div>
    `;
  }

  // Image links
  const imgMatch = text.match(/https?:\/\/\S+\.(?:png|jpg|jpeg|gif|webp)/i);
  if (imgMatch) {
    return `
      <div class="media-card" data-url="${escapeHtml(imgMatch[0])}">
        <div class="media-card-preview">
          <img src="${escapeHtml(imgMatch[0])}" alt="image" onerror="this.parentElement.innerHTML='🖼️ 이미지'" />
        </div>
        <div class="media-card-info">
          <div class="media-card-title">이미지 미리보기</div>
          <div class="media-card-domain">${new URL(imgMatch[0]).hostname}</div>
        </div>
      </div>
    `;
  }

  // Generic link
  const linkMatch = text.match(/https?:\/\/\S+/);
  if (linkMatch) {
    try {
      const url = new URL(linkMatch[0]);
      return `
        <div class="media-card" data-url="${escapeHtml(linkMatch[0])}">
          <div class="media-card-preview" style="height:60px; font-size:24px;">🔗</div>
          <div class="media-card-info">
            <div class="media-card-title">${escapeHtml(url.hostname)}</div>
            <div class="media-card-domain">${escapeHtml(url.pathname.substring(0,40))}</div>
          </div>
        </div>
      `;
    } catch(e) { /* ignore invalid URLs */ }
  }

  return '';
}

// ─── Date Separator ───
export function renderDateSeparator(dateStr) {
  return `
    <div class="date-separator">
      <span class="date-separator-text">${formatDateLabel(dateStr)}</span>
    </div>
  `;
}

// ─── Intelligence Panel ───
export function renderAISummary(messages) {
  if (!messages || messages.length === 0) {
    return '<p class="placeholder-text">메시지가 없습니다.</p>';
  }
  const emailMsgs = messages.filter(m => m.message_type === 'email' && m.body_summary);
  const lastThree = emailMsgs.slice(-3);
  if (lastThree.length === 0) return '<p class="placeholder-text">요약할 이메일이 없습니다.</p>';

  return lastThree.map(m => `
    <div class="summary-line">
      <span class="summary-bullet">›</span>
      <span>${escapeHtml(truncate(m.body_summary, 80))}</span>
    </div>
  `).join('');
}

export function renderHashtags(thread) {
  const tags = extractHashtags(thread);
  if (tags.length === 0) return '<span class="placeholder-text">#태그 없음</span>';
  return tags.map(tag => `<span class="hashtag">${tag}</span>`).join('');
}

export function renderActionCards(messages, thread) {
  const cards = [];

  // Detect dates
  const datePattern = /(\d{1,2})[\/월](\d{1,2})[일]?\s*(\d{1,2})?[시:]?/;
  for (const msg of messages) {
    if (!msg.body_summary) continue;
    if (datePattern.test(msg.body_summary)) {
      cards.push(`
        <div class="action-card">
          <div class="action-card-header">
            <span class="action-card-icon">📅</span>
            <span class="action-card-title">일정 감지</span>
          </div>
          <div class="action-card-detail">${escapeHtml(truncate(msg.body_summary, 60))}</div>
          <button class="action-card-btn">캘린더에 추가</button>
        </div>
      `);
      break;
    }
  }

  // Detect finance
  const moneyPattern = /[₩원\$][\d,]+|[\d,]+만원/;
  for (const msg of messages) {
    if (!msg.body_summary) continue;
    if (moneyPattern.test(msg.body_summary)) {
      cards.push(`
        <div class="action-card">
          <div class="action-card-header">
            <span class="action-card-icon">💰</span>
            <span class="action-card-title">금액 감지</span>
          </div>
          <div class="action-card-detail">${escapeHtml(truncate(msg.body_summary, 60))}</div>
        </div>
      `);
      break;
    }
  }

  // Telegram quick action card
  cards.push(`
    <div class="action-card" onclick="document.getElementById('telegram-modal').style.display='flex'">
      <div class="action-card-header">
        <span class="action-card-icon">✈️</span>
        <span class="action-card-title">Telegram 원격 제어</span>
      </div>
      <div class="action-card-detail">승인 · 빠른 답장 · 전달</div>
    </div>
  `);

  // Status card
  if (thread && thread.status !== 'open') {
    const statusLabel = thread.status === 'progress' ? '진행 중' : '완료';
    const statusIcon = thread.status === 'progress' ? '🔄' : '✅';
    cards.push(`
      <div class="action-card">
        <div class="action-card-header">
          <span class="action-card-icon">${statusIcon}</span>
          <span class="action-card-title">상태: ${statusLabel}</span>
        </div>
        ${thread.assigned_to ? `<div class="action-card-detail">담당: ${escapeHtml(thread.assigned_to)}</div>` : ''}
      </div>
    `);
  }

  return cards.join('');
}

export function renderParticipants(messages) {
  const seen = new Map();
  for (const msg of messages) {
    const key = msg.sender_identity;
    if (!seen.has(key)) {
      seen.set(key, { name: msg.sender_name || msg.sender_identity, email: msg.sender_identity });
    }
  }
  return Array.from(seen.values()).map(p => {
    const colorClass = getAvatarColor(p.name);
    return `
      <div class="participant-item">
        <div class="participant-avatar ${colorClass}">${getInitial(p.name)}</div>
        <div>
          <div class="participant-name">${escapeHtml(p.name)}</div>
          <div class="participant-email">${escapeHtml(p.email)}</div>
        </div>
      </div>
    `;
  }).join('');
}

// ─── Attachments List ───
export function renderAttachments(messages) {
  const attachments = [];
  for (const msg of messages) {
    if (!msg.body_summary) continue;
    // Detect mentioned files
    const filePatterns = msg.body_summary.match(/[\w가-힣]+\.(pdf|xlsx?|docx?|pptx?|zip|hwp|png|jpg)/gi);
    if (filePatterns) {
      for (const f of filePatterns) {
        if (!attachments.find(a => a.name === f)) {
          attachments.push({ name: f, icon: getFileIcon(f), size: '—' });
        }
      }
    }
  }
  if (attachments.length === 0) return '<p class="placeholder-text">첨부파일이 없습니다.</p>';
  return attachments.map(a => `
    <div class="attachment-item">
      <span class="attachment-icon">${a.icon}</span>
      <span class="attachment-name">${escapeHtml(a.name)}</span>
      <span class="attachment-size">${a.size}</span>
    </div>
  `).join('');
}

function getFileIcon(filename) {
  const ext = filename.split('.').pop().toLowerCase();
  const icons = { pdf:'📕', xlsx:'📊', xls:'📊', docx:'📘', doc:'📘', pptx:'📙', ppt:'📙', zip:'📦', hwp:'📝', png:'🖼️', jpg:'🖼️' };
  return icons[ext] || '📎';
}

// ─── Morning Briefing ───
export function renderBriefingCards(threads) {
  // Load dismissed list from localStorage
  const dismissed = JSON.parse(localStorage.getItem('briefing_dismissed') || '[]');

  // Show ALL briefing-flagged threads + ALL unread threads (no limit)
  let briefingThreads = threads.filter(t => t.is_briefing);
  const unread = threads.filter(t => !t.is_briefing && t.unread_count > 0);
  briefingThreads = briefingThreads.concat(unread);
  
  // If nothing, show recent 5 for demo
  if (briefingThreads.length === 0) {
    briefingThreads = threads.slice(0, 5);
  }

  // Filter out dismissed cards
  briefingThreads = briefingThreads.filter(t => !dismissed.includes(t.thread_id));

  if (briefingThreads.length === 0) return '';
  return briefingThreads.map(t => {
    const emoji = getDynamicEmoji(t) || '📧';
    return `
      <div class="briefing-card" data-thread-id="${t.thread_id}"
           oncontextmenu="showBriefingCardMenu(event, '${t.thread_id}', '${escapeHtml(t.subject).replace(/'/g, "\\'")}')">
        <div class="briefing-card-emoji">${emoji}</div>
        <div class="briefing-card-title">${escapeHtml(t.subject)}</div>
        <div class="briefing-card-desc">${escapeHtml(t.last_message_preview || '')}</div>
        <div class="briefing-card-sender">${escapeHtml(t.sender_name)}</div>
      </div>
    `;
  }).join('');
}

// ─── Wiki Generation ───
export function generateWikiContent(threads, allMessages) {
  const tagGroups = new Map();

  for (const thread of threads) {
    const tags = extractHashtags(thread);
    const threadMsgs = allMessages[thread.thread_id] || [];

    for (const tag of tags) {
      if (!tagGroups.has(tag)) tagGroups.set(tag, []);
      tagGroups.get(tag).push({
        subject: thread.subject,
        sender: thread.sender_name,
        date: thread.last_received_at,
        summary: threadMsgs.length > 0
          ? threadMsgs.filter(m => m.message_type === 'email').slice(-1)[0]?.body_summary || ''
          : thread.last_message_preview || '',
      });
    }
  }

  let html = '';
  for (const [tag, entries] of tagGroups) {
    html += `<div class="wiki-section"><h2>${tag}</h2>`;
    for (const e of entries) {
      html += `
        <div class="wiki-entry">
          <div class="wiki-entry-header">
            <strong>${escapeHtml(e.subject)}</strong>
            <span class="wiki-entry-date">${formatTime(e.date)} · ${escapeHtml(e.sender)}</span>
          </div>
          <div class="wiki-entry-content">${escapeHtml(truncate(e.summary, 120))}</div>
        </div>
      `;
    }
    html += '</div>';
  }
  return html || '<p class="placeholder-text">지식 베이스가 비어 있습니다.</p>';
}

// ─── Thread Jump (Past Context) ───
export function renderThreadJump(messages) {
  if (!messages || messages.length < 3) return '';
  const oldest = messages[0];
  return `
    <div class="thread-jump-summary">
      <strong>최초 메시지:</strong> ${escapeHtml(truncate(oldest.body_summary, 100))}
      <br/><small>${escapeHtml(oldest.sender_name || oldest.sender_identity)} · ${formatFullDate(oldest.created_at)}</small>
    </div>
  `;
}

// ─── Ghost Writing ───
export function generateGhostDraft(thread, messages) {
  if (!thread || !messages || messages.length === 0) return '';
  const lastMsg = messages[messages.length - 1];
  if (!lastMsg || lastMsg.is_outgoing) return '';

  const sender = lastMsg.sender_name || '상대방';
  const content = lastMsg.body_summary || '';

  if (/확인|승인|검토/.test(content)) return `${sender}님, 확인했습니다. 검토 후 알려드리겠습니다.`;
  if (/일정|미팅|회의/.test(content)) return `네, 해당 일정 확인했습니다. 참석하겠습니다.`;
  if (/감사|수고/.test(content)) return `감사합니다. 추가 사항이 있으면 말씀해 주세요.`;
  if (/요청|부탁|보내/.test(content)) return `네, 알겠습니다. 처리하여 보내드리겠습니다.`;
  if (/질문|문의|궁금/.test(content)) return `좋은 질문입니다. 확인 후 답변드리겠습니다.`;
  return `${sender}님, 메일 확인했습니다. 감사합니다.`;
}

// ─── Regex Cost Filter (classify locally) ───
export function classifyLocalRegex(subject) {
  if (/광고|AD|할인|쿠폰|프로모션|unsubscribe/i.test(subject)) return { category: 'ad', skip_ai: true };
  if (/영수증|결제 확인|주문 확인|receipt/i.test(subject)) return { category: 'receipt', skip_ai: true };
  if (/자동 발송|no-?reply|noreply/i.test(subject)) return { category: 'auto', skip_ai: true };
  return { category: 'normal', skip_ai: false };
}

// ─── Modal ───
export function renderEmailModal(msg) {
  // Use sender_name as title context, body_summary can contain CSS leak artifacts
  const title = (msg.sender_name || msg.sender_identity || '원본 이메일');
  return {
    subject: title,
    sender: msg.sender_name || msg.sender_identity,
    date: formatFullDate(msg.created_at),
    html: msg.body_original || '<p>원본 없음</p>',
  };
}

// ─── Helpers ───
function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function truncate(str, len) {
  if (!str) return '';
  return str.length > len ? str.substring(0, len) + '...' : str;
}

function getIconEmoji(iconType) {
  return { document:'📄', globe:'🌐', letter:'✉️' }[iconType] || '';
}

function extractHashtags(thread) {
  const tags = new Set();
  const s = thread.subject || '';
  if (/보고서|재정|인보이스|결제/.test(s)) tags.add('#보고서');
  if (/선교|뉴스레터|기도/.test(s)) tags.add('#선교사');
  if (/결제|재정|인보이스|금액/.test(s)) tags.add('#결제');
  if (/긴급|점검/.test(s)) tags.add('#긴급');
  if (/미팅|회의|킥오프/.test(s)) tags.add('#회의');
  if (/디자인|시안|피드백/.test(s)) tags.add('#디자인');
  if (/회식|투표/.test(s)) tags.add('#팀빌딩');
  if (/기도/.test(s)) tags.add('#기도');
  if (tags.size === 0) tags.add('#일반');
  return Array.from(tags);
}

// ─── Attachments Rendering (sidebar / detail panel) ───
window.renderAttachments = function(attachments) {
  if (!attachments || attachments.length === 0) return '';
  
  return `
    <div class="bubble-attachments" style="display:flex; flex-direction:column; gap:4px;">
      ${attachments.map(att => {
        const isImage = att.main_type === 'image' && att.thumbnail_path;
        const safePath = att.local_path ? att.local_path.replace(/\\/g, '\\\\').replace(/'/g, "\\'") : '';
        const safeName = escapeHtml(att.filename || '첨부파일');
        
        if (isImage) {
          const imgUrl = window.__TAURI__.core.convertFileSrc(att.thumbnail_path);
          return `
            <div class="attachment-item image-item" 
                 draggable="true"
                 ondragstart="window.dragAttachment(event, '${safePath}')"
                 onclick="window.openAttachment('${safePath}')" title="${safeName}">
              <img src="${imgUrl}" alt="${safeName}" loading="lazy" />
            </div>
          `;
        } else {
          const sizeKb = Math.round(att.size_bytes / 1024);
          return `
            <div class="attachment-item file-item" style="display:flex; align-items:center; justify-content:space-between;"
                 draggable="true"
                 ondragstart="window.dragAttachment(event, '${safePath}')"
                 onclick="window.openAttachment('${safePath}')" title="${safeName}">
              <div style="display:flex; align-items:center; gap:6px;">
                <span class="attachment-icon">📎</span>
                <div class="attachment-info">
                  <span class="attachment-name">${safeName}</span>
                  <span class="attachment-size">${sizeKb} KB</span>
                </div>
              </div>
              <div title="다운로드" onclick="event.stopPropagation(); window.downloadAttachment('${safePath}', '${safeName}')" 
                   style="padding:4px; font-size:14px; opacity:0.7; cursor:pointer;">💾</div>
            </div>
          `;
        }
      }).join('')}
    </div>
  `;
};

window.openAttachment = async function(path) {
  if (!path) return;
  try {
    await window.__TAURI__.core.invoke('open_attachment_cmd', { path });
  } catch (e) {
    console.error('파일을 열 수 없습니다:', e);
  }
};
