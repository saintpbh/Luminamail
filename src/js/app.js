// ═══════════════════════════════════════════════════════════
// Lumina Mail - Main Application (v2: Full Features)
// State, Events, Tauri IPC, and All Feature Logic
// ═══════════════════════════════════════════════════════════

import {
  renderThreadItem, renderBubble, renderDateSeparator,
  renderAISummary, renderHashtags, renderActionCards,
  renderParticipants, renderEmailModal, renderAttachments,
  renderBriefingCards, generateWikiContent, renderThreadJump,
  generateGhostDraft, classifyLocalRegex, getAvatarColor,
} from './components.js';

// ─── State ───
const state = {
  threads: [],
  currentThreadId: null,
  currentMessages: [],
  currentThread: null,
  filter: 'all',
  allMessages: {},       // cache: threadId -> messages
  whisperNotes: {},      // threadId -> note text
  undoSendTimer: null,
  undoSendCountdown: 5,
  ghostDraft: '',
  telegramConnected: true,
  telegramLog: [],
  briefingDismissed: false,
};

// ─── Undo System (Phase 11) ───
window.undoStack = [];
window.pushUndo = (action) => {
  window.undoStack.push(action);
  if (window.undoStack.length > 10) window.undoStack.shift();
};

window.dragAttachment = (e, localPath) => {
  e.preventDefault();
  window.__TAURI__.core.invoke('drag_attachment_cmd', { path: localPath }).catch(console.error);
};

window.downloadAttachment = async (localPath, filename) => {
  try {
    // Use Tauri dialog plugin to show native Save As dialog
    const { save } = await import('@tauri-apps/plugin-dialog');
    const destPath = await save({
      defaultPath: filename,
      title: "첨부파일 저장",
    });
    if (destPath) {
      await window.__TAURI__.core.invoke('save_attachment_cmd', {
        source: localPath,
        destination: destPath
      });
    }
  } catch (err) {
    console.error("Download failed:", err);
  }
};

document.addEventListener('keydown', async (e) => {
  // Skip during IME composition (Korean, Japanese, Chinese input)
  if (e.isComposing || e.keyCode === 229) return;

  // Catch Cmd+Z (Mac) or Ctrl+Z (Windows)
  if (e.key === 'z' && (e.metaKey || e.ctrlKey) && !e.shiftKey) {
    if (window.undoStack.length > 0) {
      e.preventDefault();
      const action = window.undoStack.pop();
      try {
        if (['delete', 'spam', 'snooze'].includes(action.type)) {
          await invoke('restore_thread_cmd', { threadId: action.threadId });
        } else if (action.type === 'pin') {
          await invoke('toggle_thread_pin', { threadId: action.threadId });
        } else if (action.type === 'important') {
          await invoke('toggle_thread_important', { threadId: action.threadId });
        } else if (action.type === 'unread') {
          await invoke('toggle_thread_unread', { threadId: action.threadId });
        } else if (action.type === 'briefing') {
          await invoke('toggle_briefing', { threadId: action.threadId, is_briefing: !action.is_briefing });
        }
        await loadThreads();
        console.log('Undo executed for:', action.type);
      } catch (err) {
        console.error('Undo failed:', err);
      }
    }
  }

  // Phase 11: Bind Delete/Backspace to move active thread to Trash
  const ae = document.activeElement;
  const isInput = ae && (ae.tagName === 'INPUT' || ae.tagName === 'TEXTAREA' || ae.isContentEditable);
  
  if (!isInput && (e.key === 'Backspace' || e.key === 'Delete')) {
    if (state.currentThreadId) {
      const thread = state.threads.find(t => t.thread_id === state.currentThreadId);
      if (thread) {
        e.preventDefault();
        window.pushUndo({ type: 'delete', threadId: thread.thread_id });
        invoke('delete_thread_cmd', { threadId: thread.thread_id }).then(async () => {
          document.getElementById('main-content').innerHTML = `
            <div class="empty-state">
              <div class="empty-icon">✉️</div>
              <h2>대화를 선택하세요</h2>
            </div>
          `;
          state.currentThreadId = null;
          state.currentThread = null;
          state.currentMessages = [];
          await loadThreads();
        });
      }
    }
  }

  // Navigation: ↑/K = previous thread, ↓/J = next thread, Enter = open
  if (!isInput) {
    if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      navigateThread(-1);
    } else if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      navigateThread(1);
    } else if (e.key === 'Enter' && state.currentThreadId) {
      e.preventDefault();
      selectThread(state.currentThreadId);
    }
  }

  // ⌘K = focus search
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault();
    const searchInput = document.querySelector('.search-input');
    if (searchInput) searchInput.focus();
  }

  // ⌘R = reload app
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') {
    e.preventDefault();
    window.location.reload();
  }

  // Cmd+, → Settings
  if ((e.metaKey || e.ctrlKey) && e.key === ',') {
    e.preventDefault();
    if (typeof openSettings === 'function') {
      openSettings();
    }
  }
  // Cmd+Option+I → DevTools
  if ((e.metaKey || e.ctrlKey) && e.altKey && e.key === 'i') {
    e.preventDefault();
    if (window.__TAURI__ && window.__TAURI__.core) {
      window.__TAURI__.core.invoke('tauri', { __tauriModule: 'Window', message: { cmd: 'manage', data: { cmd: { type: '__toggleDevtools' }}}}).catch(() => {});
    }
  }

  // ⌘/ = shortcut guide
  if ((e.metaKey || e.ctrlKey) && e.key === '/') {
    e.preventDefault();
    toggleShortcutGuide();
  }

  // ⌘? = help guide
  if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === '?') {
    e.preventDefault();
    openHelp();
  }

  // Escape to close help
  if (e.key === 'Escape') {
    const helpModal = document.getElementById('help-modal');
    if (helpModal && helpModal.style.display !== 'none') {
      closeHelp();
    }
  }
});

// ─── Help Guide ───
function openHelp() {
  const m = document.getElementById('help-modal');
  if (m) m.style.display = '';
}
function closeHelp() {
  const m = document.getElementById('help-modal');
  if (m) m.style.display = 'none';
}
window.openHelp = openHelp;
window.closeHelp = closeHelp;

// Help modal events
document.addEventListener('DOMContentLoaded', () => {
  const helpModal = document.getElementById('help-modal');
  if (!helpModal) return;

  // Close button
  document.getElementById('help-close-btn')?.addEventListener('click', closeHelp);

  // Close on overlay click
  helpModal.addEventListener('click', (e) => {
    if (e.target === helpModal) closeHelp();
  });

  // Section navigation
  helpModal.querySelectorAll('.help-nav-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const section = btn.dataset.section;
      // Toggle nav buttons
      helpModal.querySelectorAll('.help-nav-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      // Toggle sections
      helpModal.querySelectorAll('.help-section').forEach(s => s.classList.remove('active'));
      const target = helpModal.querySelector(`.help-section[data-section="${section}"]`);
      if (target) target.classList.add('active');
    });
  });
});

// ── Thread Navigation Helper ──
function navigateThread(direction) {
  const threads = state.threads;
  if (!threads.length) return;
  const currentIdx = threads.findIndex(t => t.thread_id === state.currentThreadId);
  let nextIdx;
  if (currentIdx === -1) {
    nextIdx = direction > 0 ? 0 : threads.length - 1;
  } else {
    nextIdx = currentIdx + direction;
    if (nextIdx < 0) nextIdx = 0;
    if (nextIdx >= threads.length) nextIdx = threads.length - 1;
  }
  selectThread(threads[nextIdx].thread_id);
  // Scroll selected thread into view
  const item = document.querySelector(`.thread-item[data-thread-id="${threads[nextIdx].thread_id}"]`);
  if (item) item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
}

