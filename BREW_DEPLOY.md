# 🍺 Lumina Mail - Homebrew Cask 배포 및 배포 자동화 가이드

본 문서는 macOS 사용자들이 Homebrew 패키지 관리자(`brew install`)를 통해 Lumina Mail 앱을 초간단 설치 및 자동 업데이트할 수 있도록 배포(Tap & Cask) 체계를 구축하고 배포하는 가이드북입니다.

---

## 1. 개요 및 인프라 구조
Homebrew에서 GUI를 가진 데스크톱 앱(`.dmg` 또는 `.app`)은 **Cask**라는 형식을 사용하여 배포됩니다.
일반적으로 GitHub Releases 자산(Asset)에 올라간 `.dmg` 배포 파일을 Homebrew가 다운로드하여 `/Applications` 폴더에 자동으로 이식해 줍니다.

```
[로컬 빌드 및 푸시] ──> [GitHub Release 태그 생성 (.dmg 업로드)]
                                  │
                                  ▼ (다운로드 URL 감지)
[사용자 터미널] ◀── [Homebrew Cask (lumina-mail.rb)]
```

---

## 2. 1단계: 개인 Homebrew Tap 저장소 개설하기
일반 사용자가 `brew install --cask [사용자이름]/[탭이름]/lumina-mail` 형태로 설치하게 만들려면, 본인의 깃허브에 전용 탭(Tap) 저장소를 하나 개설해야 합니다.

1. 본인의 GitHub에 **`homebrew-luminamail`** (혹은 `homebrew-tap`)이라는 이름의 **Public** 저장소를 개설합니다.
   - **중요**: Homebrew Tap의 네이밍 규칙에 맞추기 위해 반드시 저장소 이름이 **`homebrew-`** 로 시작해야 합니다.
2. 개설된 깃허브 저장소를 로컬에 클론합니다.
   ```bash
   git clone https://github.com/saintpbh/homebrew-luminamail.git
   cd homebrew-luminamail
   ```
3. 디렉토리 내에 `Casks` 폴더를 생성하고, 이 프로젝트에 빌드되어 있는 **`lumina-mail.rb`** 파일을 복사해 넣습니다.
   ```bash
   mkdir Casks
   cp /path/to/Lumina-Mail/lumina-mail.rb Casks/lumina-mail.rb
   ```

---

## 3. 2단계: 신규 릴리즈 시 Cask 파일 업데이트 및 해시 적용
새로운 버전(예: `v0.1.0`)을 릴리즈하여 깃허브 Releases에 `.dmg` 바이너리를 업로드한 직후 다음을 수행합니다.

1. 업로드된 `.dmg` 파일의 SHA-256 해시값(무결성 체크용)을 터미널에서 추출합니다.
   ```bash
   shasum -a 256 Lumina.Mail_0.1.0_x64.dmg
   # 예시 출력: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
   ```
2. 복사해 둔 `Casks/lumina-mail.rb` 파일을 텍스트 에디터로 엽니다.
3. `version` 문자열과 `sha256` 해시 부분을 추출된 실제 값으로 정교하게 업데이트합니다.
   ```ruby
   cask "lumina-mail" do
     version "0.1.0" # 새 버전 기입
     sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" # sha256 해시값 기입
     ...
   ```
4. 업데이트된 Cask 정의를 본인의 Tap 깃허브 저장소에 커밋 및 푸시합니다.
   ```bash
   git add .
   git commit -m "release: Lumina Mail v0.1.0"
   git push origin main
   ```

---

## 4. 3단계: 일반 사용자의 설치 및 실행 (User Guide)
배포가 완료되면 모든 macOS 사용자들은 단 한 줄의 터미널 명령어로 Lumina Mail을 안전하게 설치하고 실행할 수 있습니다!

### 📥 1. 신규 설치 명령어
터미널을 열고 본인의 전용 탭 저장소를 주입하여 설치를 명령합니다:
```bash
brew install --cask saintpbh/luminamail/lumina-mail
```

### 🔄 2. 최신 버전 업데이트 명령어
새 릴리즈가 배포되었을 때 즉시 로컬 앱을 무결성 업그레이드합니다:
```bash
brew update
brew upgrade --cask lumina-mail
```

### 🧹 3. 완벽한 삭제 및 청소 명령어
앱을 삭제할 때 설정값과 SQLite 데이터베이스, 찌꺼기 파일까지 완벽하게 소독 및 청소합니다:
```bash
brew uninstall --cask lumina-mail
```

---

## 5. Homebrew 공식 Cask 저장소 등록 방법 (옵션)
Lumina Mail이 더 대중화되어 누구나 `saintpbh/` 탭 명시 없이 `brew install --cask lumina-mail`로만 즉각 설치하게 하고 싶다면, Homebrew 공식 중앙 저장소에 Cask를 등재(PR 제출)하면 됩니다.

1. Homebrew Cask 중앙 저장소 규격을 테스트합니다:
   ```bash
   brew audit --cask Casks/lumina-mail.rb
   brew style --cask Casks/lumina-mail.rb
   ```
2. 중앙 공식 저장소로의 풀 리퀘스트(PR) 제출 절차:
   - [Homebrew Cask GitHub](https://github.com/Homebrew/homebrew-cask)를 Fork합니다.
   - `Casks/l/lumina-mail.rb` 경로에 파일을 올린 뒤 Commit/Push합니다.
   - Homebrew 저장소 방향으로 Pull Request를 날리면 홈브루 CI 봇이 빌드를 검증하고 메인테이너 승인 후 전 세계 Cask 공식 목록에 영구 등재됩니다!
