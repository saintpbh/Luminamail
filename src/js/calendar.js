// ═══════════════════════════════════════════════════════════
// Lumina Mail - Calendar & Todo
// ═══════════════════════════════════════════════════════════

const { invoke: cal_invoke } = window.__TAURI__.core;

let calState = {
  currentView: 'mail', // 'mail' | 'todo' | 'calendar' | 'groups'
  // Calendar
  calYear: new Date().getFullYear(),
  calMonth: new Date().getMonth(),
  selectedDate: null,
  events: [],
  dateMails: [],
  // Todo
  todos: [],
  showTodoInput: false,
};

const MONTH_NAMES = ['1월','2월','3월','4월','5월','6월','7월','8월','9월','10월','11월','12월'];
const DAY_NAMES = ['일','월','화','수','목','금','토'];

// ── Sidebar View Switching ──
function switchSidebarView(view) {
  calState.currentView = view;
  document.querySelectorAll('.sidebar-view-tab').forEach(t => t.classList.toggle('active', t.dataset.view === view));

  document.getElementById('thread-list-section').style.display = view === 'mail' ? 'block' : 'none';
  document.getElementById('todo-section').style.display = view === 'todo' ? 'flex' : 'none';
  document.getElementById('calendar-section').style.display = view === 'calendar' ? 'flex' : 'none';
  const groupsSection = document.getElementById('groups-section');
  if (groupsSection) groupsSection.style.display = view === 'groups' ? 'flex' : 'none';

  if (view === 'todo') loadTodos();
  if (view === 'calendar') loadCalendar();
  if (view === 'groups') loadGroupsSidebar();
}

// Add to calState
calState.mailGroups = [];
calState.contactGroups = [];
calState.contacts = [];
calState.contactsSearchTerm = '';