// ── Shortcut Guide Modal ──
function toggleShortcutGuide() {
  let modal = document.getElementById('shortcut-guide-modal');
  if (modal) {
    modal.remove();
    return;
  }
  modal = document.createElement('div');
  modal.id = 'shortcut-guide-modal';
  modal.className = 'modal-overlay';
  modal.onclick = (e) => { if (e.target === modal) modal.remove(); };
  const isMac = navigator.platform.includes('Mac');
  const mod = isMac ? '⌘' : 'Ctrl+';
  modal.innerHTML = `
    <div class="shortcut-guide-container">
      <div class="shortcut-guide-header">
        <h2>⌨️ 단축키 안내</h2>
        <button class="modal-close-btn" onclick="this.closest('.modal-overlay').remove()">✕</button>
      </div>
      <div class="shortcut-guide-body">
        <div class="shortcut-section">
          <h3>📧 메일</h3>
          <div class="shortcut-row"><span>새 메일 작성</span><kbd>${mod}N</kbd></div>
          <div class="shortcut-row"><span>답장</span><kbd>${mod}R</kbd></div>
          <div class="shortcut-row"><span>전달</span><kbd>${mod}⇧F</kbd></div>
          <div class="shortcut-row"><span>전체 동기화</span><kbd>${mod}⇧S</kbd></div>
          <div class="shortcut-row"><span>삭제</span><kbd>Delete</kbd></div>
          <div class="shortcut-row"><span>보관</span><kbd>${mod}E</kbd></div>
        </div>
        <div class="shortcut-section">
          <h3>📌 정리</h3>
          <div class="shortcut-row"><span>읽음/안읽음</span><kbd>${mod}⇧U</kbd></div>
          <div class="shortcut-row"><span>중요 표시</span><kbd>${mod}⇧I</kbd></div>
          <div class="shortcut-row"><span>핀 고정/해제</span><kbd>${mod}⇧P</kbd></div>
          <div class="shortcut-row"><span>실행취소</span><kbd>${mod}Z</kbd></div>
        </div>
        <div class="shortcut-section">
          <h3>🔍 탐색</h3>
          <div class="shortcut-row"><span>이전 메일</span><kbd>↑</kbd> / <kbd>K</kbd></div>
          <div class="shortcut-row"><span>다음 메일</span><kbd>↓</kbd> / <kbd>J</kbd></div>
          <div class="shortcut-row"><span>메일 열기</span><kbd>Enter</kbd></div>
          <div class="shortcut-row"><span>검색</span><kbd>${mod}K</kbd></div>
        </div>
        <div class="shortcut-section">
          <h3>👁 보기</h3>
          <div class="shortcut-row"><span>전체 메일</span><kbd>${mod}1</kbd></div>
          <div class="shortcut-row"><span>안 읽음</span><kbd>${mod}2</kbd></div>
          <div class="shortcut-row"><span>고정됨</span><kbd>${mod}3</kbd></div>
          <div class="shortcut-row"><span>휴지통</span><kbd>${mod}4</kbd></div>
          <div class="shortcut-row"><span>스팸</span><kbd>${mod}5</kbd></div>
          <div class="shortcut-row"><span>주소록</span><kbd>${mod}6</kbd></div>
        </div>
        <div class="shortcut-section">
          <h3>⚙️ 앱</h3>
          <div class="shortcut-row"><span>설정</span><kbd>${mod},</kbd></div>
          <div class="shortcut-row"><span>단축키 안내</span><kbd>${mod}/</kbd></div>
        </div>
      </div>
    </div>
  `;
  document.body.appendChild(modal);
}
window.toggleShortcutGuide = toggleShortcutGuide;

// ─── Tauri IPC ───
async function invoke(cmd, args = {}) {
  if (window.__TAURI__ && window.__TAURI__.core) {
    return window.__TAURI__.core.invoke(cmd, args);
  }
  console.warn('Tauri not available');
  return null;
}

// ─── Init ───
document.addEventListener('DOMContentLoaded', async () => {
  state.contacts = await invoke('get_contacts') || [];
  await loadThreads();
  setupEventListeners();
  showMorningBriefing();
  initTelegramStatus();
  loadWhisperNotes();

  // Dismiss splash screen with fade-out
  const splash = document.getElementById('splash-screen');
  if (splash) {
    splash.classList.add('fade-out');
    setTimeout(() => splash.remove(), 600);
  }

  // Listen for sync events from settings
  window.addEventListener('threads-updated', async () => {
    await loadThreads();
  });

  if (window.__TAURI__ && window.__TAURI__.event) {
    window.__TAURI__.event.listen('sync-progress', (event) => {
      const payload = event.payload;
      const progressDiv = document.getElementById('global-sync-progress');
      const textSpan = document.getElementById('sync-progress-text');
      const bar = document.getElementById('sync-progress-bar');

      if (progressDiv && textSpan && bar) {
        progressDiv.style.display = 'block';
        textSpan.textContent = `${payload.current} / ${payload.total}`;
        const pct = Math.min(100, Math.round((payload.current / payload.total) * 100));
        bar.style.width = `${pct}%`;

        if (payload.current >= payload.total) {
          setTimeout(() => { progressDiv.style.display = 'none'; }, 3000);
        }
      }
    });
  }

  // Automatically sync email accounts in the background on startup
  triggerBackgroundSync();

  // Enable Real-Time Background Sync every 5 minutes (incremental UID-based)
  setInterval(() => {
    console.log('Running scheduled incremental sync...');
    triggerBackgroundSync();
  }, 5 * 60 * 1000);
});

async function triggerBackgroundSync() {
  try {
    // 1. Silent trash purge
    invoke('auto_empty_trash_cmd').then(count => {
      if (count > 0) console.log(`Auto-purged ${count} old emails from trash.`);
    }).catch(e => console.error('Trash purge error:', e));

    const accounts = await invoke('get_email_accounts');
    if (accounts && accounts.length > 0) {
      console.log('Starting background sync for accounts...');
      let anySynced = false;
      for (const acc of accounts) {
        if (acc.enabled !== false) { // Default enabled
          try {
            await invoke('sync_email_account', { accountId: acc.id });
            anySynced = true;
          } catch(err) {
            console.warn(`Sync failed for ${acc.email}:`, err);
          }
        }
      }
      if (anySynced) {
        await loadThreads();
      }
    }
  } catch (e) {
    console.warn('Background sync failed:', e);
  }
}

// ── Pull-to-Refresh Gesture Handler ──
(function initPullToRefresh() {
  const threadList = document.getElementById('thread-list');
  const ptr = document.getElementById('pull-to-refresh');
  if (!threadList || !ptr) return;

  let startY = 0;
  let pulling = false;
  let isSyncing = false;

  threadList.addEventListener('touchstart', (e) => {
    if (threadList.scrollTop <= 0 && !isSyncing) {
      startY = e.touches[0].clientY;
      pulling = true;
    }
  }, { passive: true });

  // Scroll-wheel overscroll (desktop)
  let overscrollAccum = 0;
  threadList.addEventListener('wheel', (e) => {
    if (threadList.scrollTop <= 0 && e.deltaY < 0 && !isSyncing) {
      overscrollAccum += Math.abs(e.deltaY);
      if (overscrollAccum > 400) {
        triggerPullRefresh();
        overscrollAccum = 0;
      }
    } else {
      overscrollAccum = 0;
    }
  }, { passive: true });

  const onMove = (clientY) => {
    if (!pulling || isSyncing) return;
    const diff = clientY - startY;
    if (diff > 140) {
      ptr.classList.add('active');
    } else {
      ptr.classList.remove('active');
    }
  };

  threadList.addEventListener('touchmove', (e) => onMove(e.touches[0].clientY), { passive: true });

  const onEnd = (clientY) => {
    if (!pulling || isSyncing) return;
    pulling = false;
    const diff = clientY - startY;
    if (diff > 140) {
      triggerPullRefresh();
    } else {
      ptr.classList.remove('active');
    }
  };

  threadList.addEventListener('touchend', (e) => onEnd(e.changedTouches[0].clientY));

  async function triggerPullRefresh() {
    if (isSyncing) return;
    isSyncing = true;
    ptr.classList.remove('active');
    ptr.classList.add('syncing');
    ptr.querySelector('.ptr-text').textContent = '새 메일 확인 중...';

    // Show laser progress bar
    const laser = document.getElementById('laser-progress');
    if (laser) laser.classList.add('active');

    try {
      await triggerBackgroundSync();
      ptr.querySelector('.ptr-text').textContent = '✓ 완료!';
    } catch (e) {
      ptr.querySelector('.ptr-text').textContent = '⚠ 동기화 실패';
      console.error('Pull-to-refresh sync failed:', e);
    }

    if (laser) laser.classList.remove('active');

    setTimeout(() => {
      ptr.classList.remove('syncing');
      ptr.querySelector('.ptr-text').textContent = '새 메일 확인 중...';
      isSyncing = false;
    }, 1500);
  }

  // Expose for external use
  window.manualSync = triggerPullRefresh;
})();

