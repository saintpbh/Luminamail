// ═══════════════════════════════════════════════════
// Group Management Module
// Create/edit/delete groups, manage members,
// send group emails, schedule/recurring emails
// ═══════════════════════════════════════════════════

const { invoke } = window.__TAURI__.core;

let groupState = {
  groups: [],
  selectedGroupId: null,
  members: [],
  scheduledEmails: [],
};

// ── Colors for groups ──
const GROUP_COLORS = [
  '#6366f1', '#8b5cf6', '#ec4899', '#f43f5e',
  '#f97316', '#eab308', '#22c55e', '#14b8a6',
  '#06b6d4', '#3b82f6', '#6d28d9', '#db2777',
];

// ╔═══════════════════════════════════════════════╗
// ║   OPEN / CLOSE GROUP MODAL                    ║
// ╚═══════════════════════════════════════════════╝

async function openGroupManager() {
  const modal = document.getElementById('group-modal');
  if (!modal) return;
  modal.style.display = 'flex';
  await loadGroups();
  renderGroupManager();
}

function closeGroupManager() {
  const modal = document.getElementById('group-modal');
  if (modal) modal.style.display = 'none';
}

async function loadGroups() {
  try {
    groupState.groups = await invoke('get_mail_groups');
    groupState.scheduledEmails = await invoke('get_scheduled_emails_cmd');
  } catch(e) { console.error('Load groups error:', e); }
}

// ╔═══════════════════════════════════════════════╗
// ║   RENDER GROUP MANAGER                        ║
// ╚═══════════════════════════════════════════════╝

function renderGroupManager() {
  const body = document.getElementById('group-modal-body');
  if (!body) return;

  if (groupState.selectedGroupId) {
    renderGroupDetail(body);
    return;
  }

  const groupCards = groupState.groups.map(g => `
    <div class="group-card" onclick="selectGroup('${g.id}')" style="border-left: 4px solid ${g.color}">
      <div class="group-card-header">
        <div class="group-card-color" style="background:${g.color}">${g.name.charAt(0)}</div>
        <div class="group-card-info">
          <h4>${escapeHtml(g.name)}</h4>
          <p>${g.description ? escapeHtml(g.description) : '설명 없음'}</p>
        </div>
        <div class="group-card-meta">
          <span class="group-member-count">👥 ${g.member_count}명</span>
        </div>
      </div>
      <div class="group-card-actions">
        <button class="btn-small" onclick="event.stopPropagation(); sendGroupMail('${g.id}','${escapeHtml(g.name)}')">✉️ 단체메일</button>
        <button class="btn-small" onclick="event.stopPropagation(); showScheduleForGroup('${g.id}','${escapeHtml(g.name)}')">⏰ 예약발송</button>
        <button class="btn-small danger" onclick="event.stopPropagation(); deleteGroup('${g.id}','${escapeHtml(g.name)}')">🗑️</button>
      </div>
    </div>
  `).join('');

  const scheduledCards = groupState.scheduledEmails.map(se => {
    const groupName = groupState.groups.find(g => g.id === se.group_id)?.name || '개별';
    const typeLabel = se.schedule_type === 'once' ? '1회' : se.schedule_type === 'weekly' ? '매주' : se.schedule_type === 'monthly' ? '매월' : se.schedule_type;
    return `
      <div class="scheduled-card ${se.enabled ? '' : 'disabled'}">
        <div class="scheduled-info">
          <span class="scheduled-type-badge">${typeLabel}</span>
          <strong>${escapeHtml(se.subject)}</strong>
          <span class="scheduled-meta">→ ${groupName} | ${se.schedule_time}</span>
        </div>
        <div class="scheduled-actions">
          <button class="btn-small" onclick="toggleScheduled('${se.id}')">${se.enabled ? '⏸️' : '▶️'}</button>
          <button class="btn-small danger" onclick="deleteScheduled('${se.id}')">🗑️</button>
        </div>
      </div>
    `;
  }).join('');

  body.innerHTML = `
    <div class="group-toolbar">
      <button class="btn-primary" onclick="showCreateGroupForm()">➕ 새 그룹 만들기</button>
    </div>

    <div id="create-group-form" style="display:none" class="create-group-form">
      <h4>📋 새 그룹</h4>
      <input id="new-group-name" placeholder="그룹 이름 (예: 교회 청년부)" class="input-field" />
      <input id="new-group-desc" placeholder="설명 (선택)" class="input-field" />
      <div class="color-picker-row">
        ${GROUP_COLORS.map(c => `<div class="color-dot ${c === '#6366f1' ? 'selected' : ''}" style="background:${c}" data-color="${c}" onclick="selectGroupColor(this)"></div>`).join('')}
      </div>
      <div class="form-actions">
        <button class="btn-primary" onclick="createGroup()">만들기</button>
        <button class="btn-small" onclick="document.getElementById('create-group-form').style.display='none'">취소</button>
      </div>
    </div>

    <div class="group-section-title">📂 내 그룹 (${groupState.groups.length})</div>
    ${groupState.groups.length ? groupCards : '<div class="empty-state">아직 그룹이 없습니다. 위 버튼으로 새 그룹을 만들어보세요!</div>'}

    ${groupState.scheduledEmails.length ? `
      <div class="group-section-title" style="margin-top:20px">⏰ 예약/정기 메일 (${groupState.scheduledEmails.length})</div>
      ${scheduledCards}
    ` : ''}
  `;
}

