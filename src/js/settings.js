// ═══════════════════════════════════════════════════════════
// Lumina Mail - Settings & Account Management
// ═══════════════════════════════════════════════════════════

const { invoke } = window.__TAURI__.core;

// Provider defaults
const PROVIDERS = {
  gmail:   { name: 'Gmail',   icon: '📧', imap: 'imap.gmail.com',          imapPort: 993, smtp: 'smtp.gmail.com',          smtpPort: 587 },
  naver:   { name: 'Naver',   icon: '📗', imap: 'imap.naver.com',          imapPort: 993, smtp: 'smtp.naver.com',          smtpPort: 587 },
  daum:    { name: 'Daum',    icon: '📙', imap: 'imap.daum.net',           imapPort: 993, smtp: 'smtp.daum.net',           smtpPort: 465 },
  outlook: { name: 'Outlook', icon: '📘', imap: 'outlook.office365.com',   imapPort: 993, smtp: 'smtp.office365.com',      smtpPort: 587 },
};

let settingsState = {
  activeTab: 'accounts',
  telegramCode: null,
  telegramPolling: false,
  telegramLink: null,
  accounts: [],
  addingProvider: null,
  editingAccountId: null,
  cloudProviders: [],
};

// ── Open / Close ──
function openSettings() {
  document.getElementById('settings-overlay').style.display = 'flex';
  loadSettingsData();
}

function closeSettings() {
  document.getElementById('settings-overlay').style.display = 'none';
  settingsState.telegramPolling = false;
}