async function loadGroupsSidebar(forceFetch = true) {
  const container = document.getElementById('groups-sidebar-list');
  if (!container) return;

  if (forceFetch) {
    try {
      calState.mailGroups = await cal_invoke('get_mail_groups') || [];
      calState.contactGroups = await cal_invoke('get_contact_groups') || [];
      calState.contacts = await cal_invoke('get_contacts') || [];
    } catch(e) { console.error(e); }
  }

  const { mailGroups, contactGroups, contacts, contactsSearchTerm } = calState;
  let html = '';

  // 0. Search Bar
  html += `
    <div style="padding: 0 16px 12px 16px; position: sticky; top: 0; background: var(--bg-primary); z-index: 10;">
      <input id="contacts-search-input" type="text" placeholder="🔍 이름 또는 이메일 검색..." 
        value="${escapeHtml(contactsSearchTerm)}"
        oninput="window.filterContacts(this.value)"
        style="width:100%; padding:8px 12px; background:var(--bg-tertiary); border:1px solid var(--border-subtle); border-radius:8px; color:var(--text-primary); font-size:13px; outline:none; transition:border-color 0.2s;">
    </div>
  `;

  // Filter contacts if term exists
  const term = contactsSearchTerm.toLowerCase();
  
  // 1. Mail Groups (Internal)
  if (mailGroups.length > 0 && !term) { // Hide groups if searching, or keep them? Let's hide if searching
    html += '<div class="thread-section-label" style="padding-left:16px;margin-top:0">메일 그룹</div>';
    html += mailGroups.map(g => `
      <div class="sidebar-group-btn" onclick="openGroupManager(); setTimeout(() => selectGroup('${g.id}'), 200)">
        <span class="group-btn-icon" style="background:${g.color};width:28px;height:28px;border-radius:8px;display:flex;align-items:center;justify-content:center;font-size:14px;color:white;font-weight:700">${g.name.charAt(0)}</span>
        <span style="flex:1">${g.name}</span>
        <span style="font-size:11px;color:rgba(255,255,255,0.35)">👥${g.member_count}</span>
      </div>
    `).join('');
  }
  
  if (!term) {
    html += `<button class="sidebar-group-btn" onclick="openGroupManager()" style="justify-content:center;color:var(--accent);margin-bottom:16px;">➕ 내부 새 그룹 만들기</button>`;
  }

  // 2. Synced Contacts & Contact Groups
  let hasVisibleContacts = false;

  if (contactGroups.length > 0) {
    if (!term) html += '<div class="thread-section-label" style="padding-left:16px">동기화된 주소록 그룹</div>';
    
    html += contactGroups.map(g => {
      let gContacts = contacts.filter(c => c.group_id === g.id);
      
      // Apply search filter
      if (term) {
        gContacts = gContacts.filter(c => 
          (c.name && c.name.toLowerCase().includes(term)) || 
          (c.email && c.email.toLowerCase().includes(term)) ||
          (c.company && c.company.toLowerCase().includes(term))
        );
      }

      if (gContacts.length === 0) return '';
      hasVisibleContacts = true;

      let groupHtml = '';
      if (!term) {
        groupHtml += `
          <div class="sidebar-group-btn" style="cursor:default; margin-bottom: 4px;">
            <span class="group-btn-icon" style="background:#555;width:28px;height:28px;border-radius:8px;display:flex;align-items:center;justify-content:center;font-size:14px;color:white;">☁️</span>
            <span style="flex:1">${g.name}</span>
            <span style="font-size:11px;color:rgba(255,255,255,0.35)">👥${contacts.filter(c => c.group_id === g.id).length}</span>
          </div>
        `;
      }

      groupHtml += `
        <div style="padding-left: ${term ? '16px' : '44px'}; padding-right: 16px; display: flex; flex-direction: column; gap: 2px; padding-bottom: 12px;">
          ${gContacts.map(c => {
            const emailOrPhone = c.email || c.phone || '';
            const composeAction = c.email ? `onclick="openComposeModal('${c.name.replace(/'/g, "\\'")} <${c.email.replace(/'/g, "\\'")}>')"` : '';
            const actionIcon = c.email ? '✉️' : '';
            return `
              <div class="contact-list-item" ${composeAction} style="font-size:13px; color:var(--text-primary); display:flex; justify-content:space-between; align-items:center; padding:8px 12px; border-radius:6px; cursor:${c.email ? 'pointer' : 'default'}; transition:background 0.2s;">
                <div style="display:flex; flex-direction:column; gap:2px;">
                  <span style="font-weight:500;">${c.name} <span style="font-size:11px; color:var(--text-tertiary); margin-left:4px;">${c.company || ''}</span></span>
                  <span style="font-size:11px; color:var(--text-secondary);">${emailOrPhone}</span>
                </div>
                <div style="font-size:14px; opacity:0.7;">${actionIcon}</div>
              </div>
            `;
          }).join('')}
        </div>
      `;
      return groupHtml;
    }).join('');
  }

  if (term && !hasVisibleContacts) {
    html += `<div style="padding: 20px; text-align: center; color: var(--text-tertiary); font-size: 13px;">검색 결과가 없습니다.</div>`;
  }

  container.innerHTML = html;
  
  if (term) {
    // Preserve focus on input if rebuilding HTML
    const input = document.getElementById('contacts-search-input');
    if (input) {
      input.focus();
      // Keep cursor at the end
      const val = input.value;
      input.value = '';
      input.value = val;
    }
  }
}

window.filterContacts = function(term) {
  calState.contactsSearchTerm = term;
  loadGroupsSidebar(false);
};

window.syncContacts = async function(provider) {
  try {
    const icon = provider === 'google' ? 'Google' : 'Apple';
    showMiniNotification(`${icon} 연락처 동기화를 시작합니다...`);
    await cal_invoke('sync_mock_contacts', { provider });
    await loadGroupsSidebar(true);
    showMiniNotification(`✅ ${icon} 동기화 완료!`);
  } catch (e) {
    console.error(e);
    alert('동기화 실패: ' + e);
  }
};

// ╔═══════════════════════════════════════════════╗
// ║   TODO                                        ║
// ╚═══════════════════════════════════════════════╝

async function loadTodos() {
  try {
    calState.todos = await cal_invoke('get_todos');
  } catch(e) { console.error(e); calState.todos = []; }
  renderTodos();
}

function renderTodos() {
  const container = document.getElementById('todo-list');
  if (!container) return;

  if (calState.todos.length === 0) {
    container.innerHTML = `<div class="date-mail-empty"><div class="empty-icon">✅</div>할 일이 없습니다</div>`;
    return;
  }

  container.innerHTML = calState.todos.map(t => {
    const isComplete = t.completed;
    const today = new Date().toISOString().slice(0,10);
    const isOverdue = t.due_date && t.due_date < today && !isComplete;
    const priorityClass = t.priority === 'high' ? 'high' : t.priority === 'low' ? 'low' : 'normal';
    const priorityLabel = t.priority === 'high' ? '긴급' : t.priority === 'low' ? '낮음' : '보통';

    return `<div class="todo-item ${isComplete ? 'completed' : ''}" data-id="${t.id}">
      <div class="todo-check" onclick="toggleTodoItem('${t.id}')"></div>
      <div class="todo-content">
        <div class="todo-title">${t.title}</div>
        <div class="todo-meta">
          ${t.due_date ? `<span class="todo-due ${isOverdue ? 'overdue' : ''}">${t.due_date.slice(5)}</span>` : ''}
          <span class="todo-priority-badge ${priorityClass}">${priorityLabel}</span>
          ${t.source_thread_id ? '<span class="todo-source">📧 메일에서</span>' : ''}
        </div>
      </div>
      <button class="todo-delete" onclick="deleteTodoItem('${t.id}')">✕</button>
    </div>`;
  }).join('');
}

function toggleTodoInput() {
  calState.showTodoInput = !calState.showTodoInput;
  const box = document.getElementById('todo-input-box');
  if (box) box.classList.toggle('visible', calState.showTodoInput);
  if (calState.showTodoInput) {
    const input = document.getElementById('todo-title-input');
    if (input) input.focus();
  }
}

async function submitTodo() {
  const title = document.getElementById('todo-title-input')?.value;
  if (!title) return;
  const dueDate = document.getElementById('todo-due-input')?.value || null;
  const priority = document.getElementById('todo-priority-input')?.value || 'normal';

  try {
    await cal_invoke('add_todo', {
      title, description: '', sourceThreadId: null, sourceMsgId: null, dueDate, priority,
    });
    document.getElementById('todo-title-input').value = '';
    calState.showTodoInput = false;
    document.getElementById('todo-input-box')?.classList.remove('visible');
    loadTodos();
  } catch(e) { console.error(e); }
}

async function toggleTodoItem(id) {
  try { await cal_invoke('toggle_todo', { id }); loadTodos(); } catch(e) { console.error(e); }
}

async function deleteTodoItem(id) {
  try { await cal_invoke('delete_todo', { id }); loadTodos(); } catch(e) { console.error(e); }
}

// Add todo from mail message
async function addTodoFromMail(threadId, msgId, title) {
  try {
    await cal_invoke('add_todo', {
      title, description: '', sourceThreadId: threadId, sourceMsgId: msgId, dueDate: null, priority: 'normal',
    });
    // Show notification
    showMiniNotification('✅ Todo에 추가됨');
  } catch(e) { console.error(e); }
}

// ╔═══════════════════════════════════════════════╗
// ║   CALENDAR                                    ║
// ╚═══════════════════════════════════════════════╝

async function loadCalendar() {
  const month = `${calState.calYear}-${String(calState.calMonth + 1).padStart(2, '0')}`;
  try {
    calState.events = await cal_invoke('get_calendar_events', { month });
  } catch(e) { console.error(e); calState.events = []; }
  renderCalendar();
}

function renderCalendar() {
  const header = document.getElementById('cal-month-year');
  if (header) header.textContent = `${calState.calYear}년 ${MONTH_NAMES[calState.calMonth]}`;

  const grid = document.getElementById('cal-grid');
  if (!grid) return;

  // Day headers
  let html = DAY_NAMES.map(d => `<div class="cal-day-header">${d}</div>`).join('');

  // Calculate days
  const firstDay = new Date(calState.calYear, calState.calMonth, 1);
  const lastDay = new Date(calState.calYear, calState.calMonth + 1, 0);
  const startDay = firstDay.getDay();
  const totalDays = lastDay.getDate();
  const today = new Date();

  // Previous month padding
  const prevMonthLast = new Date(calState.calYear, calState.calMonth, 0).getDate();
  for (let i = startDay - 1; i >= 0; i--) {
    html += `<div class="cal-day other-month">${prevMonthLast - i}</div>`;
  }

  // Current month days
  const eventDates = new Set(calState.events.map(e => e.event_date));
  for (let d = 1; d <= totalDays; d++) {
    const dateStr = `${calState.calYear}-${String(calState.calMonth + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
    const isToday = today.getFullYear() === calState.calYear && today.getMonth() === calState.calMonth && today.getDate() === d;
    const isSelected = calState.selectedDate === dateStr;
    const hasEvent = eventDates.has(dateStr);

    let classes = 'cal-day';
    if (isToday) classes += ' today';
    if (isSelected) classes += ' selected';
    if (hasEvent) classes += ' has-event';
    classes += ' has-mail'; // All days potentially have mail

    html += `<div class="${classes}" onclick="selectCalDate('${dateStr}')">${d}</div>`;
  }

  // Next month padding
  const endDay = lastDay.getDay();
  for (let i = 1; i <= 6 - endDay; i++) {
    html += `<div class="cal-day other-month">${i}</div>`;
  }

  grid.innerHTML = html;

  // If a date is selected, load its mails
  if (calState.selectedDate) {
    loadDateMails(calState.selectedDate);
  }
}