// ─── Load Threads ───
async function loadThreads() {
  try {
    state.threads = await invoke('get_threads', { filter: state.filter || 'all' }) || [];
    // Debug: check if ai_tags is populated
    const tagged = state.threads.filter(t => t.ai_tags);
    if (tagged.length > 0) {
      console.log('[loadThreads] Threads with ai_tags:', tagged.map(t => ({ id: t.thread_id, ai_tags: t.ai_tags })));
    } else {
      console.log('[loadThreads] No threads have ai_tags set. Sample thread keys:', state.threads[0] ? Object.keys(state.threads[0]) : 'none');
    }
    renderThreadList();
    
    // Background pre-fetch all thread messages for instant access
    prefetchAllMessages();
  } catch (e) {
    console.error('Failed to load threads:', e);
  }
}

// ── Background Message Pre-fetcher ──
async function prefetchAllMessages() {
  for (const thread of state.threads) {
    if (state.allMessages[thread.thread_id]) continue; // Already cached
    try {
      const msgs = await invoke('get_messages', { threadId: thread.thread_id });
      if (msgs) state.allMessages[thread.thread_id] = msgs;
    } catch (e) { /* silent */ }
  }
}

// ─── Render Thread List ───
function renderThreadList() {
  const container = document.getElementById('thread-list');
  let filtered = state.threads;

  if (state.filter === 'unread') {
    filtered = filtered.filter(t => t.unread_count > 0);
  } else if (state.filter === 'pinned') {
    filtered = filtered.filter(t => t.pinned);
  } else if (state.filter === 'media') {
    // Filter threads that likely have media content
    filtered = filtered.filter(t => {
      const preview = (t.last_message_preview || '') + (t.subject || '');
      return /https?:\/\/|\.pdf|\.png|\.jpg|동영상|이미지|파일/i.test(preview);
    });
  } else if (state.filter === 'trash' || state.filter === 'spam') {
    // Backend already returned explicitly requested trash/spam
  }

  const pinned = filtered.filter(t => t.pinned);
  const unpinned = filtered.filter(t => !t.pinned);

  let html = '';
  if (pinned.length > 0 && state.filter === 'all') {
    html += '<div class="thread-section-label">📌 고정됨</div>';
    html += pinned.map(t => renderThreadItem(t, t.thread_id === state.currentThreadId)).join('');
    html += '<div class="thread-section-label">모든 메일</div>';
  }
  html += unpinned.map(t => renderThreadItem(t, t.thread_id === state.currentThreadId)).join('');

  if (filtered.length === 0) {
    html = `<div style="padding:40px 20px;text-align:center;color:var(--text-tertiary);">
      <div style="font-size:32px;margin-bottom:8px;">📭</div>
      <p style="font-size:12px;">표시할 메일이 없습니다.</p>
    </div>`;
  }
  container.innerHTML = html;
}

// ─── Select Thread ───
async function selectThread(threadId) {
  if (state.currentThreadId === threadId) return;
  state.currentThreadId = threadId;
  const thread = state.threads.find(t => t.thread_id === threadId);
  state.currentThread = thread;

  let unreadChanged = false;
  if (thread && thread.unread_count > 0) {
    thread.unread_count = 0;
    unreadChanged = true;
  }

  // 1. Instant Visual Feedback (Sidebar list item)
  document.querySelectorAll('.thread-item').forEach(el => {
    if (el.dataset.threadId === threadId) {
      el.classList.add('active');
      if (unreadChanged) {
        el.classList.remove('unread');
        const badge = el.querySelector('.unread-badge');
        if (badge) badge.remove();
      }
    } else {
      el.classList.remove('active');
    }
  });

  // 2. Fire-and-forget background tasks (Mark as read)
  invoke('mark_thread_read', { threadId }).catch(e => console.error('mark read:', e));

  // 3. Fetch messages instantly (local SQLite is sub-millisecond)
  if (state.allMessages[threadId]) {
    state.currentMessages = state.allMessages[threadId];
    renderChatView();
  } else {
    const laserProgress = document.getElementById('laser-progress');
    if (laserProgress) laserProgress.style.display = 'block';
    
    try {
      const msgs = await invoke('get_messages', { threadId }) || [];
      if (laserProgress) laserProgress.style.display = 'none';
      
      if (state.currentThreadId === threadId) {
        state.currentMessages = msgs;
        state.allMessages[threadId] = msgs;
        renderChatView();
      }
    } catch (e) { 
      console.error('load messages:', e); 
      if (laserProgress) laserProgress.style.display = 'none';
    }
  }
}

// ─── Render Chat View ───
function renderChatView() {
  const thread = state.currentThread;
  const messages = state.currentMessages;

  // Header
  document.getElementById('chat-subject').textContent = thread ? thread.subject : '대화를 선택하세요';

  const badgeEl = document.getElementById('chat-status-badge');
  if (thread && thread.status) {
    badgeEl.textContent = getStatusLabel(thread.status);
    badgeEl.className = `status-badge ${thread.status}`;
    badgeEl.style.display = 'inline-block';
  } else {
    badgeEl.style.display = 'none';
  }

  // Anti-tracking badge (show if any message has tracking pixels)
  const antiTrack = document.getElementById('anti-tracking-badge');
  const hasTrackers = messages.some(m => m.body_original && /1x1|pixel|track|beacon|\.gif\?/i.test(m.body_original));
  antiTrack.style.display = hasTrackers ? 'inline-flex' : 'none';

  // Thread Jump (show past context for long threads)
  const jumpEl = document.getElementById('thread-jump');
  const jumpContent = document.getElementById('thread-jump-content');
  if (messages.length >= 3) {
    jumpContent.innerHTML = renderThreadJump(messages);
    jumpEl.style.display = 'block';
  } else {
    jumpEl.style.display = 'none';
  }

  // Messages
  const container = document.getElementById('chat-messages');
  if (!messages || messages.length === 0) {
    container.innerHTML = `<div class="empty-state">
      <div class="empty-state-icon">💬</div>
      <h3>메시지가 없습니다</h3>
    </div>`;
    return;
  }

  let html = '';
  let lastDate = '';
  messages.forEach((msg, i) => {
    // Ensure every message displays a subject. Fallback to thread subject.
    if (!msg.subject && thread) {
      msg.subject = thread.subject;
    }
    const msgDate = msg.created_at ? msg.created_at.split('T')[0] : '';
    if (msgDate !== lastDate) {
      html += renderDateSeparator(msg.created_at);
      lastDate = msgDate;
    }
    html += renderBubble(msg, i);
  });
  container.innerHTML = html;

  requestAnimationFrame(() => {
    container.scrollTop = container.scrollHeight;
  });

  document.getElementById('chat-input-area').style.display = 'block';
  renderIntelPanel();
  showGhostDraft();
  loadWhisperForThread();
}

// ─── Render Intelligence Panel ───
async function renderIntelPanel() {
  const messages = state.currentMessages;
  const thread = state.currentThread;

  // Basic rendering first (instant)
  document.getElementById('ai-summary').innerHTML = renderAISummary(messages);
  document.getElementById('hashtags').innerHTML = thread ? renderHashtags(thread) : '';
  document.getElementById('action-cards').innerHTML = renderActionCards(messages, thread);
  document.getElementById('participants').innerHTML = renderParticipants(messages);
  document.getElementById('attachments-list').innerHTML = renderAttachments(messages);

  // Cost optimization
  if (thread) {
    const classification = classifyLocalRegex(thread.subject);
    const costEl = document.getElementById('cost-saving');
    if (classification.skip_ai) {
      costEl.textContent = `✅ Regex 분류됨 (${classification.category})`;
    } else {
      costEl.textContent = 'API 비용 80% ↓';
    }
  }

  // ─── AI Processing (on-demand) ───
  if (!thread) return;
  try {
    const summaryEl = document.getElementById('ai-summary');
    summaryEl.classList.add('loading');
    
    const result = await invoke('process_email_ai_cmd', { threadId: thread.thread_id });
    
    // Remove loading indicator
    summaryEl.classList.remove('loading');
    
    if (result.status === 'processed' || result.status === 'already_processed') {
      // Show AI Summary
      if (result.summary) {
        summaryEl.innerHTML = `
          <div class="ai-summary-badge" style="display:flex;align-items:center;gap:6px;margin-bottom:8px;">
            <span style="background:linear-gradient(135deg,#667eea,#764ba2);color:white;font-size:10px;padding:2px 8px;border-radius:10px;font-weight:600;">AI 요약</span>
          </div>
          <div style="color:var(--text-primary);font-size:14px;line-height:1.6;">${escapeHtml(result.summary)}</div>
          ${result.translation ? `
            <div id="ai-translation-section" style="margin-top:10px;display:none;">
              <span style="background:linear-gradient(135deg,#11998e,#38ef7d);color:white;font-size:10px;padding:2px 8px;border-radius:10px;font-weight:600;">번역</span>
              <div style="color:var(--text-secondary);font-size:13px;line-height:1.5;margin-top:6px;">${escapeHtml(result.translation)}</div>
            </div>
            <button onclick="toggleTranslation()" style="margin-top:8px;background:none;border:1px solid var(--border-subtle);color:var(--text-secondary);padding:4px 12px;border-radius:6px;font-size:12px;cursor:pointer;">🌐 번역 보기</button>
          ` : ''}
        `;
      }
      
      // Show AI Tags (merge with existing manual tags)
      if (result.tags && result.tags.length > 0) {
        const existingTags = new Set();
        // Keep manual tags from thread.ai_tags
        if (thread.ai_tags) {
          thread.ai_tags.split(',').map(t => t.trim()).filter(Boolean).forEach(t => {
            existingTags.add(t.startsWith('#') ? t : `#${t}`);
          });
        }
        // Add AI tags
        result.tags.forEach(t => existingTags.add(t));
        document.getElementById('hashtags').innerHTML = Array.from(existingTags).map(tag =>
          `<span class="hashtag">${tag}</span>`
        ).join('');
      }
    }
  } catch (err) {
    // If API key not set, silently skip — AI panel stays with basic content
    const loadingEl = document.querySelector('.ai-loading');
    if (loadingEl) loadingEl.remove();
    if (err && !err.includes('API')) console.error('AI processing error:', err);
  }
}