function switchSettingsTab(tab) {
  settingsState.activeTab = tab;
  document.querySelectorAll('.settings-tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
  renderSettingsBody();
}

async function loadSettingsData() {
  try {
    settingsState.accounts = await invoke('get_email_accounts');
    settingsState.telegramLink = await invoke('telegram_get_status');
    try {
      settingsState.cloudProviders = await invoke('cloud_get_status');
    } catch(e) { settingsState.cloudProviders = []; }
  } catch(e) { console.error('Settings load error:', e); }
  renderSettingsBody();
}

async function renderSettingsBody() {
  const body = document.getElementById('settings-body');
  if (settingsState.activeTab === 'accounts') {
    body.innerHTML = renderAccountsTab();
  } else if (settingsState.activeTab === 'telegram') {
    body.innerHTML = renderTelegramTab();
  } else if (settingsState.activeTab === 'cloud') {
    body.innerHTML = renderCloudTab();
  } else if (settingsState.activeTab === 'ai') {
    body.innerHTML = await renderAITab();
  } else if (settingsState.activeTab === 'signatures') {
    body.innerHTML = await renderSignaturesTab();
  }
  bindSettingsEvents();
}

async function renderAITab() {
  let currentKey = '';
  let currentModel = 'gemini-2.0-flash';
  try { 
    currentKey = await invoke('get_app_setting', { key: 'gemini_api_key' });
    currentModel = await invoke('get_app_setting', { key: 'gemini_model' }) || 'gemini-2.0-flash';
  } catch(e) {}
  
  if (!currentKey) {
    try { currentKey = await invoke('get_gemini_api_key'); } catch(e) {}
  }
  
  let modelOptionsHtml = `
    <option value="gemini-2.5-flash" ${currentModel === 'gemini-2.5-flash' ? 'selected' : ''}>Gemini 2.5 Flash (빠르고 경제적)</option>
    <option value="gemini-2.5-pro" ${currentModel === 'gemini-2.5-pro' ? 'selected' : ''}>Gemini 2.5 Pro (깊이있는 분석)</option>
    <option value="gemini-2.0-flash" ${currentModel === 'gemini-2.0-flash' ? 'selected' : ''}>Gemini 2.0 Flash</option>
  `;

  if (currentKey) {
    try {
      const models = await invoke('get_available_models_cmd');
      if (models && models.length > 0) {
        // Filter out non-text or experimental/unwanted models
        const filteredModels = models.filter(m => {
          const name = (m.displayName || m.name).toLowerCase();
          const isGemini = name.includes('gemini');
          const isUnwanted = name.includes('tts') || name.includes('gemma') || name.includes('nano') || 
                             name.includes('banana') || name.includes('vision') || name.includes('001') || 
                             name.includes('latest') || name.includes('preview');
          return isGemini && !isUnwanted;
        });
        
        if (filteredModels.length > 0) {
          modelOptionsHtml = filteredModels.map(m => {
            const id = m.name.replace('models/', '');
            const name = m.displayName || id;
            return `<option value="${id}" ${currentModel === id ? 'selected' : ''}>${name}</option>`;
          }).join('');
        }
      }
    } catch(e) { console.error('Model fetch failed', e); }
  }
  
  const masked = currentKey ? '●'.repeat(Math.max(0, currentKey.length - 4)) + currentKey.slice(-4) : '';
  
  return `
    <div style="padding:20px;">
      <h3 style="color:var(--text-primary);margin-bottom:16px;">🤖 AI 연결 설정</h3>
      <p style="color:var(--text-secondary);font-size:13px;margin-bottom:16px;line-height:1.5;">
        Google Gemini API를 연결하면 메일 요약, 스마트 태그, 번역 기능이 자동으로 작동합니다.<br>
        API 키 발급: <a href="https://aistudio.google.com/app/apikey" target="_blank" style="color:var(--accent);text-decoration:none;">Google AI Studio API Key ↗</a>
      </p>
      
      <div style="margin-bottom:12px;">
        <label style="font-size:13px;color:var(--text-secondary);display:block;margin-bottom:4px;">Gemini API 키</label>
        <div style="display:flex;gap:8px;">
          <input id="ai-api-key" type="password" placeholder="${masked || 'AIzaSy...'}" 
            style="flex:1;padding:10px 14px;background:var(--bg-tertiary);border:1px solid var(--border-subtle);border-radius:8px;color:var(--text-primary);font-size:14px;outline:none;">
        </div>
      </div>
      
      <div style="margin-bottom:16px;">
        <label style="font-size:13px;color:var(--text-secondary);display:block;margin-bottom:4px;">사용 모델 (AI 플랜)</label>
        <div style="display:flex;gap:8px;">
          <select id="ai-model" style="flex:1;padding:10px 14px;background:var(--bg-tertiary);border:1px solid var(--border-subtle);border-radius:8px;color:var(--text-primary);font-size:14px;outline:none;cursor:pointer;">
            ${modelOptionsHtml}
          </select>
          <button onclick="saveAISettingsBtn()" style="padding:8px 16px;background:var(--accent);color:white;border:none;border-radius:8px;font-weight:600;cursor:pointer;">설정 저장</button>
        </div>
      </div>
      
      <div style="display:flex;gap:8px;margin-bottom:16px;">
        <button onclick="testAIConnection()" style="padding:8px 16px;background:linear-gradient(135deg,#667eea,#764ba2);color:white;border:none;border-radius:8px;cursor:pointer;font-weight:600;">🔌 연결 테스트</button>
        <button onclick="syncContactsFromMail()" style="padding:8px 16px;background:var(--bg-tertiary);border:1px solid var(--border-subtle);color:var(--text-primary);border-radius:8px;cursor:pointer;">📇 주소록 동기화</button>
      </div>
      
      <div id="ai-test-result" style="padding:12px;border-radius:8px;font-size:13px;display:none;"></div>

      
      <div style="margin-top:20px;padding:16px;background:var(--bg-tertiary);border-radius:8px;">
        <h4 style="color:var(--text-primary);margin-bottom:8px;">AI가 하는 일</h4>
        <ul style="color:var(--text-secondary);font-size:13px;line-height:1.8;margin:0;padding-left:18px;">
          <li><strong>📝 요약</strong> — 메일 핵심 내용 1~2문장 요약</li>
          <li><strong>🏷️ 태그</strong> — 이모지 + 분류 태그 자동 생성</li>
          <li><strong>🌐 번역</strong> — 한↔영 자동 번역 (토글)</li>
          <li><strong>⭐ 중요</strong> — 공문, 계약, 결제 등 중요 메일 자동 분류</li>
          <li><strong>📋 후속 조치</strong> — 회신/처리가 필요한 메일 표시</li>
        </ul>
      </div>
    </div>
  `;
}

async function renderSignaturesTab() {
  try {
    const sigs = await invoke('get_signatures_cmd');
    return `
      <div style="padding:20px;">
        <h3 style="color:var(--text-primary);margin-bottom:16px;">✍️ 서명 관리</h3>
        <div id="sig-list" style="margin-bottom:20px;">
          ${sigs.map(s => `
            <div style="background:var(--bg-tertiary);border:1px solid var(--border-subtle);border-radius:8px;padding:12px;margin-bottom:8px;">
              <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;">
                <strong style="color:var(--text-primary);">${s.name} ${s.is_default ? '⭐' : ''}</strong>
                <button onclick="deleteSignature('${s.id}')" style="background:none;border:none;color:var(--danger);cursor:pointer;">삭제</button>
              </div>
              <div style="font-size:13px;color:var(--text-secondary);">${s.body_html || '<em>내용 없음</em>'}</div>
            </div>
          `).join('') || '<p style="color:var(--text-tertiary);">서명이 없습니다.</p>'}
        </div>
        <div style="border-top:1px solid var(--border-subtle);padding-top:16px;">
          <input id="sig-name" placeholder="서명 이름" style="width:100%;padding:8px 12px;background:var(--bg-tertiary);border:1px solid var(--border-subtle);border-radius:6px;color:var(--text-primary);margin-bottom:8px;">
          <textarea id="sig-body" placeholder="서명 내용 (HTML 지원)" rows="4" style="width:100%;padding:8px 12px;background:var(--bg-tertiary);border:1px solid var(--border-subtle);border-radius:6px;color:var(--text-primary);margin-bottom:8px;resize:vertical;"></textarea>
          <div style="display:flex;gap:8px;align-items:center;">
            <label style="color:var(--text-secondary);font-size:13px;"><input type="checkbox" id="sig-default"> 기본 서명</label>
            <button onclick="saveNewSignature()" style="padding:8px 16px;background:var(--accent);color:white;border:none;border-radius:6px;cursor:pointer;">저장</button>
          </div>
        </div>
      </div>
    `;
  } catch (e) {
    return '<p style="padding:20px;color:var(--text-tertiary);">서명을 불러올 수 없습니다.</p>';
  }
}


// ╔═══════════════════════════════════════════════╗
// ║   ACCOUNTS TAB                                ║
// ╚═══════════════════════════════════════════════╝

function renderAccountsTab() {
  let html = '';

  // Connected accounts
  if (settingsState.accounts.length > 0) {
    html += '<div class="settings-section"><h3>🔗 연결된 계정</h3>';
    html += '<button class="btn-primary" onclick="syncAllAccounts()" style="margin-bottom:12px;padding:8px 16px;font-size:13px">🔄 전체 동기화</button>';
    html += '<div id="sync-all-result" style="margin-bottom:12px;font-size:13px;color:var(--text-secondary)"></div>';
    settingsState.accounts.forEach(acc => {
      const p = PROVIDERS[acc.provider] || { name: acc.provider, icon: '📧' };
      html += `
        <div class="account-card" data-id="${acc.id}">
          <div class="provider-icon ${acc.provider}">${p.icon}</div>
          <div class="account-meta">
            <div class="account-email">${acc.email}</div>
            <div class="account-provider">${p.name} · ${acc.display_name || ''}</div>
            <div id="acc-status-${acc.id}" class="account-sync-status"></div>
          </div>
          <div class="account-actions-col">
            <div class="sync-toggle">
              <select class="sync-select" data-account-id="${acc.id}" onchange="updateSyncMode(this)">
                <option value="readonly" ${acc.sync_mode === 'readonly' ? 'selected' : ''}>👁 읽기전용</option>
                <option value="bidirectional" ${acc.sync_mode === 'bidirectional' ? 'selected' : ''}>🔄 양방향</option>
              </select>
            </div>
            <div style="display:flex;gap:4px;margin-top:4px">
              <button class="btn-small" onclick="testEmailConnection('${acc.id}')" title="연결 테스트">🔌 테스트</button>
              <button class="btn-small btn-sync" onclick="syncEmailAccount('${acc.id}')" title="메일 가져오기">🔄 동기화</button>
              <button class="btn-small" onclick="editAccount('${acc.id}')" title="계정 수정">✏️ 수정</button>
              <button class="account-delete" onclick="deleteAccount('${acc.id}')" title="삭제">🗑</button>
            </div>
          </div>
        </div>`;
    });
    html += '</div>';
  }

  // Add new account
  if (settingsState.addingProvider) {
    html += renderAccountForm();
  } else {
    html += `<div class="settings-section"><h3>➕ 메일 계정 추가</h3>
      <div class="provider-grid">
        <div class="provider-card" onclick="startAddAccount('gmail')">
          <div class="provider-icon gmail">G</div>
          <div class="provider-info"><h4>Gmail</h4><p>Google 메일 (IMAP)</p></div>
        </div>
        <div class="provider-card" onclick="startAddAccount('naver')">
          <div class="provider-icon naver">N</div>
          <div class="provider-info"><h4>Naver 메일</h4><p>네이버 메일 (IMAP)</p></div>
        </div>
        <div class="provider-card" onclick="startAddAccount('daum')">
          <div class="provider-icon daum">D</div>
          <div class="provider-info"><h4>Daum 메일</h4><p>다음/카카오 메일 (IMAP)</p></div>
        </div>
        <div class="provider-card" onclick="startAddAccount('outlook')">
          <div class="provider-icon outlook">O</div>
          <div class="provider-info"><h4>Outlook</h4><p>Microsoft 365 (IMAP)</p></div>
        </div>
      </div>
    </div>`;
  }

  // Sync mode explanation
  html += `<div class="settings-section"><h3>ℹ️ 동기화 모드 설명</h3>
    <div class="sync-mode-box">
      <div class="sync-option">
        <div class="sync-radio"></div>
        <div class="sync-option-text">
          <h4>👁 읽기전용</h4>
          <p>메일을 Lumina Mail에서 확인만 가능합니다. 앱에서 삭제해도 원본 메일은 유지됩니다.</p>
        </div>
      </div>
      <div class="sync-option">
        <div class="sync-radio"></div>
        <div class="sync-option-text">
          <h4>🔄 양방향 동기화</h4>
          <p>앱에서 삭제하면 메일 서버에서도 함께 삭제됩니다. 언제든 설정에서 변경 가능합니다.</p>
        </div>
      </div>
    </div>
  </div>`;

  return html;
}

function startAddAccount(provider) {
  settingsState.addingProvider = provider;
  renderSettingsBody();
}

function renderAccountForm() {
  const providerKey = (settingsState.addingProvider || '').toLowerCase();
  const p = PROVIDERS[providerKey] || { name: settingsState.addingProvider || 'Email', icon: '📧', imap: '', imapPort: 993, smtp: '', smtpPort: 465 };
  const title = settingsState.editingAccountId ? `${p.icon} ${p.name} 연동 수정` : `${p.icon} ${p.name} 계정 추가`;
  const submitText = settingsState.editingAccountId ? `저장하기` : `연결하기`;
  const passPlaceholder = settingsState.editingAccountId ? `(변경 시에만 입력)` : `앱 비밀번호를 입력하세요`;

  return `<div class="settings-section"><h3>${title}</h3>
    <div class="account-form">
      <div class="form-row">
        <div class="form-group">
          <label>이메일 주소</label>
          <input type="email" id="acc-email" placeholder="example@${settingsState.addingProvider === 'gmail' ? 'gmail.com' : settingsState.addingProvider + '.com'}">
        </div>
        <div class="form-group">
          <label>표시 이름</label>
          <input type="text" id="acc-name" placeholder="나의 ${p.name}">
        </div>
      </div>
      <div class="form-row">
        <div class="form-group" style="flex:1;">
          <label>비밀번호 / 앱 비밀번호</label>
          <input type="password" id="acc-password" placeholder="${passPlaceholder}">
          ${providerKey === 'gmail' ? `
            <details style="margin-top:8px; font-size:11px; color:var(--text-secondary); background: var(--bg-surface-hover); padding: 8px; border-radius: 6px;">
              <summary style="cursor:pointer; font-weight:bold; color:var(--primary-color);">🤔 구글 앱 비밀번호 발급 방법</summary>
              <ol style="margin-top:8px; padding-left:16px; line-height:1.5;">
                <li><a href="https://myaccount.google.com/security" target="_blank" style="color:var(--primary-color); text-decoration:underline;">구글 보안 설정(클릭)</a>에 접속</li>
                <li>화면 맨 위 <b>'Google 계정 검색'</b>(돋보기) 클릭</li>
                <li><b>'앱 비밀번호'</b>라고 검색 후 나오는 메뉴 클릭 <br><span style="font-size:10px; color:#888;">(※ 2단계 인증이 켜져 있어야 검색됩니다)</span></li>
                <li>앱 이름에 'Lumina' 입력 후 만들기 클릭</li>
                <li>발급된 <b>16자리 영문자</b>를 위 칸에 복사해 넣으세요</li>
              </ol>
            </details>
          ` : ''}
        </div>
      </div>
      <div class="form-row">
        <div class="form-group">
          <label>IMAP 서버</label>
          <input type="text" id="acc-imap" value="${p.imap}">
        </div>
        <div class="form-group" style="max-width:80px">
          <label>포트</label>
          <input type="number" id="acc-imap-port" value="${p.imapPort}">
        </div>
      </div>
      <div class="form-row">
        <div class="form-group">
          <label>SMTP 서버</label>
          <input type="text" id="acc-smtp" value="${p.smtp}">
        </div>
        <div class="form-group" style="max-width:80px">
          <label>포트</label>
          <input type="number" id="acc-smtp-port" value="${p.smtpPort}">
        </div>
      </div>
      <div class="form-row">
        <div class="form-group">
          <label>동기화 모드</label>
          <select id="acc-sync">
            <option value="readonly">👁 읽기전용 — 보기만 가능</option>
            <option value="bidirectional">🔄 양방향 동기화 — 삭제 시 서버에서도 삭제</option>
          </select>
        </div>
      </div>
      <div class="form-actions">
        <button class="btn-secondary" onclick="cancelAddAccount()">취소</button>
        <button class="btn-primary" onclick="submitAddAccount()">${submitText}</button>
      </div>
    </div>
  </div>`;
}

function cancelAddAccount() {
  settingsState.addingProvider = null;
  settingsState.editingAccountId = null;
  renderSettingsBody();
}

function editAccount(id) {
  const acc = settingsState.accounts.find(a => a.id === id);
  if(!acc) return;
  settingsState.editingAccountId = id;
  settingsState.addingProvider = acc.provider;
  renderSettingsBody();

  setTimeout(() => {
    document.getElementById('acc-email').value = acc.email;
    document.getElementById('acc-email').disabled = true;
    document.getElementById('acc-name').value = acc.display_name;
    document.getElementById('acc-imap').value = acc.imap_host;
    document.getElementById('acc-imap-port').value = acc.imap_port;
    document.getElementById('acc-smtp').value = acc.smtp_host;
    document.getElementById('acc-smtp-port').value = acc.smtp_port;
    document.getElementById('acc-sync').value = acc.sync_mode;
    document.getElementById('acc-sync').disabled = true;
  }, 50);
}

async function submitAddAccount() {
  const email = document.getElementById('acc-email').value;
  const name = document.getElementById('acc-name').value || email;
  let password = document.getElementById('acc-password').value || '';
  password = password.replace(/\s+/g, '');
  
  const imap = document.getElementById('acc-imap').value;
  const imapPort = parseInt(document.getElementById('acc-imap-port').value);
  const smtp = document.getElementById('acc-smtp').value;
  const smtpPort = parseInt(document.getElementById('acc-smtp-port').value);
  const sync = document.getElementById('acc-sync').value;

  if (settingsState.editingAccountId) {
    if (!password) {
      const acc = settingsState.accounts.find(a => a.id === settingsState.editingAccountId);
      if(acc) password = acc.password_encrypted;
    }
  } else {
    if (!email || !password) { alert('이메일과 비밀번호를 입력하세요.'); return; }
  }

  try {
    if (settingsState.editingAccountId) {
      await invoke('update_email_account_details', {
        id: settingsState.editingAccountId,
        displayName: name,
        imapHost: imap, imapPort,
        smtpHost: smtp, smtpPort,
        password: password
      });
    } else {
      await invoke('add_email_account', {
        provider: settingsState.addingProvider,
        email, displayName: name, imapHost: imap, imapPort, smtpHost: smtp, smtpPort,
        username: email, password, syncMode: sync,
      });
    }
    settingsState.addingProvider = null;
    settingsState.editingAccountId = null;
    await loadSettingsData();
  } catch(e) { alert('저장 실패: ' + e); }
}

async function updateSyncMode(el) {
  const id = el.dataset.accountId;
  const mode = el.value;
  try {
    await invoke('update_email_sync_mode', { id, syncMode: mode });
  } catch(e) { alert('설정 변경 실패: ' + e); }
}

async function deleteAccount(id) {
  if (!confirm('이 메일 계정을 삭제하시겠습니까?')) return;
  try {
    await invoke('delete_email_account', { id });
    await loadSettingsData();
  } catch(e) { alert('삭제 실패: ' + e); }
}

// ╔═══════════════════════════════════════════════╗
// ║   TELEGRAM TAB                                ║
// ╚═══════════════════════════════════════════════╝

function renderTelegramTab() {
  if (settingsState.telegramLink) {
    return renderTelegramConnected();
  }
  if (settingsState.telegramCode) {
    return renderTelegramLinking();
  }
  return renderTelegramStart();
}

function renderTelegramStart() {
  return `<div class="telegram-section">
    <div style="font-size:48px;margin-bottom:16px">✈️</div>
    <h3 style="margin-bottom:8px">Telegram 연결</h3>
    <p style="font-size:13px;color:var(--text-muted);margin-bottom:20px">
      나의 Telegram과 Lumina Mail을 연결하여<br>중요한 메일 알림을 실시간으로 받으세요.
    </p>
    <a href="https://t.me/Luminamail_bot" target="_blank" class="telegram-bot-link">
      ✈️ @Luminamail_bot 열기
    </a>
    <br><br>
    <button class="btn-primary" onclick="startTelegramLinking()" style="padding:14px 32px;font-size:15px">
      🔗 연결 코드 생성
    </button>
    <div style="margin-top:20px">
      <p style="font-size:12px;color:var(--text-muted)">연결 방법:</p>
      <p style="font-size:12px;color:var(--text-muted)">1. 위 버튼으로 봇을 열거나 Telegram에서 @Luminamail_bot 검색</p>
      <p style="font-size:12px;color:var(--text-muted)">2. "연결 코드 생성"을 클릭합니다</p>
      <p style="font-size:12px;color:var(--text-muted)">3. 표시된 6자리 코드를 봇에게 전송합니다</p>
    </div>
  </div>`;
}

function renderTelegramLinking() {
  return `<div class="telegram-section">
    <div style="font-size:48px;margin-bottom:12px">🔗</div>
    <h3 style="margin-bottom:8px">연결 코드</h3>
    <p style="font-size:13px;color:var(--text-muted);margin-bottom:8px">
      아래 코드를 <b>@Luminamail_bot</b>에게 전송하세요
    </p>
    <div class="link-code-display">
      <div class="link-code-number">${settingsState.telegramCode}</div>
      <div class="link-code-label">이 코드를 Telegram 봇에게 보내세요</div>
    </div>
    <div class="link-status">
      <div class="link-spinner"></div>
      <span>코드 수신 대기 중...</span>
    </div>
    <button class="btn-secondary" onclick="cancelTelegramLinking()" style="margin-top:16px">취소</button>
  </div>`;
}

function renderTelegramConnected() {
  const link = settingsState.telegramLink;
  return `<div class="telegram-section">
    <div class="telegram-connected">
      <div class="tg-avatar">✈️</div>
      <div class="tg-info">
        <h4>@${link.username || 'User'}</h4>
        <p>✅ 연결됨 · ${link.created_at?.slice(0,10) || ''}</p>
      </div>
      <div class="tg-actions">
        <button class="btn-secondary" onclick="testTelegram()">🔔 테스트</button>
        <button class="btn-secondary" onclick="disconnectTelegram()" style="color:#ff453a">연결 해제</button>
      </div>
    </div>
    <div style="margin-top:20px;text-align:left">
      <h4 style="font-size:13px;font-weight:600;margin-bottom:8px">📋 알림 설정</h4>
      <p style="font-size:12px;color:var(--text-muted)">• 중요 메일 수신 시 요약 알림</p>
      <p style="font-size:12px;color:var(--text-muted)">• 인라인 버튼으로 승인/답장/고정</p>
      <p style="font-size:12px;color:var(--text-muted)">• 대용량 파일 발송 완료 알림</p>
    </div>
  </div>`;
}

async function startTelegramLinking() {
  try {
    settingsState.telegramCode = await invoke('telegram_start_linking');
    settingsState.telegramPolling = true;
    renderSettingsBody();
    pollTelegramLink();
  } catch(e) { alert('코드 생성 실패: ' + e); }
}

async function pollTelegramLink() {
  if (!settingsState.telegramPolling) return;
  try {
    const result = await invoke('telegram_poll_link', { code: settingsState.telegramCode });
    if (result) {
      settingsState.telegramPolling = false;
      settingsState.telegramCode = null;
      await loadSettingsData();
      // Update sidebar status
      updateTelegramStatus(true);
      return;
    }
  } catch(e) { console.error('Poll error:', e); }
  if (settingsState.telegramPolling) {
    setTimeout(pollTelegramLink, 3000);
  }
}

function cancelTelegramLinking() {
  settingsState.telegramPolling = false;
  settingsState.telegramCode = null;
  renderSettingsBody();
}

async function testTelegram() {
  try {
    await invoke('telegram_send_test');
    alert('테스트 알림이 전송되었습니다! Telegram을 확인하세요.');
  } catch(e) { alert('전송 실패: ' + e); }
}

async function disconnectTelegram() {
  if (!confirm('Telegram 연결을 해제하시겠습니까?')) return;
  try {
    await invoke('telegram_disconnect');
    settingsState.telegramLink = null;
    renderSettingsBody();
    updateTelegramStatus(false);
  } catch(e) { alert('해제 실패: ' + e); }
}

function updateTelegramStatus(connected) {
  const dot = document.querySelector('.telegram-dot');
  if (dot) dot.style.background = connected ? '#30d158' : '#8e8e93';
}

function bindSettingsEvents() {
  // Any dynamic event binding needed after render
}

// ╔═══════════════════════════════════════════════╗
// ║   EMAIL SYNC FUNCTIONS                        ║
// ╚═══════════════════════════════════════════════╝

async function testEmailConnection(accountId) {
  const statusEl = document.getElementById(`acc-status-${accountId}`);
  if (statusEl) statusEl.innerHTML = '<span style="color:#ffd60a">⏳ 연결 중...</span>';
  try {
    const result = await invoke('test_email_connection', { accountId });
    if (statusEl) statusEl.innerHTML = `<span style="color:#30d158">${result}</span>`;
  } catch(e) {
    if (statusEl) statusEl.innerHTML = `<span style="color:#ff453a">❌ ${e}</span>`;
  }
}

async function syncEmailAccount(accountId) {
  const statusEl = document.getElementById(`acc-status-${accountId}`);
  if (statusEl) statusEl.innerHTML = '<span style="color:#ffd60a">⏳ 메일 동기화 중... (최대 30통)</span>';
  try {
    const result = await invoke('sync_email_account', { accountId });
    if (statusEl) statusEl.innerHTML = `<span style="color:#30d158">${result}</span>`;
    // Refresh thread list in main app
    if (window.__TAURI__ && window.location.protocol !== 'file:') {
      // Trigger app reload of threads
      const event = new CustomEvent('threads-updated');
      window.dispatchEvent(event);
    }
  } catch(e) {
    if (statusEl) statusEl.innerHTML = `<span style="color:#ff453a">❌ ${e}</span>`;
  }
}

async function syncAllAccounts() {
  const resultEl = document.getElementById('sync-all-result');
  if (resultEl) resultEl.innerHTML = '⏳ 전체 계정 동기화 중...';
  try {
    const result = await invoke('sync_all_accounts');
    if (resultEl) resultEl.innerHTML = result;
    const event = new CustomEvent('threads-updated');
    window.dispatchEvent(event);
  } catch(e) {
    if (resultEl) resultEl.innerHTML = `❌ ${e}`;
  }
}

// Make functions globally available
window.openSettings = openSettings;
window.closeSettings = closeSettings;
window.switchSettingsTab = switchSettingsTab;
window.startAddAccount = startAddAccount;
window.cancelAddAccount = cancelAddAccount;
window.submitAddAccount = submitAddAccount;
window.editAccount = editAccount;
window.updateSyncMode = updateSyncMode;
window.deleteAccount = deleteAccount;
window.startTelegramLinking = startTelegramLinking;
window.cancelTelegramLinking = cancelTelegramLinking;
window.testTelegram = testTelegram;
window.disconnectTelegram = disconnectTelegram;
window.testEmailConnection = testEmailConnection;
window.syncEmailAccount = syncEmailAccount;
window.syncAllAccounts = syncAllAccounts;

// ╔═══════════════════════════════════════════════╗
// ║   CLOUD STORAGE TAB                           ║
// ╚═══════════════════════════════════════════════╝

function renderCloudTab() {
  const gdriveConnected = settingsState.cloudProviders.includes('gdrive');
  const onedriveConnected = settingsState.cloudProviders.includes('onedrive');

  return `<div class="settings-section">
    <h3>☁️ 대용량 파일 클라우드 업로드</h3>
    <p style="font-size:12px;color:var(--text-muted);margin-bottom:16px">
      10MB 이상 첨부파일은 클라우드에 업로드하고 공유 링크를 메일에 포함합니다.
    </p>

    <div class="provider-grid">
      <!-- Google Drive -->
      <div class="provider-card cloud-card ${gdriveConnected ? 'connected' : ''}">
        <div class="provider-icon gdrive-icon">📁</div>
        <div class="provider-info">
          <h4>Google Drive</h4>
          <p>${gdriveConnected ? '✅ 연결됨' : '연결 안 됨'}</p>
        </div>
        <div class="cloud-card-action">
          ${gdriveConnected
            ? '<button class="btn-small" onclick="disconnectCloud(\'gdrive\')">연결 해제</button>'
            : '<button class="btn-primary" onclick="connectCloud(\'gdrive\')" style="padding:6px 14px;font-size:12px">🔗 연결</button>'
          }
        </div>
      </div>

      <!-- OneDrive -->
      <div class="provider-card cloud-card ${onedriveConnected ? 'connected' : ''}">
        <div class="provider-icon onedrive-icon">💾</div>
        <div class="provider-info">
          <h4>OneDrive</h4>
          <p>${onedriveConnected ? '✅ 연결됨' : '연결 안 됨'}</p>
        </div>
        <div class="cloud-card-action">
          ${onedriveConnected
            ? '<button class="btn-small" onclick="disconnectCloud(\'onedrive\')">연결 해제</button>'
            : '<button class="btn-primary" onclick="connectCloud(\'onedrive\')" style="padding:6px 14px;font-size:12px">🔗 연결</button>'
          }
        </div>
      </div>
    </div>

    <div id="cloud-auth-status" style="margin-top:12px;font-size:13px"></div>
  </div>

  <div class="settings-section">
    <h3>📋 업로드 설정</h3>
    <div class="sync-mode-box">
      <div class="sync-option">
        <div class="sync-option-text">
          <h4>☁️ 클라우드 전환 임계값</h4>
          <p>이 크기를 초과하는 첨부파일은 자동으로 클라우드에 업로드됩니다.</p>
          <p style="margin-top:6px;font-weight:600;color:var(--accent)">현재: 10MB</p>
        </div>
      </div>
      <div class="sync-option">
        <div class="sync-option-text">
          <h4>🔒 공유 링크 권한</h4>
          <p>업로드된 파일의 공유 링크는 <b>보기 전용</b>으로 생성됩니다.</p>
        </div>
      </div>
      <div class="sync-option">
        <div class="sync-option-text">
          <h4>📢 전송 완료 알림</h4>
          <p>대용량 파일 업로드 완료 시 Telegram으로 알림을 보냅니다. (Telegram 연결 필요)</p>
        </div>
      </div>
    </div>
  </div>`;
}

async function connectCloud(provider) {
  const statusEl = document.getElementById('cloud-auth-status');
  const providerName = provider === 'gdrive' ? 'Google Drive' : 'OneDrive';
  if (statusEl) statusEl.innerHTML = `⏳ ${providerName} 인증 중... 브라우저에서 로그인하세요.`;

  try {
    const result = await invoke('cloud_start_auth', { provider });
    if (statusEl) statusEl.innerHTML = `<span style="color:#30d158">${result}</span>`;
    await loadSettingsData();
  } catch(e) {
    if (statusEl) statusEl.innerHTML = `<span style="color:#ff453a">❌ ${e}</span>`;
  }
}

async function disconnectCloud(provider) {
  const providerName = provider === 'gdrive' ? 'Google Drive' : 'OneDrive';
  if (!confirm(`${providerName} 연결을 해제하시겠습니까?`)) return;
  try {
    await invoke('cloud_disconnect', { provider });
    await loadSettingsData();
  } catch(e) { alert('해제 실패: ' + e); }
}

// Make cloud functions globally available
window.connectCloud = connectCloud;
window.disconnectCloud = disconnectCloud;
window.renderCloudTab = renderCloudTab;