// ╔═══════════════════════════════════════════════╗
// ║   GROUP DETAIL VIEW                           ║
// ╚═══════════════════════════════════════════════╝

async function renderGroupDetail(body) {
  const group = groupState.groups.find(g => g.id === groupState.selectedGroupId);
  if (!group) { groupState.selectedGroupId = null; renderGroupManager(); return; }

  try {
    groupState.members = await invoke('get_group_members_cmd', { groupId: group.id });
  } catch(e) { groupState.members = []; }

  const memberRows = groupState.members.map(m => `
    <div class="member-row">
      <div class="member-avatar" style="background:${group.color}">${(m.display_name || m.email).charAt(0).toUpperCase()}</div>
      <div class="member-info">
        <span class="member-name">${escapeHtml(m.display_name || m.email)}</span>
        <span class="member-email">${escapeHtml(m.email)}</span>
      </div>
      <button class="btn-small danger" onclick="removeMember('${m.id}')">✕</button>
    </div>
  `).join('');

  body.innerHTML = `
    <div class="group-detail-header">
      <button class="btn-back" onclick="backToGroupList()">← 목록</button>
      <div class="group-detail-title" style="border-left: 4px solid ${group.color}; padding-left: 12px">
        <h3>${escapeHtml(group.name)}</h3>
        <p>${group.description ? escapeHtml(group.description) : ''}</p>
      </div>
      <div class="group-detail-actions">
        <button class="btn-primary" onclick="sendGroupMail('${group.id}','${escapeHtml(group.name)}')">✉️ 단체메일</button>
        <button class="btn-small" onclick="showScheduleForGroup('${group.id}','${escapeHtml(group.name)}')">⏰ 예약</button>
      </div>
    </div>

    <div class="member-add-row">
      <input id="add-member-email" placeholder="이메일 주소" class="input-field" style="flex:1" />
      <input id="add-member-name" placeholder="이름 (선택)" class="input-field" style="width:120px" />
      <button class="btn-primary" onclick="addMember('${group.id}')">추가</button>
    </div>

    <div class="group-section-title">👥 멤버 (${groupState.members.length}명)</div>
    ${groupState.members.length ? memberRows : '<div class="empty-state">아직 멤버가 없습니다. 위에서 이메일을 입력해 추가하세요.</div>'}
  `;
}

// ╔═══════════════════════════════════════════════╗
// ║   GROUP ACTIONS                               ║
// ╚═══════════════════════════════════════════════╝

function showCreateGroupForm() {
  document.getElementById('create-group-form').style.display = 'block';
  document.getElementById('new-group-name').focus();
}

let selectedColor = '#6366f1';
function selectGroupColor(el) {
  document.querySelectorAll('.color-dot').forEach(d => d.classList.remove('selected'));
  el.classList.add('selected');
  selectedColor = el.dataset.color;
}

async function createGroup() {
  const name = document.getElementById('new-group-name').value.trim();
  const desc = document.getElementById('new-group-desc').value.trim() || null;
  if (!name) { alert('그룹 이름을 입력하세요'); return; }
  try {
    await invoke('create_mail_group', { name, description: desc, color: selectedColor });
    await loadGroups();
    renderGroupManager();
  } catch(e) { alert('생성 실패: ' + e); }
}

async function deleteGroup(id, name) {
  if (!confirm(`"${name}" 그룹을 삭제하시겠습니까? 모든 멤버와 예약 메일이 삭제됩니다.`)) return;
  try {
    await invoke('delete_mail_group', { id });
    groupState.selectedGroupId = null;
    await loadGroups();
    renderGroupManager();
  } catch(e) { alert('삭제 실패: ' + e); }
}