window.toggleTranslation = function() {
  const section = document.getElementById('ai-translation-section');
  if (section) {
    section.style.display = section.style.display === 'none' ? 'block' : 'none';
  }
};

// Escape HTML helper (ensure it exists)
function escapeHtml(text) {
  if (!text) return '';
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ─── Ghost Writing (AI Draft) ───
function showGhostDraft() {
  const ghostEl = document.getElementById('ghost-text');
  const inputEl = document.getElementById('reply-input');

  if (inputEl.innerText.trim().length > 0) {
    ghostEl.style.display = 'none';
    return;
  }

  state.ghostDraft = generateGhostDraft(state.currentThread, state.currentMessages);
  if (state.ghostDraft) {
    ghostEl.textContent = state.ghostDraft;
    ghostEl.style.display = 'block';
    ghostEl.classList.add('visible');
  } else {
    ghostEl.style.display = 'none';
  }
}

function acceptGhostDraft() {
  const inputEl = document.getElementById('reply-input');
  const ghostEl = document.getElementById('ghost-text');
  if (state.ghostDraft && inputEl.innerText.trim().length === 0) {
    inputEl.innerText = state.ghostDraft;
    ghostEl.style.display = 'none';
    inputEl.style.height = 'auto';
    inputEl.style.height = Math.min(inputEl.scrollHeight, 120) + 'px';
  }
}

// ─── Undo Send ───
function startUndoSend(text, html, isWhisper) {
  const toast = document.getElementById('undo-send-toast');
  const countdownEl = document.getElementById('undo-countdown');
  const progressEl = document.getElementById('undo-progress');

  state.undoSendCountdown = 5;
  toast.style.display = 'flex';
  countdownEl.textContent = '5';
  progressEl.style.width = '0%';

  // Animate progress
  requestAnimationFrame(() => {
    progressEl.style.width = '100%';
    progressEl.style.transition = 'width 5s linear';
  });

  state.undoSendTimer = setInterval(() => {
    state.undoSendCountdown--;
    countdownEl.textContent = state.undoSendCountdown;

    if (state.undoSendCountdown <= 0) {
      clearInterval(state.undoSendTimer);
      toast.style.display = 'none';
      progressEl.style.width = '0%';
      progressEl.style.transition = 'none';
      // Actually send
      actuallySendMessage(text, html, isWhisper);
    }
  }, 1000);
}

function cancelUndoSend() {
  clearInterval(state.undoSendTimer);
  const toast = document.getElementById('undo-send-toast');
  const progressEl = document.getElementById('undo-progress');
  toast.style.display = 'none';
  progressEl.style.width = '0%';
  progressEl.style.transition = 'none';

  // Log to telegram
  addTelegramLog('⛔ 전송 취소됨');
}

async function actuallySendMessage(text, html, isWhisper) {
  if (!state.currentThreadId || !html.trim()) return;
  try {
    const msgType = isWhisper ? 'whisper' : 'reply';
    await invoke('send_reply', {
      threadId: state.currentThreadId,
      bodyText: text,
      bodyHtml: html,
      messageType: msgType,
    });

    // Simulate telegram notification for sent message
    if (!isWhisper) {
      addTelegramLog(`✅ 발송 완료: "${text.substring(0, 30)}..."`);
      simulateTelegramNotification(`발송 완료: ${state.currentThread?.subject || ''}`);
    }

    // Reload messages
    state.currentMessages = await invoke('get_messages', { threadId: state.currentThreadId }) || [];
    renderChatView();
  } catch (e) {
    console.error('send failed:', e);
  }
}

// ─── File Upload with Cloud Hybrid ───
const CLOUD_THRESHOLD_MB = 10;

function simulateUpload(filename, sizeMB) {
  const uploadEl = document.getElementById('upload-progress');
  const filenameEl = document.getElementById('upload-filename');
  const sizeEl = document.getElementById('upload-size');
  const fillEl = document.getElementById('upload-bar-fill');
  const cloudBadge = document.getElementById('upload-cloud-badge');

  filenameEl.textContent = filename;
  uploadEl.style.display = 'block';

  const isCloud = sizeMB > CLOUD_THRESHOLD_MB;
  cloudBadge.style.display = isCloud ? 'inline' : 'none';
  cloudBadge.textContent = isCloud ? '☁️ 클라우드 업로드' : '';

  let progress = 0;
  const interval = setInterval(() => {
    progress += Math.random() * 15 + 5;
    if (progress >= 100) progress = 100;

    fillEl.style.width = progress + '%';
    sizeEl.textContent = `${(sizeMB * progress / 100).toFixed(1)}MB / ${sizeMB}MB`;

    if (progress >= 100) {
      clearInterval(interval);
      setTimeout(() => {
        uploadEl.style.display = 'none';
        fillEl.style.width = '0%';
        addTelegramLog(`📎 파일 업로드 완료: ${filename}`);
        simulateTelegramNotification(`파일 전송 완료: ${filename}`);
      }, 800);
    }
  }, 300);
}

async function handleCloudUpload(filePath, fileName, fileSize) {
  const sizeMB = fileSize / (1024 * 1024);
  const uploadEl = document.getElementById('upload-progress');
  const filenameEl = document.getElementById('upload-filename');
  const sizeEl = document.getElementById('upload-size');
  const fillEl = document.getElementById('upload-bar-fill');
  const cloudBadge = document.getElementById('upload-cloud-badge');

  filenameEl.textContent = fileName;
  uploadEl.style.display = 'block';

  if (sizeMB <= CLOUD_THRESHOLD_MB) {
    // Small file: standard attach (simulated progress)
    cloudBadge.style.display = 'none';
    simulateUpload(fileName, sizeMB);
    return;
  }

  // Large file: attempt cloud upload
  cloudBadge.style.display = 'inline';
  sizeEl.textContent = `${sizeMB.toFixed(1)}MB — 클라우드 업로드 중...`;
  fillEl.style.width = '30%';

  // Try to find a connected cloud provider
  let provider = null;
  try {
    const providers = await invoke('cloud_get_status');
    if (providers.includes('gdrive')) provider = 'gdrive';
    else if (providers.includes('onedrive')) provider = 'onedrive';
  } catch(e) {}

  if (!provider) {
    cloudBadge.textContent = '☁️ 클라우드 미연결';
    sizeEl.textContent = `${sizeMB.toFixed(1)}MB — ⚠️ 설정에서 클라우드를 연결하세요`;
    fillEl.style.width = '100%';
    fillEl.style.background = '#ff9f0a';
    setTimeout(() => {
      uploadEl.style.display = 'none';
      fillEl.style.width = '0%';
      fillEl.style.background = '';
    }, 3000);
    return;
  }

  const providerName = provider === 'gdrive' ? 'Google Drive' : 'OneDrive';
  cloudBadge.textContent = `☁️ ${providerName}`;
  fillEl.style.width = '50%';

  try {
    const result = await invoke('cloud_upload_file', { provider, filePath });
    fillEl.style.width = '100%';
    sizeEl.textContent = `✅ ${providerName} 업로드 완료!`;

    // Insert share link as a reply
    if (state.currentThreadId) {
      const linkMsg = `📎 대용량 파일: ${result.file_name} (${(result.file_size / 1024 / 1024).toFixed(1)}MB)\n🔗 ${result.share_link}`;
      try {
        await invoke('send_reply', {
          threadId: state.currentThreadId,
          body: linkMsg,
        });
        state.currentMessages = await invoke('get_messages', { threadId: state.currentThreadId }) || [];
        renderChatView();
      } catch(e) {}
    }

    addTelegramLog(`☁️ ${providerName} 업로드 완료: ${result.file_name}`);
    simulateTelegramNotification(`☁️ 대용량 파일 전송 완료: ${result.file_name}`);

    setTimeout(() => {
      uploadEl.style.display = 'none';
      fillEl.style.width = '0%';
    }, 2000);

  } catch(e) {
    fillEl.style.width = '100%';
    fillEl.style.background = '#ff453a';
    sizeEl.textContent = `❌ 업로드 실패: ${e}`;
    setTimeout(() => {
      uploadEl.style.display = 'none';
      fillEl.style.width = '0%';
      fillEl.style.background = '';
    }, 3000);
  }
}

// ─── Morning Briefing ───
function showMorningBriefing() {
  if (state.briefingDismissed) return;
  const hour = new Date().getHours();
  // Show between 6am and 11am, or always show (for demo purposes)
  const briefingEl = document.getElementById('morning-briefing');
  const cardsEl = document.getElementById('briefing-cards');

  // Wait until threads are loaded
  setTimeout(() => {
    const html = renderBriefingCards(state.threads);
    if (html) {
      cardsEl.innerHTML = html;
      briefingEl.style.display = 'block';
    }
  }, 500);
}

function dismissBriefing() {
  state.briefingDismissed = true;
  document.getElementById('morning-briefing').style.display = 'none';
}

function refreshBriefingCards() {
  const cardsEl = document.getElementById('briefing-cards');
  const briefingEl = document.getElementById('morning-briefing');
  const html = renderBriefingCards(state.threads);
  if (html) {
    cardsEl.innerHTML = html;
    briefingEl.style.display = 'block';
  } else {
    briefingEl.style.display = 'none';
  }
}
window.refreshBriefingCards = refreshBriefingCards;

// ─── Telegram Simulation ───
async function initTelegramStatus() {
  const dotEl = document.getElementById('telegram-dot');
  try {
    const link = await invoke('telegram_get_status');
    state.telegramConnected = !!link;
    dotEl.classList.toggle('connected', state.telegramConnected);
    if (link) addTelegramLog('🔗 @' + (link.username || 'User') + ' 연결됨');
    else addTelegramLog('⚪ 연결 안됨 — 설정에서 연결하세요');
  } catch(e) {
    state.telegramConnected = false;
    dotEl.classList.toggle('connected', false);
  }
}

function addTelegramLog(message) {
  state.telegramLog.push({ time: new Date().toLocaleTimeString('ko-KR', { hour:'2-digit', minute:'2-digit' }), message });
  const logEl = document.getElementById('telegram-log');
  if (logEl) {
    logEl.innerHTML = state.telegramLog.slice(-10).reverse().map(entry =>
      `<div class="telegram-log-entry"><span>${entry.time}</span><span>${entry.message}</span></div>`
    ).join('');
  }
}

async function simulateTelegramNotification(text) {
  if (state.telegramConnected) {
    addTelegramLog('📤 ' + text);
  }
}

// ─── Whisper Notes (per-thread private notes → DB backed) ───
function loadWhisperNotes() {
  // Legacy: load from localStorage as fallback
  try {
    const stored = localStorage.getItem('lumina_whisper_notes');
    if (stored) state.whisperNotes = JSON.parse(stored);
  } catch (e) { /* ignore */ }
}

let _memoSaveTimer = null;
function saveWhisperNotes() {
  // Save to DB with debounce
  if (state.currentThreadId) {
    clearTimeout(_memoSaveTimer);
    _memoSaveTimer = setTimeout(() => {
      invoke('save_memo_cmd', {
        threadId: state.currentThreadId,
        content: state.whisperNotes[state.currentThreadId] || ''
      }).catch(console.error);
    }, 500);
  }
}

async function loadWhisperForThread() {
  const notepad = document.getElementById('whisper-notepad');
  if (notepad && state.currentThreadId) {
    try {
      const memo = await invoke('get_memo_cmd', { threadId: state.currentThreadId });
      if (memo !== null && memo !== undefined) {
        state.whisperNotes[state.currentThreadId] = memo;
      }
    } catch (e) { /* fallback to cache */ }
    notepad.value = state.whisperNotes[state.currentThreadId] || '';
  }
}

// ─── Wiki / Knowledge Base ───
function openWikiModal() {
  const modal = document.getElementById('wiki-modal');
  const content = document.getElementById('wiki-content');
  content.innerHTML = generateWikiContent(state.threads, state.allMessages);
  modal.style.display = 'flex';
}

function closeWikiModal() {
  document.getElementById('wiki-modal').style.display = 'none';
}

// ─── Email Modal ───
function openEmailModal(msgId) {
  const msg = state.currentMessages.find(m => m.id === msgId);
  if (!msg || !msg.body_original) return;

  const modal = document.getElementById('email-modal');
  const data = renderEmailModal(msg);

  document.getElementById('modal-subject').textContent = data.subject;
  document.getElementById('modal-sender').textContent = `보낸 사람: ${data.sender}`;
  document.getElementById('modal-date').textContent = data.date;

  const bodyContainer = document.getElementById('modal-body');
  bodyContainer.innerHTML = '';

  const iframe = document.createElement('iframe');
  iframe.style.cssText = 'width:100%;border:none;min-height:300px;background:white;border-radius:10px;';
  bodyContainer.appendChild(iframe);

  requestAnimationFrame(() => {
    const doc = iframe.contentDocument || iframe.contentWindow.document;
    // Anti-tracking: strip tracking pixels from HTML
    const cleanHtml = stripTrackingPixels(data.html);
    doc.open();
    doc.write(`<!DOCTYPE html><html><head><meta charset="UTF-8">
      <style>body{font-family:-apple-system,sans-serif;padding:16px;margin:0;color:#333;line-height:1.6;}
      table{font-size:14px;}img{max-width:100%;}</style></head>
      <body>${cleanHtml}</body></html>`);
    doc.close();
    setTimeout(() => { iframe.style.height = (doc.body.scrollHeight + 20) + 'px'; }, 100);
  });

  modal.style.display = 'flex';
}

// Anti-tracking: strip 1x1 pixel images
function stripTrackingPixels(html) {
  if (!html) return '';
  return html
    .replace(/<img[^>]*(?:width|height)\s*=\s*["']?1["']?[^>]*>/gi, '<!-- tracking pixel blocked -->')
    .replace(/<img[^>]*(?:pixel|track|beacon)[^>]*>/gi, '<!-- tracking pixel blocked -->');
}

function closeEmailModal() {
  document.getElementById('email-modal').style.display = 'none';
}

// ─── Event Listeners ───
function setupEventListeners() {
  // Thread click
  document.getElementById('thread-list').addEventListener('click', (e) => {
    const item = e.target.closest('.thread-item');
    if (item) selectThread(item.dataset.threadId);
  });

  // Thread right-click context menu (delegated - immune to special chars in email data)
  document.getElementById('thread-list').addEventListener('contextmenu', (e) => {
    const item = e.target.closest('.thread-item');
    if (item && window._threadContextMap) {
      const threadData = window._threadContextMap[item.dataset.threadId];
      if (threadData) {
        showThreadContextMenu(e, threadData);
      }
    }
  });

  // Briefing card click
  document.getElementById('briefing-cards').addEventListener('click', (e) => {
    const card = e.target.closest('.briefing-card');
    if (card) {
      selectThread(card.dataset.threadId);
      // Removed dismissBriefing() to keep it always visible
    }
  });

  // Bubble click (original email)
  document.getElementById('chat-messages').addEventListener('click', (e) => {
    const bubble = e.target.closest('.bubble.has-original');
    if (bubble) openEmailModal(bubble.dataset.msgId);
  });

  // Modal close buttons
  document.getElementById('modal-close').addEventListener('click', closeEmailModal);
  document.getElementById('email-modal').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeEmailModal();
  });
  document.getElementById('wiki-close').addEventListener('click', closeWikiModal);
  document.getElementById('wiki-modal').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) closeWikiModal();
  });
  document.getElementById('telegram-close').addEventListener('click', () => {
    document.getElementById('telegram-modal').style.display = 'none';
  });
  document.getElementById('telegram-modal').addEventListener('click', (e) => {
    if (e.target === e.currentTarget) e.currentTarget.style.display = 'none';
  });

  // Escape closes modals
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeEmailModal();
      closeWikiModal();
      document.getElementById('telegram-modal').style.display = 'none';
    }
  });

  // Wiki button
  document.getElementById('wiki-btn').addEventListener('click', openWikiModal);

  // Briefing dismiss
  document.getElementById('briefing-close').addEventListener('click', dismissBriefing);

  // Thread jump close
  document.getElementById('thread-jump-close').addEventListener('click', () => {
    document.getElementById('thread-jump').style.display = 'none';
  });

  // Filter buttons
  document.querySelectorAll('.filter-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      state.filter = btn.dataset.filter;
      await loadThreads();
    });
  });

  // Input tab switching
  document.querySelectorAll('.input-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.input-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      const input = document.getElementById('reply-input');
      if (tab.dataset.tab === 'whisper') {
        input.placeholder = '내부 메모를 입력하세요... (팀원만 볼 수 있습니다)';
        input.style.borderColor = 'rgba(255, 214, 10, 0.3)';
      } else {
        input.placeholder = '답장을 입력하세요...';
        input.style.borderColor = '';
      }
    });
  });

  // Auto-resize textarea + ghost text interaction (Optimized to prevent typing lag)
  const textarea = document.getElementById('reply-input');
  textarea.addEventListener('input', () => {
    // Only adjust height if scrollHeight > clientHeight to minimize layout thrashing
    if (textarea.scrollHeight > textarea.clientHeight) {
      textarea.style.height = 'auto';
      textarea.style.height = Math.min(textarea.scrollHeight, 120) + 'px';
    }

    // Use textContent instead of innerText! innerText forces a synchronous layout calculation (Reflow)
    // which causes severe typing lag, especially with IME (Korean). textContent is O(n) string concat without layout.
    const hasText = textarea.textContent.trim().length > 0;
    const ghostEl = document.getElementById('ghost-text');
    
    if (hasText) {
      ghostEl.style.display = 'none';
    } else {
      // If empty, reset height and show ghost draft
      textarea.style.height = 'auto';
      showGhostDraft();
    }
  });

  // Tab key accepts ghost draft
  textarea.addEventListener('keydown', (e) => {
    if (e.isComposing) return; // Ignore during IME composition
    
    if (e.key === 'Tab' && state.ghostDraft && textarea.textContent.trim().length === 0) {
      e.preventDefault();
      acceptGhostDraft();
    }
  });

  // Send button with Undo Send
  document.getElementById('send-btn').addEventListener('click', () => {
    const input = document.getElementById('reply-input');
    const htmlObj = input.innerHTML;
    const textObj = input.innerText.trim();
    if (!htmlObj || !state.currentThreadId) return;

    const isWhisper = document.querySelector('.input-tab.active')?.dataset.tab === 'whisper';
    input.innerHTML = '';
    input.style.height = 'auto';
    document.getElementById('ghost-text').style.display = 'none';

    // Start undo send countdown
    startUndoSend(textObj, htmlObj, isWhisper);
  });

  // Undo send cancel
  document.getElementById('undo-send-btn').addEventListener('click', cancelUndoSend);

  // Enter to send
  textarea.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey) {
      e.preventDefault();
      document.getElementById('send-btn').click();
    }
  });

  // Search with tag chips
  const searchInput = document.getElementById('search-input');
  const chipsArea = document.getElementById('search-chips-area');
  const searchChips = []; // array of tag strings in chips

  // Click on chips area focuses input
  chipsArea.addEventListener('click', () => searchInput.focus());

  function createChip(tag) {
    const normalized = tag.replace(/^#/, '').trim();
    if (!normalized || searchChips.includes(normalized)) return;

    searchChips.push(normalized);

    const chip = document.createElement('span');
    chip.className = 'search-chip';
    chip.dataset.tag = normalized;
    chip.innerHTML = `🏷️ ${normalized} <span class="chip-remove">×</span>`;

    chip.querySelector('.chip-remove').addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = searchChips.indexOf(normalized);
      if (idx > -1) searchChips.splice(idx, 1);
      chip.remove();
      triggerSearch();
      searchInput.focus();
    });

    chipsArea.insertBefore(chip, searchInput);
    searchInput.value = '';
    searchInput.placeholder = searchChips.length > 0 ? '추가 검색...' : '메일 검색...';
    triggerSearch();
  }

  searchInput.addEventListener('keydown', (e) => {
    const val = searchInput.value;

    // Comma or Enter after #tag → create chip
    if ((e.key === ',' || e.key === 'Enter') && val.trim().startsWith('#')) {
      e.preventDefault();
      createChip(val.trim());
      return;
    }

    // Backspace on empty input → remove last chip
    if (e.key === 'Backspace' && val === '' && searchChips.length > 0) {
      const lastChip = chipsArea.querySelector('.search-chip:last-of-type');
      if (lastChip) {
        const tag = lastChip.dataset.tag;
        const idx = searchChips.indexOf(tag);
        if (idx > -1) searchChips.splice(idx, 1);
        lastChip.remove();
        searchInput.placeholder = searchChips.length > 0 ? '추가 검색...' : '메일 검색...';
        triggerSearch();
      }
    }
  });

  searchInput.addEventListener('input', () => triggerSearch());

  function triggerSearch() {
    const freeText = searchInput.value.toLowerCase().trim();

    document.querySelectorAll('.thread-item').forEach(item => {
      if (searchChips.length === 0 && !freeText) {
        item.style.display = '';
        return;
      }

      const text = item.textContent.toLowerCase();
      const threadId = item.dataset.threadId;
      const threadData = state.threads.find(t => t.thread_id === threadId);
      const aiTags = (threadData && threadData.ai_tags) ? threadData.ai_tags.toLowerCase() : '';
      const combined = text + ' ' + aiTags;

      // All chip tags must match (AND)
      const chipsMatch = searchChips.every(chip => combined.includes(chip));
      // Free text must also match (if present)
      const textMatch = !freeText || combined.includes(freeText.replace(/^#/, ''));

      item.style.display = (chipsMatch && textMatch) ? '' : 'none';
    });
  }

  // Attachment search toggle
  document.getElementById('attachment-search-btn').addEventListener('click', () => {
    const searchInput = document.getElementById('search-input');
    if (searchInput.placeholder === '첨부파일 검색...') {
      searchInput.placeholder = '메일 검색...';
    } else {
      searchInput.placeholder = '첨부파일 검색... (파일명 입력)';
      searchInput.focus();
    }
  });

  // Attach button — real file picker with cloud hybrid
  document.getElementById('attach-btn').addEventListener('click', () => {
    const fileInput = document.createElement('input');
    fileInput.type = 'file';
    fileInput.style.display = 'none';
    fileInput.addEventListener('change', async (e) => {
      const file = e.target.files[0];
      if (!file) return;

      const sizeMB = file.size / (1024 * 1024);
      if (sizeMB > CLOUD_THRESHOLD_MB) {
        // Large file: use cloud upload
        // Create a temp path from the file name for the backend
        // In Tauri, we'd use the dialog API, but for now we use the web File API
        handleCloudUpload(file.name, file.name, file.size);
      } else {
        // Small file: simulated inline attach
        simulateUpload(file.name, sizeMB);
      }
      fileInput.remove();
    });
    document.body.appendChild(fileInput);
    fileInput.click();
  });

  // Whisper notepad save
  const notepad = document.getElementById('whisper-notepad');
  notepad.addEventListener('input', () => {
    if (state.currentThreadId) {
      state.whisperNotes[state.currentThreadId] = notepad.value;
      saveWhisperNotes();
    }
  });

  // Telegram status click
  document.getElementById('telegram-status').addEventListener('click', () => {
    document.getElementById('telegram-modal').style.display = 'flex';
  });

  // Manual Tag button
  const addTagBtn = document.getElementById('add-tag-btn');
  if (addTagBtn) {
    addTagBtn.addEventListener('click', addManualTag);
  }
}

// ─── Manual Tagging ───
async function addManualTag() {
  if (!state.currentThreadId) {
    showToast('태그를 추가할 대화를 먼저 선택해주세요.');
    return;
  }

  const btn = document.getElementById('add-tag-btn');
  if (!btn) return;

  // Check if input already visible
  if (btn.nextElementSibling && btn.nextElementSibling.classList.contains('tag-input-wrap')) {
    btn.nextElementSibling.querySelector('input').focus();
    return;
  }

  // Create inline input
  const wrap = document.createElement('div');
  wrap.className = 'tag-input-wrap';
  wrap.style.cssText = 'display:flex;gap:4px;margin-top:6px;align-items:center;';
  wrap.innerHTML = `
    <input type="text" id="manual-tag-input" placeholder="태그 입력 (예: 긴급)" 
      style="flex:1;padding:4px 8px;border-radius:8px;border:1px solid var(--border-subtle);background:rgba(255,255,255,0.05);color:var(--text-primary);font-size:12px;outline:none;" />
    <button id="manual-tag-confirm" style="padding:4px 8px;border-radius:8px;border:none;background:linear-gradient(135deg,#667eea,#764ba2);color:white;font-size:11px;cursor:pointer;">추가</button>
    <button id="manual-tag-cancel" style="padding:4px 8px;border-radius:8px;border:1px solid var(--border-subtle);background:transparent;color:var(--text-muted);font-size:11px;cursor:pointer;">✕</button>
  `;
  btn.parentElement.appendChild(wrap);

  const input = wrap.querySelector('#manual-tag-input');
  const confirmBtn = wrap.querySelector('#manual-tag-confirm');
  const cancelBtn = wrap.querySelector('#manual-tag-cancel');

  input.focus();

  async function submitTag() {
    const tag = input.value.trim();
    if (!tag) return;

    console.log('[ManualTag] Adding tag:', tag, 'to thread:', state.currentThreadId);
    showToast(`태그 "${tag}" 추가 중...`);

    try {
      await invoke('add_thread_tag_cmd', { threadId: state.currentThreadId, tag: tag });
      console.log('[ManualTag] Tag added successfully');
      showToast(`✅ 태그 "${tag}" 추가 완료!`);

      // Remove input first
      wrap.remove();

      // Refresh
      const scrollPos = document.getElementById('thread-list').scrollTop;
      await loadThreads();
      if (state.currentThreadId) {
        await selectThread(state.currentThreadId);
      }
      document.getElementById('thread-list').scrollTop = scrollPos;
    } catch (err) {
      console.error('[ManualTag] 태그 추가 실패:', err);
      showToast(`❌ 태그 추가 실패: ${err}`);
    }
  }

  confirmBtn.addEventListener('click', submitTag);
  cancelBtn.addEventListener('click', () => wrap.remove());
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') submitTag();
    if (e.key === 'Escape') wrap.remove();
  });
}
window.addManualTag = addManualTag;