function calPrev() {
  calState.calMonth--;
  if (calState.calMonth < 0) { calState.calMonth = 11; calState.calYear--; }
  calState.selectedDate = null;
  loadCalendar();
}

function calNext() {
  calState.calMonth++;
  if (calState.calMonth > 11) { calState.calMonth = 0; calState.calYear++; }
  calState.selectedDate = null;
  loadCalendar();
}

function calToday() {
  const now = new Date();
  calState.calYear = now.getFullYear();
  calState.calMonth = now.getMonth();
  calState.selectedDate = now.toISOString().slice(0, 10);
  loadCalendar();
}

async function selectCalDate(dateStr) {
  calState.selectedDate = dateStr;
  renderCalendar();
  loadDateMails(dateStr);
}

async function loadDateMails(dateStr) {
  const container = document.getElementById('date-mail-list');
  if (!container) return;

  try {
    calState.dateMails = await cal_invoke('get_messages_by_date', { date: dateStr });
  } catch(e) { console.error(e); calState.dateMails = []; }

  const headerEl = document.getElementById('date-mail-header');
  if (headerEl) {
    const parts = dateStr.split('-');
    headerEl.innerHTML = `📅 ${parseInt(parts[1])}월 ${parseInt(parts[2])}일 메일 <span class="mail-count">${calState.dateMails.length}</span>`;
  }

  if (calState.dateMails.length === 0) {
    container.innerHTML = `<div class="date-mail-empty"><div class="empty-icon">📭</div>해당 날짜의 메일이 없습니다</div>`;
    return;
  }

  container.innerHTML = calState.dateMails.map(m => {
    const time = m.created_at.slice(11, 16);
    const sender = m.sender_name || m.sender_identity;
    const preview = m.body_summary || '';
    return `<div class="date-mail-item" onclick="jumpToThread('${m.thread_id}')">
      <div class="date-mail-time">${time}</div>
      <div class="date-mail-sender">${sender}</div>
      <div class="date-mail-body">
        <div class="date-mail-preview">${preview.slice(0, 80)}</div>
      </div>
    </div>`;
  }).join('');
}

