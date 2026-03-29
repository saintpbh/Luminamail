// ═══════════════════════════════════════════════════
// Context Menu System - Right-click / Ctrl-click
// Provides native-feel context menus for threads & messages
// ═══════════════════════════════════════════════════

const { invoke } = window.__TAURI__.core;

let activeContextMenu = null;

// ── Close any active context menu ──
function closeContextMenu() {
  if (activeContextMenu) {
    activeContextMenu.remove();
    activeContextMenu = null;
  }
}

// ── Create and show context menu ──
function showContextMenu(e, items) {
  e.preventDefault();
  e.stopPropagation();
  closeContextMenu();

  const menu = document.createElement('div');
  menu.className = 'context-menu';

  items.forEach(item => {
    if (item.type === 'divider') {
      const div = document.createElement('div');
      div.className = 'context-menu-divider';
      menu.appendChild(div);
      return;
    }
    if (item.type === 'header') {
      const hdr = document.createElement('div');
      hdr.className = 'context-menu-header';
      hdr.textContent = item.label;
      menu.appendChild(hdr);
      return;
    }

    const el = document.createElement('div');
    el.className = `context-menu-item ${item.danger ? 'danger' : ''}`;
    el.innerHTML = `
      <span class="cm-icon">${item.icon || ''}</span>
      <span class="cm-label">${item.label}</span>
      ${item.shortcut ? `<span class="cm-shortcut">${item.shortcut}</span>` : ''}
    `;
    el.addEventListener('click', () => {
      closeContextMenu();
      if (item.action) item.action();
    });
    menu.appendChild(el);
  });

  document.body.appendChild(menu);
  activeContextMenu = menu;

  // Position: keep within viewport
  // Use offsetWidth/Height instead of getBoundingClientRect to ignore CSS transform scale() animation state
  const width = menu.offsetWidth;
  const height = menu.offsetHeight;
  let x = e.clientX;
  let y = e.clientY;
  
  if (x + width > window.innerWidth) x = window.innerWidth - width - 8;
  if (y + height > window.innerHeight) y = window.innerHeight - height - 8;
  
  // Safe boundaries
  if (x < 8) x = 8;
  if (y < 8) y = 8;

  menu.style.left = x + 'px';
  menu.style.top = y + 'px';
}

// Close on click anywhere else
document.addEventListener('click', closeContextMenu);
document.addEventListener('contextmenu', (e) => {
  // Only close if not handled by our custom menus
  if (!e.defaultPrevented) closeContextMenu();
  // Always suppress the native macOS browser context menu
  e.preventDefault();
});

// ╔═══════════════════════════════════════════════╗
// ║   THREAD LIST CONTEXT MENU                    ║
// ╚═══════════════════════════════════════════════╝

function showThreadContextMenu(e, thread) {
  const items = [
    { type: 'header', label: '메일 관리' },
    {
      icon: thread.pinned ? '📌' : '📍',
      label: thread.pinned ? '핀 해제' : '핀 고정',
      shortcut: '⌘P',
      action: async () => {
        window.pushUndo({ type: 'pin', threadId: thread.thread_id });
        await invoke('toggle_thread_pin', { threadId: thread.thread_id });
        refreshThreadList();
      }
    },
    {
      icon: thread.important ? '⭐' : '☆',
      label: thread.important ? '중요 해제' : '중요 표시',
      shortcut: '⌘I',
      action: async () => {
        window.pushUndo({ type: 'important', threadId: thread.thread_id });
        await invoke('toggle_thread_important', { threadId: thread.thread_id });
        refreshThreadList();
      }
    },
    {
      icon: thread.unread_count > 0 ? '📭' : '📬',
      label: thread.unread_count > 0 ? '읽음으로 표시' : '안 읽음으로 표시',
      action: async () => {
        window.pushUndo({ type: 'unread', threadId: thread.thread_id });
        await invoke('toggle_thread_unread', { threadId: thread.thread_id });
        refreshThreadList();
      }
    },
    {
      icon: thread.is_briefing ? '🌤️' : '☀️',
      label: thread.is_briefing ? '브리핑에서 제외' : '아침 브리핑에 추가',
      shortcut: '⌘B',
      action: async () => {
        window.pushUndo({ type: 'briefing', threadId: thread.thread_id, is_briefing: thread.is_briefing });
        await invoke('toggle_briefing', { threadId: thread.thread_id, is_briefing: !thread.is_briefing });
        refreshThreadList();
      }
    },
    { type: 'divider' },
    { type: 'header', label: '변환' },
    {
      icon: '✅',
      label: 'Todo로 추가',
      shortcut: '⌘T',
      action: () => {
        window.addTodoFromMail(thread.thread_id, '', thread.subject);
      }
    },
    {
      icon: '📅',
      label: '캘린더에 추가',
      shortcut: '⌘E',
      action: () => {
        window.showAddEventModal(thread.thread_id, '', thread.subject);
      }
    },
    { type: 'divider' }
  ];

  if (window.state && (window.state.filter === 'trash' || window.state.filter === 'spam')) {
    items.push({
      icon: '♻️',
      label: '복구 (받은편지함으로 이동)',
      action: async () => {
        await invoke('restore_thread_cmd', { threadId: thread.thread_id });
        refreshThreadList();
      }
    });
  } else {
    items.push({
      icon: '🗑️',
      label: '로컬 삭제',
      danger: true,
      action: async () => {
        window.pushUndo({ type: 'delete', threadId: thread.thread_id });
        await invoke('delete_thread_cmd', { threadId: thread.thread_id });
        refreshThreadList();
      }
    });
    items.push({
      icon: '🚫',
      label: '스팸 차단',
      danger: true,
      action: async () => {
        window.pushUndo({ type: 'spam', threadId: thread.thread_id });
        await invoke('spam_thread_cmd', { threadId: thread.thread_id });
        refreshThreadList();
      }
    });
    items.push({ type: 'divider' });
    items.push({ type: 'header', label: '한동안 안보기 (알림 음소거)' });
    items.push({
      icon: '🔇',
      label: '1개월간 숨기기',
      action: async () => {
        window.pushUndo({ type: 'snooze', threadId: thread.thread_id });
        await invoke('snooze_thread_cmd', { threadId: thread.thread_id, months: 1 });
        refreshThreadList();
      }
    });
    items.push({
      icon: '🔇',
      label: '3개월간 숨기기',
      action: async () => {
        window.pushUndo({ type: 'snooze', threadId: thread.thread_id });
        await invoke('snooze_thread_cmd', { threadId: thread.thread_id, months: 3 });
        refreshThreadList();
      }
    });
    items.push({
      icon: '🔇',
      label: '1년간 숨기기',
      action: async () => {
        window.pushUndo({ type: 'snooze', threadId: thread.thread_id });
        await invoke('snooze_thread_cmd', { threadId: thread.thread_id, months: 12 });
        refreshThreadList();
      }
    });
  }
  showContextMenu(e, items);
}