function selectGroup(id) {
  groupState.selectedGroupId = id;
  renderGroupManager();
}

function backToGroupList() {
  groupState.selectedGroupId = null;
  renderGroupManager();
}

async function addMember(groupId) {
  const email = document.getElementById('add-member-email').value.trim();
  const name = document.getElementById('add-member-name').value.trim() || null;
  if (!email) { alert('이메일을 입력하세요'); return; }
  try {
    await invoke('add_group_member_cmd', { groupId, email, displayName: name });
    await loadGroups();
    const body = document.getElementById('group-modal-body');
    renderGroupDetail(body);
  } catch(e) { alert('추가 실패: ' + e); }
}

async function removeMember(memberId) {
  if (!confirm('이 멤버를 삭제하시겠습니까?')) return;
  try {
    await invoke('remove_group_member_cmd', { memberId });
    await loadGroups();
    const body = document.getElementById('group-modal-body');
    renderGroupDetail(body);
  } catch(e) { alert('삭제 실패: ' + e); }
}

// ╔═══════════════════════════════════════════════╗
// ║   GROUP MAIL COMPOSE                          ║
// ╚═══════════════════════════════════════════════╝

async function sendGroupMail(groupId, groupName) {
  let members = [];
  try { members = await invoke('get_group_members_cmd', { groupId }); } catch(e) {}
  if (!members.length) { alert('이 그룹에 멤버가 없습니다. 먼저 멤버를 추가하세요.'); return; }

  const emailList = members.map(m => m.email).join(', ');
  const body = document.getElementById('group-modal-body');
  body.innerHTML = `
    <div class="group-detail-header">
      <button class="btn-back" onclick="backToGroupList()">← 목록</button>
      <h3>✉️ ${escapeHtml(groupName)} 단체메일</h3>
    </div>
    <div class="compose-form">
      <div class="compose-field">
        <label>받는 사람 (${members.length}명)</label>
        <div class="recipient-tags">${members.map(m => `<span class="recipient-tag">${escapeHtml(m.display_name || m.email)}</span>`).join('')}</div>
      </div>
      <div class="compose-field">
        <label>제목</label>
        <input id="group-mail-subject" class="input-field" placeholder="메일 제목" />
      </div>
      <div class="compose-field">
        <label>본문</label>
        <textarea id="group-mail-body" class="input-field compose-textarea" placeholder="메일 내용을 입력하세요..." rows="8"></textarea>
      </div>
      <div class="compose-actions">
        <button class="btn-primary" onclick="doSendGroupMail('${groupId}')">📤 지금 발송</button>
        <button class="btn-small" onclick="showScheduleForGroup('${groupId}','${escapeHtml(groupName)}')">⏰ 예약 발송</button>
        <button class="btn-small" onclick="backToGroupList()">취소</button>
      </div>
    </div>
  `;
}

async function doSendGroupMail(groupId) {
  const subject = document.getElementById('group-mail-subject').value.trim();
  const body = document.getElementById('group-mail-body').value.trim();
  if (!subject || !body) { alert('제목과 본문을 입력하세요'); return; }

  let members = [];
  try { members = await invoke('get_group_members_cmd', { groupId }); } catch(e) {}
  const toEmails = members.map(m => m.email).join(', ');

  // For now, create a scheduled email with immediate execution
  try {
    const now = new Date().toISOString().slice(0, 19);
    await invoke('create_scheduled_email_cmd', {
      groupId, toEmails, subject, body, scheduleType: 'once', scheduleTime: now, recurrenceRule: null
    });
    alert(`📤 ${members.length}명에게 발송 예약됨: ${subject}`);
    await loadGroups();
    renderGroupManager();
  } catch(e) { alert('발송 실패: ' + e); }
}

// ╔═══════════════════════════════════════════════╗
// ║   SCHEDULED / RECURRING EMAILS                ║
// ╚═══════════════════════════════════════════════╝