function jumpToThread(threadId) {
  // Switch to mail view and select the thread
  switchSidebarView('mail');
  // Trigger thread selection in app.js
  if (window.selectThread) window.selectThread(threadId);
}

// ── Calendar Event from Mail ──
function showAddEventModal(threadId, msgId, title) {
  const overlay = document.createElement('div');
  overlay.className = 'mini-modal-overlay';
  overlay.onclick = () => { overlay.remove(); modal.remove(); };
  document.body.appendChild(overlay);

  const modal = document.createElement('div');
  modal.className = 'mini-modal';
  modal.innerHTML = `
    <h3>📅 캘린더에 추가</h3>
    <div class="form-group" style="margin-bottom:12px">
      <label>제목</label>
      <input type="text" id="event-title" value="${title}" style="background:var(--bg-primary);border:1px solid var(--border);border-radius:8px;color:var(--text-primary);padding:10px;width:100%;font-size:13px">
    </div>
    <div class="form-row">
      <div class="form-group">
        <label>날짜</label>
        <input type="date" id="event-date" value="${new Date().toISOString().slice(0,10)}" style="background:var(--bg-primary);border:1px solid var(--border);border-radius:8px;color:var(--text-primary);padding:10px;font-size:13px">
      </div>
      <div class="form-group">
        <label>시간</label>
        <input type="time" id="event-time" value="09:00" style="background:var(--bg-primary);border:1px solid var(--border);border-radius:8px;color:var(--text-primary);padding:10px;font-size:13px">
      </div>
    </div>
    <div class="form-actions" style="margin-top:16px">
      <button class="btn-secondary" onclick="this.closest('.mini-modal').remove();document.querySelector('.mini-modal-overlay')?.remove()">취소</button>
      <button class="btn-primary" onclick="submitCalEvent('${threadId}','${msgId}')">추가</button>
    </div>
  `;
  document.body.appendChild(modal);
}