// ╔═══════════════════════════════════════════════╗
// ║   MESSAGE BUBBLE CONTEXT MENU                 ║
// ╚═══════════════════════════════════════════════╝

function showBubbleContextMenu(e, msg) {
  const items = [
    { type: 'header', label: '메시지' },
    {
      icon: '📋',
      label: '텍스트 복사',
      shortcut: '⌘C',
      action: () => {
        navigator.clipboard.writeText(msg.body_summary || '');
      }
    },
    {
      icon: '↩️',
      label: '답장',
      action: () => {
        const input = document.getElementById('compose-input');
        if (input) {
          input.value = `> ${(msg.body_summary || '').slice(0, 60)}...\n`;
          input.focus();
        }
      }
    },
    {
      icon: '↗️',
      label: '전달',
      action: () => {
        navigator.clipboard.writeText(msg.body_summary || '');
        // Could open a "forward to" dialog in future
        alert('메시지가 클립보드에 복사되었습니다. 전달할 대화를 선택하세요.');
      }
    },
    { type: 'divider' },
    { type: 'header', label: '변환' },
    {
      icon: '✅',
      label: 'Todo로 추가',
      shortcut: '⌘T',
      action: () => {
        const summary = (msg.body_summary || '').slice(0, 50);
        window.addTodoFromMail(msg.thread_id, msg.id, summary);
      }
    },
    {
      icon: '📅',
      label: '캘린더에 추가',
      shortcut: '⌘E',
      action: () => {
        const summary = (msg.body_summary || '').slice(0, 50);
        window.showAddEventModal(msg.thread_id, msg.id, summary);
      }
    },
    { type: 'divider' },
    {
      icon: '📝',
      label: '위스퍼 메모 추가',
      action: () => {
        const notepad = document.getElementById('whisper-notepad');
        if (notepad) {
          notepad.value += `\n[참조] ${(msg.body_summary || '').slice(0, 40)}`;
          notepad.focus();
        }
      }
    },
  ];

  showContextMenu(e, items);
}

// ── Helper: refresh thread list ──
async function refreshThreadList() {
  try {
    const filterState = window.state ? window.state.filter : 'all';
    const threads = await invoke('get_threads', { filter: filterState });
    if (window.state) window.state.threads = threads;
    if (window.renderThreadList) window.renderThreadList();
  } catch(e) { console.error('Refresh failed:', e); }
}

// ╔═══════════════════════════════════════════════╗
// ║   BRIEFING CARD CONTEXT MENU                  ║
// ╚═══════════════════════════════════════════════╝

function showBriefingCardMenu(e, threadId, subject) {
  const dismissed = JSON.parse(localStorage.getItem('briefing_dismissed') || '[]');
  const isDismissed = dismissed.includes(threadId);

  const items = [
    { type: 'header', label: '핵심이슈 관리' },
    {
      icon: '🙈',
      label: '이 소식 내리기',
      action: () => {
        const list = JSON.parse(localStorage.getItem('briefing_dismissed') || '[]');
        if (!list.includes(threadId)) {
          list.push(threadId);
          localStorage.setItem('briefing_dismissed', JSON.stringify(list));
        }
        // Re-render briefing cards
        if (window.refreshBriefingCards) window.refreshBriefingCards();
      }
    },
    { type: 'divider' },
    {
      icon: '🔄',
      label: '모든 소식 복원',
      action: () => {
        localStorage.removeItem('briefing_dismissed');
        if (window.refreshBriefingCards) window.refreshBriefingCards();
      }
    },
  ];
  showContextMenu(e, items);
}

// ── Make functions globally available ──
window.showThreadContextMenu = showThreadContextMenu;
window.showBubbleContextMenu = showBubbleContextMenu;
window.showBriefingCardMenu = showBriefingCardMenu;
window.closeContextMenu = closeContextMenu;
window.refreshThreadList = refreshThreadList;