function showToast(message) {
  let toast = document.getElementById('simple-toast');
  if (!toast) {
    toast = document.createElement('div');
    toast.id = 'simple-toast';
    toast.style.cssText = 'position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:rgba(0,0,0,0.85);color:white;padding:10px 20px;border-radius:10px;font-size:13px;z-index:99999;transition:opacity 0.3s;';
    document.body.appendChild(toast);
  }
  toast.textContent = message;
  toast.style.opacity = '1';
  setTimeout(() => { toast.style.opacity = '0'; }, 2500);
}

// ─── Helpers ───
function getStatusLabel(status) {
  return { open:'Open', progress:'진행 중', closed:'완료' }[status] || status;
}

// ─── Compose Modal Logic ───
window.openComposeModal = async function(replyTo) {
  const modal = document.getElementById('compose-modal');
  document.getElementById('compose-to').value = replyTo || '';
  document.getElementById('compose-cc').value = '';
  document.getElementById('compose-bcc').value = '';
  document.getElementById('cc-bcc-container').style.display = 'none';
  const toggleBtn = document.getElementById('toggle-cc-bcc-btn');
  if (toggleBtn) toggleBtn.textContent = '참조/숨은참조 추가';
  document.getElementById('compose-subject').value = '';
  document.getElementById('compose-body').innerHTML = '';
  
  state.composeAttachments = [];
  renderComposeAttachments();

  // Populate signature dropdown
  try {
    const sigs = await invoke('get_signatures_cmd');
    const select = document.getElementById('compose-signature');
    select.innerHTML = '<option value="">서명 없음</option>';
    sigs.forEach(s => {
      const opt = document.createElement('option');
      opt.value = s.id;
      opt.textContent = s.name;
      if (s.is_default) opt.selected = true;
      select.appendChild(opt);
    });
  } catch (e) { /* ignore */ }
  
  modal.style.display = 'flex';
};

