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

## 배포

`npm run tauri build`로 만든 `DeskCal_x.y.z_x64-setup.exe`를 GitHub Releases에 첨부합니다.
서명이 없으므로 사용자에게 SmartScreen 경고가 표시됩니다. 없애려면 코드 서명 인증서가 필요합니다.
