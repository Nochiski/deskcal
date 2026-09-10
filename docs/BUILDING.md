# 소스에서 빌드하기 (개발자용)

배포된 설치 파일을 쓰는 사용자는 이 문서를 볼 필요가 없습니다.

## 요구 사항

Rust(stable), Node.js 20+, Windows 10/11. Tauri v2 + React/TypeScript이며 런타임 npm 의존성은 없습니다.

```bash
npm install
npm run tauri dev      # 앱 실행 (첫 Rust 빌드는 수 분 소요)
npm run dev            # 브라우저에서 UI만 (샘플 데이터 mock)
npm run tauri build    # src-tauri/target/release/bundle/nsis/ 에 설치 파일 생성
cd src-tauri && cargo test
cd src-tauri && cargo test --lib feed -- --ignored   # 네트워크로 공개 iCal 피드를 실제 조회하는 테스트
```

## Google OAuth 클라이언트

배포 빌드에는 클라이언트가 내장되어 있지만, 소스에서 빌드하면 본인의 클라이언트가 필요합니다. 무료입니다.

1. [Google Cloud Console](https://console.cloud.google.com/) → 프로젝트 생성 → **API 및 서비스 → 라이브러리**에서 *Google Calendar API* 사용 설정.
2. **OAuth 동의 화면** 구성(외부). 범위: `calendar.readonly`, `calendar.events`, `email`.
   "테스트" 상태로 두면 리프레시 토큰이 7일마다 만료되므로 **앱 게시(프로덕션)** 로 바꾸는 것을 권합니다(검증 없이도 가능).
   테스트 상태로 쓸 때는 로그인할 계정을 테스트 사용자에 추가해야 "액세스 차단됨" 오류가 나지 않습니다.
3. **사용자 인증 정보 → OAuth 클라이언트 ID → 유형: 데스크톱 앱**. 루프백(`http://127.0.0.1:<포트>`) 리디렉션이 자동 허용되므로 URI 등록은 필요 없습니다.
4. 프로젝트 루트에 `.env` 파일을 만들고 값을 넣습니다(git 제외, 빌드 시 내장). `.env.example` 참고.
   ```
   DESKCAL_GOOGLE_CLIENT_ID=xxxx.apps.googleusercontent.com
   DESKCAL_GOOGLE_CLIENT_SECRET=GOCSPX-xxxx
   ```
   또는 앱 설정 → 계정 → *고급: OAuth 클라이언트* 에 입력해도 됩니다.

## 구조

```
src/                     React UI (월 그리드, 일정 편집기, 설정 모달, 팝오버)
src/lib/types.ts         프론트-백엔드 공용 데이터 계약
src/lib/api.ts           Tauri 커맨드/이벤트 래퍼 (브라우저에서는 mock)
src-tauri/build.rs       .env의 DESKCAL_* 값을 빌드에 내장
src-tauri/src/
  model.rs               데이터 모델 (types.ts와 1:1)
  commands.rs            프론트에 노출되는 커맨드 (동기화, 계정, 일정 CRUD, iCal 피드)
  google.rs              OAuth PKCE + Calendar API v3 (읽기/쓰기, 복수 계정)
  apple.rs               iCloud CalDAV 탐색·조회·PUT/DELETE
  feed.rs                iCal 구독 URL (읽기 전용)
  ics.rs / vevent.rs     iCalendar 파서, RRULE 로컬 전개, VEVENT 직렬화
  sync.rs                제공자 통합 동기화 + 백그라운드 주기 동기화
  notify.rs              알림 스케줄러 (시스템 시계 기준, 캘린더별 on/off)
  window_mode.rs         Win32 창 모드 (Progman / WorkerW 재부모화, 아크릴, 라운드 코너)
  tray.rs                트레이 아이콘·메뉴
  store.rs / state.rs    JSON 저장소, 키링, 앱 상태, 구버전 데이터 이전
```

## 데이터 위치

- 설정/캐시: `%APPDATA%\com.nochiski.deskcal\` (`settings.json`, `accounts.json`, `cache.json`, `window.json`)
- 로그: `%LOCALAPPDATA%\com.nochiski.deskcal\logs\deskcal.log`
- 시크릿: Windows 자격 증명 관리자 (`DeskCal` 항목, Google 계정별 `google_refresh_token:<id>`)

## 배포 (자동)

- PR이 `main`에 머지되면 `.github/workflows/release.yml`이 Windows 러너에서 설치 파일을 빌드하고 GitHub Release를 만듭니다.
  - 태그/제목: `src-tauri/tauri.conf.json`의 `version` → `v0.1.2`, `DeskCal 0.1.2`
  - 릴리스 노트: **머지된 PR 본문**을 그대로 사용합니다. 사용자에게 보여줄 변경 사항을 PR 본문에 쓰세요.
  - 같은 버전의 릴리스가 이미 있으면 빌드하지 않고 종료합니다. 릴리스를 내려면 PR에서 버전(`package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`)을 올리세요.
  - Google 클라이언트는 저장소 시크릿 `DESKCAL_GOOGLE_CLIENT_ID` / `DESKCAL_GOOGLE_CLIENT_SECRET`에서 주입됩니다.
  - Actions 탭에서 `Release` 워크플로를 수동 실행(workflow_dispatch)할 수도 있습니다.
- PR마다 `.github/workflows/ci.yml`이 타입 검사·프론트 빌드·Rust 테스트를 돌립니다.
- 서명이 없으므로 사용자에게 SmartScreen 경고가 표시됩니다. 없애려면 코드 서명 인증서가 필요합니다.