function renderComposeAttachments() {
  const container = document.getElementById('compose-attachments');
  if (!container) return;
  if (!state.composeAttachments || state.composeAttachments.length === 0) {
    container.style.display = 'none';
    container.innerHTML = '';
    return;
  }
  container.style.display = 'flex';
  container.innerHTML = state.composeAttachments.map((file, i) => `
    <div style="display:flex; align-items:center; background:var(--bg-secondary); padding:4px 8px; border-radius:6px; border:1px solid var(--border-subtle); gap:6px;">
      <span style="font-size:12px; color:var(--text-primary); max-width:150px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">${escapeHtml(file.name)}</span>
      <button type="button" onclick="window.removeComposeAttachment(${i})" style="background:none; border:none; color:var(--text-tertiary); cursor:pointer; font-size:14px; padding:0; line-height:1;">&times;</button>
    </div>
  `).join('');
}

window.removeComposeAttachment = (index) => {
  if (state.composeAttachments) {
    state.composeAttachments.splice(index, 1);
    renderComposeAttachments();
  }
};

document.getElementById('compose-attach-btn')?.addEventListener('click', async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: true,
      title: '첨부파일 선택',
    });
    if (selected) {
      if (!state.composeAttachments) state.composeAttachments = [];
      const paths = Array.isArray(selected) ? selected : [selected];
      
      // We extract filename from path for display
      for (const p of paths) {
        // Handle both Windows and POSIX paths
        const name = p.split(/[\\/]/).pop();
        // Prevent duplicates
        if (!state.composeAttachments.some(a => a.path === p)) {
          state.composeAttachments.push({ path: p, name: name });
        }
      }
      renderComposeAttachments();
    }
  } catch (err) {
    console.error('File dialog error:', err);
  }
});

