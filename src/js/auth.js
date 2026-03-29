// ═══════════════════════════════════════════════════════════
// Lumina Mail - Authentication (Firebase)
// ═══════════════════════════════════════════════════════════

import { initializeApp } from "https://www.gstatic.com/firebasejs/10.11.0/firebase-app.js";
import { 
  getAuth, 
  signInWithEmailAndPassword, 
  createUserWithEmailAndPassword, 
  onAuthStateChanged, 
  signOut, 
  GoogleAuthProvider, 
  signInWithPopup 
} from "https://www.gstatic.com/firebasejs/10.11.0/firebase-auth.js";

// TODO: Replace with the actual Firebase config
const firebaseConfig = {
  apiKey: "AIzaSyCvETpIIPSXuBD7sqdSUtxWX5H_thF3gjA",
  authDomain: "luminamail-da62d.firebaseapp.com",
  projectId: "luminamail-da62d",
  storageBucket: "luminamail-da62d.firebasestorage.app",
  messagingSenderId: "1026808577815",
  appId: "1:1026808577815:web:cea1cb3f35e3c9a971c152",
  measurementId: "G-D5L456KK3M"
};

const app = initializeApp(firebaseConfig);
export const auth = getAuth(app);
const googleProvider = new GoogleAuthProvider();

document.addEventListener('DOMContentLoaded', () => {
  const authContainer = document.getElementById('auth-container');
  const appContainer = document.getElementById('app');
  
  const emailInput = document.getElementById('auth-email');
  const passwordInput = document.getElementById('auth-password');
  const errorDiv = document.getElementById('auth-error');
  
  const loginBtn = document.getElementById('auth-login-btn');
  const registerBtn = document.getElementById('auth-register-btn');
  const googleBtn = document.getElementById('auth-google-btn');

  function showError(msg) {
    errorDiv.textContent = msg;
    errorDiv.style.display = 'block';
  }

  function clearError() {
    errorDiv.style.display = 'none';
    errorDiv.textContent = '';
  }

  // Handle Auth State Changes
  onAuthStateChanged(auth, (user) => {
    if (user || localStorage.getItem('imap_logged_in') === 'true') {
      // Logged in
      authContainer.style.display = 'none';
      appContainer.style.display = 'grid';
    } else {
      // Logged out
      authContainer.style.display = 'flex';
      appContainer.style.display = 'none';
    }
  });

  loginBtn.addEventListener('click', async () => {
    clearError();
    const email = emailInput.value.trim();
    const password = passwordInput.value;
    if(!email || !password) return showError("이메일과 비밀번호를 입력해주세요.");
    
    try {
      await signInWithEmailAndPassword(auth, email, password);
    } catch (err) {
      console.error(err);
      if(err.code === 'auth/invalid-credential') {
        showError("로그인 정보가 올바르지 않습니다.");
      } else {
        showError(err.message);
      }
    }
  });

  registerBtn.addEventListener('click', async () => {
    clearError();
    const email = emailInput.value.trim();
    const password = passwordInput.value;
    if(!email || !password) return showError("이메일과 비밀번호를 입력해주세요.");
    
    try {
      await createUserWithEmailAndPassword(auth, email, password);
    } catch (err) {
      console.error(err);
      if(err.code === 'auth/email-already-in-use') {
        showError("이미 가입된 이메일입니다.");
      } else if(err.code === 'auth/weak-password') {
        showError("비밀번호는 최소 6자리 이상이어야 합니다.");
      } else {
        showError(err.message);
      }
    }
  });

  googleBtn.addEventListener('click', () => {
    clearError();
    document.getElementById('auth-imap-modal').style.display = 'block';
  });

  const imapCancelBtn = document.getElementById('imap-cancel-btn');
  const imapConnectBtn = document.getElementById('imap-connect-btn');
  const imapEmailInput = document.getElementById('imap-email');
  const imapPasswordInput = document.getElementById('imap-password');
  const imapErrorDiv = document.getElementById('imap-error');
  const imapLoadingDiv = document.getElementById('imap-loading');

  if (imapCancelBtn) {
    imapCancelBtn.addEventListener('click', () => {
      document.getElementById('auth-imap-modal').style.display = 'none';
    });
  }

  if (imapConnectBtn) {
    imapConnectBtn.addEventListener('click', async () => {
      const email = imapEmailInput.value.trim();
      let password = imapPasswordInput.value.trim();
      
      // Google App Passwords often contain spaces when copied (e.g. "abcd efgh ijkl mnop")
      // Remove all whitespaces.
      password = password.replace(/\s+/g, '');

      if (!email || !password) {
        imapErrorDiv.textContent = '이메일과 앱 비밀번호를 입력해주세요.';
        imapErrorDiv.style.display = 'block';
        return;
      }
      
      imapErrorDiv.style.display = 'none';
      imapLoadingDiv.style.display = 'block';
      imapConnectBtn.disabled = true;

      try {
        const invoke = window.__TAURI__.core.invoke;
        
        // 1. Add IMAP Account to SQLite
        const accId = await invoke('add_email_account', {
          provider: 'gmail',
          email: email,
          displayName: email.split('@')[0],
          imapHost: 'imap.gmail.com',
          imapPort: 993,
          smtpHost: 'smtp.gmail.com',
          smtpPort: 465,
          username: email,
          password: password,
          syncMode: 'readonly'
        });

        // 2. Fetch Latest 30 Emails via IMAP
        await invoke('sync_email_account', { accountId: accId });

        // 3. Bypass Firebase Auth & Load Dashboard
        localStorage.setItem('imap_logged_in', 'true');
        imapLoadingDiv.style.display = 'none';
        document.getElementById('auth-imap-modal').style.display = 'none';
        authContainer.style.display = 'none';
        appContainer.style.display = 'grid';
        
        // Force refresh threads in app.js
        if(window.loadThreads) {
          window.loadThreads();
        }
      } catch (err) {
        console.error(err);
        imapErrorDiv.textContent = 'IMAP 연결 실패: ' + err;
        imapErrorDiv.style.display = 'block';
        imapLoadingDiv.style.display = 'none';
        imapConnectBtn.disabled = false;
      }
    });
  }
});

export async function logoutUser() {
  localStorage.removeItem('imap_logged_in');
  await signOut(auth);
  window.location.reload();
}