async function submitCalEvent(threadId, msgId) {
  const title = document.getElementById('event-title').value;
  const date = document.getElementById('event-date').value;
  const time = document.getElementById('event-time').value;
  if (!title || !date) return;

  try {
    await cal_invoke('add_calendar_event', {
      title, description: '', eventDate: date, eventTime: time || null,
      sourceThreadId: threadId || null, sourceMsgId: msgId || null,
    });
    document.querySelector('.mini-modal')?.remove();
    document.querySelector('.mini-modal-overlay')?.remove();
    showMiniNotification('📅 캘린더에 추가됨');
    if (calState.currentView === 'calendar') loadCalendar();
  } catch(e) { console.error(e); }
}

// ── Mini notification ──
function showMiniNotification(text) {
  const el = document.createElement('div');
  el.style.cssText = 'position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:var(--accent);color:#fff;padding:10px 20px;border-radius:10px;font-size:13px;font-weight:600;z-index:2000;animation:fadeIn 0.2s ease';
  el.textContent = text;
  document.body.appendChild(el);
  setTimeout(() => { el.style.opacity = '0'; el.style.transition = 'opacity 0.3s'; setTimeout(() => el.remove(), 300); }, 2000);
}

// ── Helpers ──
function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Make functions globally available
window.switchSidebarView = switchSidebarView;
window.loadTodos = loadTodos;
window.toggleTodoInput = toggleTodoInput;
window.submitTodo = submitTodo;
window.toggleTodoItem = toggleTodoItem;
window.deleteTodoItem = deleteTodoItem;
window.addTodoFromMail = addTodoFromMail;
window.loadCalendar = loadCalendar;
window.calPrev = calPrev;
window.calNext = calNext;
window.calToday = calToday;
window.selectCalDate = selectCalDate;
window.jumpToThread = jumpToThread;
window.showAddEventModal = showAddEventModal;
window.submitCalEvent = submitCalEvent;
window.showMiniNotification = showMiniNotification;