document.getElementById('compose-send-btn')?.addEventListener('click', async () => {
  const to = document.getElementById('compose-to').value.trim();
  const cc = document.getElementById('compose-cc').value.trim();
  const bcc = document.getElementById('compose-bcc').value.trim();
  const subject = document.getElementById('compose-subject').value.trim();
  const bodyHtml = document.getElementById('compose-body').innerHTML;
  const attachments = (state.composeAttachments || []).map(a => a.path);
  
  if (!to) return alert('받는 사람을 입력해주세요.');
  if (!subject) return alert('제목을 입력해주세요.');
  
  const btn = document.getElementById('compose-send-btn');
  btn.textContent = '⏳ 전송 중...';
  btn.disabled = true;
  
  try {
    await invoke('send_email_cmd', { to, cc, bcc, subject, bodyHtml, attachments });
    document.getElementById('compose-modal').style.display = 'none';
    await loadThreads();
    alert('✅ 메일이 전송되었습니다!');
  } catch (err) {
    alert('❌ 전송 실패: ' + err);
  } finally {
    btn.textContent = '📤 보내기';
    btn.disabled = false;
  }
});

// ─── Contact Autocomplete ───
document.getElementById('toggle-cc-bcc-btn')?.addEventListener('click', (e) => {
  e.preventDefault();
  const container = document.getElementById('cc-bcc-container');
  if (container.style.display === 'none') {
    container.style.display = 'block';
    e.target.textContent = '참조/숨은참조 닫기';
  } else {
    container.style.display = 'none';
    e.target.textContent = '참조/숨은참조 추가';
    document.getElementById('compose-cc').value = '';
    document.getElementById('compose-bcc').value = '';
  }
});

