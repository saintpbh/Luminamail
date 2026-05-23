cask "lumina-mail" do
  version "0.1.0"
  sha256 :no_check # 배포 단계에서 릴리즈 바이너리의 실제 sha256 해시값(shasum -a 256 [파일명].dmg)으로 갱신하는 것을 권장합니다.

  # GitHub Releases에 업로드된 Tauri macOS 빌드 산출물(.dmg) 경로 매핑
  # 실제 빌드 산출물 파일명 패턴에 맞게 URL을 커스텀할 수 있습니다.
  url "https://github.com/saintpbh/Luminamail/releases/download/v#{version}/Lumina.Mail_#{version}_x64.dmg"
  name "Lumina Mail"
  desc "Gemini AI 기반 챗 스타일 스마트 이메일 클라이언트"
  homepage "https://github.com/saintpbh/Luminamail"

  # macOS 응용 프로그램 설치 대상 지정
  app "Lumina Mail.app"

  # 앱 삭제(Uninstall) 시 찌꺼기 청소를 위한 zap 리스트 정의
  zap trash: [
    "~/Library/Application Support/com.bongpark.lumina-mail",
    "~/Library/Caches/com.bongpark.lumina-mail",
    "~/Library/Saved Application State/com.bongpark.lumina-mail.savedState",
    "~/Library/WebKit/com.bongpark.lumina-mail",
  ]
end