function showScheduleForGroup(groupId, groupName) {
  const body = document.getElementById('group-modal-body');
  body.innerHTML = `
    <div class="group-detail-header">
      <button class="btn-back" onclick="backToGroupList()">← 목록</button>
      <h3>⏰ ${escapeHtml(groupName)} 예약/정기 메일</h3>
    </div>
    <div class="compose-form">
      <div class="compose-field">
        <label>제목</label>
        <input id="sched-subject" class="input-field" placeholder="예: [주간 소식] 이번 주 안내사항" />
      </div>
      <div class="compose-field">
        <label>본문</label>
        <textarea id="sched-body" class="input-field compose-textarea" placeholder="메일 내용..." rows="6"></textarea>
      </div>
      <div class="compose-field">
        <label>발송 유형</label>
        <div class="schedule-type-grid">
          <label class="schedule-type-option selected" onclick="selectScheduleType(this, 'once')">
            <input type="radio" name="sched-type" value="once" checked /> 📅 1회 예약
          </label>
          <label class="schedule-type-option" onclick="selectScheduleType(this, 'weekly')">
            <input type="radio" name="sched-type" value="weekly" /> 🔄 매주 반복
          </label>
          <label class="schedule-type-option" onclick="selectScheduleType(this, 'monthly')">
            <input type="radio" name="sched-type" value="monthly" /> 📆 매월 반복
          </label>
        </div>
      </div>
      <div class="compose-field">
        <label>발송 날짜/시간</label>
        <input id="sched-time" type="datetime-local" class="input-field" />
      </div>
      <div id="recurrence-options" style="display:none" class="compose-field">
        <label>반복 규칙</label>
        <select id="sched-rule" class="input-field">
          <option value="every_week">매주 같은 요일</option>
          <option value="biweekly">격주</option>
          <option value="first_of_month">매월 첫째 주</option>
          <option value="last_of_month">매월 마지막 주</option>
        </select>
      </div>
      <div class="compose-actions">
        <button class="btn-primary" onclick="createScheduledMail('${groupId}')">⏰ 예약 등록</button>
        <button class="btn-small" onclick="backToGroupList()">취소</button>
      </div>
    </div>
  `;
  // Set default datetime to tomorrow 9:00
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(9, 0, 0, 0);
  document.getElementById('sched-time').value = tomorrow.toISOString().slice(0, 16);
}

let currentScheduleType = 'once';
function selectScheduleType(el, type) {
  document.querySelectorAll('.schedule-type-option').forEach(o => o.classList.remove('selected'));
  el.classList.add('selected');
  currentScheduleType = type;
  document.getElementById('recurrence-options').style.display = type === 'once' ? 'none' : 'block';
}

async function createScheduledMail(groupId) {
  const subject = document.getElementById('sched-subject').value.trim();
  const body = document.getElementById('sched-body').value.trim();
  const scheduleTime = document.getElementById('sched-time').value;
  if (!subject || !body || !scheduleTime) { alert('모든 필드를 입력하세요'); return; }

  let members = [];
  try { members = await invoke('get_group_members_cmd', { groupId }); } catch(e) {}
  const toEmails = members.map(m => m.email).join(', ');
  const recurrenceRule = currentScheduleType !== 'once' ? document.getElementById('sched-rule')?.value || null : null;

  try {
    await invoke('create_scheduled_email_cmd', {
      groupId, toEmails, subject, body,
      scheduleType: currentScheduleType, scheduleTime,
      recurrenceRule
    });
    alert(`⏰ 예약 등록 완료! (${currentScheduleType === 'once' ? '1회' : currentScheduleType})`);
    await loadGroups();
    renderGroupManager();
  } catch(e) { alert('예약 실패: ' + e); }
}

async function toggleScheduled(id) {
  try {
    await invoke('toggle_scheduled_email_cmd', { id });
    await loadGroups();
    renderGroupManager();
  } catch(e) { alert('변경 실패: ' + e); }
}

async function deleteScheduled(id) {
  if (!confirm('이 예약 메일을 삭제하시겠습니까?')) return;
  try {
    await invoke('delete_scheduled_email_cmd', { id });
    await loadGroups();
    renderGroupManager();
  } catch(e) { alert('삭제 실패: ' + e); }
}

// ── Helpers ──
function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ── Make functions globally available ──
window.openGroupManager = openGroupManager;
window.closeGroupManager = closeGroupManager;
window.selectGroup = selectGroup;
window.backToGroupList = backToGroupList;
window.showCreateGroupForm = showCreateGroupForm;
window.selectGroupColor = selectGroupColor;
window.createGroup = createGroup;
window.deleteGroup = deleteGroup;
window.addMember = addMember;
window.removeMember = removeMember;
window.sendGroupMail = sendGroupMail;
window.doSendGroupMail = doSendGroupMail;
window.showScheduleForGroup = showScheduleForGroup;
window.selectScheduleType = selectScheduleType;
window.createScheduledMail = createScheduledMail;
window.toggleScheduled = toggleScheduled;
window.deleteScheduled = deleteScheduled;