function attachContactAutocomplete(inputId, dropdownId) {
  const input = document.getElementById(inputId);
  const dropdown = document.getElementById(dropdownId);
  if (!input || !dropdown) return;

  let activeIndex = -1;
  let matches = [];

  const renderDropdown = () => {
    dropdown.innerHTML = '';
    if (matches.length === 0) {
      dropdown.style.display = 'none';
      return;
    }
    matches.forEach((contact, idx) => {
      const item = document.createElement('div');
      item.className = 'autocomplete-item' + (idx === activeIndex ? ' active' : '');
      item.innerHTML = `
        <span class="autocomplete-name">${escapeHtml(contact.name)}</span>
        <span class="autocomplete-email">${escapeHtml(contact.email)}</span>
      `;
      item.addEventListener('mousedown', (e) => {
        e.preventDefault(); // Prevent input blur
        input.value = `${contact.name} <${contact.email}>`;
        dropdown.style.display = 'none';
      });
      dropdown.appendChild(item);
    });
    dropdown.style.display = 'block';
    
    // Auto-scroll to active item
    const activeItem = dropdown.querySelector('.autocomplete-item.active');
    if (activeItem) {
      activeItem.scrollIntoView({ block: 'nearest' });
    }
  };

  input.addEventListener('input', () => {
    const val = input.value.trim().toLowerCase();
    activeIndex = -1;
    if (val.length < 1) {
      matches = [];
      renderDropdown();
      return;
    }
    matches = (state.contacts || []).filter(c => 
      c.name.toLowerCase().includes(val) || c.email.toLowerCase().includes(val)
    ).slice(0, 10); // Limit to top 10
    renderDropdown();
  });

  document.addEventListener('mousedown', (e) => {
    if (!input.contains(e.target) && !dropdown.contains(e.target)) {
      dropdown.style.display = 'none';
      activeIndex = -1;
    }
  });

  input.addEventListener('focus', () => { if (matches.length > 0 && input.value.trim().length > 0) dropdown.style.display = 'block'; });

  input.addEventListener('keydown', (e) => {
    if (matches.length === 0 || dropdown.style.display === 'none') return;
    if (e.isComposing) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      activeIndex = (activeIndex + 1) % matches.length;
      renderDropdown();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      activeIndex = activeIndex - 1 < 0 ? matches.length - 1 : activeIndex - 1;
      renderDropdown();
    } else if (e.key === 'Enter') {
      if (activeIndex >= 0 && activeIndex < matches.length) {
        e.preventDefault();
        input.value = `${matches[activeIndex].name} <${matches[activeIndex].email}>`;
        dropdown.style.display = 'none';
        matches = [];
      }
    } else if (e.key === 'Escape') {
      dropdown.style.display = 'none';
    }
  });
}

// Attach autocomplete to Compose fields
attachContactAutocomplete('compose-to', 'autocomplete-to');
attachContactAutocomplete('compose-cc', 'autocomplete-cc');
attachContactAutocomplete('compose-bcc', 'autocomplete-bcc');

// ─── Rich Text Editor (RTE) Logic ───
document.querySelectorAll('.rte-btn[data-command]').forEach(btn => {
  btn.addEventListener('mousedown', (e) => {
    e.preventDefault(); // Prevent losing focus from editor
    const command = btn.getAttribute('data-command');
    document.execCommand(command, false, null);
  });
});

document.getElementById('rte-link-btn')?.addEventListener('mousedown', (e) => {
  e.preventDefault();
  const url = prompt('링크 URL을 입력하세요 (예: https://example.com):', 'https://');
  if (url && url !== 'https://') {
    document.execCommand('createLink', false, url);
  }
});

// ─── macOS Menu Event Handler ───
try {
  const { listen } = window.__TAURI__.event || await import('@tauri-apps/api/event');
  listen('menu-action', async (event) => {
    const action = event.payload;
    switch (action) {
      case 'settings':
        document.getElementById('settings-modal').style.display = 'flex';
        break;
      case 'compose':
        window.openComposeModal();
        break;
      case 'reply':
        if (state.currentThread) {
          window.openComposeModal(state.currentThread.sender_name);
        }
        break;
      case 'forward':
        if (state.currentThread) {
          window.openComposeModal(null, `Fwd: ${state.currentThread.subject || ''}`);
        }
        break;
      case 'sync':
        invoke('sync_all_accounts_cmd').catch(console.error);
        break;
      case 'toggle_unread':
        if (state.currentThreadId) {
          await invoke('toggle_thread_unread', { threadId: state.currentThreadId });
          await loadThreads();
        }
        break;
      case 'toggle_important':
        if (state.currentThreadId) {
          await invoke('toggle_thread_important', { threadId: state.currentThreadId });
          await loadThreads();
        }
        break;
      case 'toggle_pin':
        if (state.currentThreadId) {
          await invoke('toggle_thread_pin', { threadId: state.currentThreadId });
          await loadThreads();
        }
        break;
      case 'archive':
        if (state.currentThreadId) {
          window.pushUndo({ type: 'delete', threadId: state.currentThreadId });
          await invoke('delete_thread_cmd', { threadId: state.currentThreadId });
          await loadThreads();
        }
        break;
      case 'filter_all':
        document.querySelector('.filter-btn[data-filter="all"]')?.click();
        break;
      case 'filter_unread':
        document.querySelector('.filter-btn[data-filter="unread"]')?.click();
        break;
      case 'filter_pinned':
        document.querySelector('.filter-btn[data-filter="pinned"]')?.click();
        break;
      case 'filter_trash':
        document.querySelector('.filter-btn[data-filter="trash"]')?.click();
        break;
      case 'filter_spam':
        document.querySelector('.filter-btn[data-filter="spam"]')?.click();
        break;
      case 'contacts':
        document.querySelector('.sidebar-nav-item[data-view="groups"]')?.click();
        break;
      case 'shortcut_guide':
        toggleShortcutGuide();
        break;
    }
  });
} catch (e) { /* menu events not available outside Tauri */ }

// ─── Settings Tabs removed from app.js to solve load order collision ───
// Handled directly natively in settings.js now.

window.saveNewSignature = async function() {
  const name = document.getElementById('sig-name')?.value.trim();
  const bodyHtml = document.getElementById('sig-body')?.value.trim();
  const isDefault = document.getElementById('sig-default')?.checked || false;
  if (!name) return alert('서명 이름을 입력해주세요.');
  try {
    await invoke('save_signature_cmd', { name, bodyHtml, isDefault });
    window.switchSettingsTab('signatures');
  } catch (e) { alert('저장 실패: ' + e); }
};

window.deleteSignature = async function(id) {
  try {
    await invoke('delete_signature_cmd', { id });
    window.switchSettingsTab('signatures');
  } catch (e) { alert('삭제 실패: ' + e); }
};

// ─── AI Settings Functions ───
window.saveAISettingsBtn = async function() {
  const keyEl = document.getElementById('ai-api-key');
  const modelEl = document.getElementById('ai-model');
  const key = keyEl?.value.trim();
  const model = modelEl?.value;
  
  if (!key && !keyEl.placeholder.includes('●')) return alert('API 키를 입력해주세요.');
  
  try {
    if (key) {
      await invoke('save_gemini_api_key', { apiKey: key });
    }
    if (model) {
      await invoke('save_app_setting', { key: 'gemini_model', value: model });
    }
    const resultEl = document.getElementById('ai-test-result');
    resultEl.style.display = 'block';
    resultEl.style.background = 'var(--bg-tertiary)';
    resultEl.style.color = 'var(--accent)';
    resultEl.textContent = '✅ AI 설정이 저장되었습니다.';
    
    if (key) {
      keyEl.value = '';
      keyEl.placeholder = '●'.repeat(Math.max(0, key.length - 4)) + key.slice(-4);
    }
  } catch (e) { alert('저장 실패: ' + e); }
};

window.testAIConnection = async function() {
  const resultEl = document.getElementById('ai-test-result');
  resultEl.style.display = 'block';
  resultEl.style.background = 'var(--bg-tertiary)';
  resultEl.style.color = 'var(--text-secondary)';
  resultEl.textContent = '🔄 연결 테스트 중...';
  try {
    const msg = await invoke('test_ai_connection_cmd');
    resultEl.style.color = '#38ef7d';
    resultEl.textContent = msg;
  } catch (e) {
    resultEl.style.color = 'var(--danger)';
    resultEl.textContent = e;
  }
};

window.syncContactsFromMail = async function() {
  try {
    const count = await invoke('sync_contacts_from_mail_cmd');
    alert(`📇 ${count}명의 연락처를 동기화했습니다.`);
  } catch (e) { alert('동기화 실패: ' + e); }
};

// ─── Globals for cross-module access ───
window.selectThread = selectThread;
window.renderThreadList = renderThreadList;
window.state = state;
