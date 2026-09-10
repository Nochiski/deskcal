# DeskCal

Windows용 초경량 데스크톱 캘린더. Google 캘린더와 iCloud(Apple) 캘린더를 한 화면에 월별로 보여주고,
작업표시줄에는 뜨지 않고 트레이 아이콘으로만 동작합니다. 바탕화면 위젯처럼 붙이거나 배경화면 뒤(아이콘 뒤)로
보낼 수 있고, 일정 시각에 맞춰 Windows 알림을 띄웁니다. 일정 생성·수정·삭제도 됩니다.

- Tauri v2 (Rust) + React/TypeScript. WebView2를 사용하므로 설치 용량이 작고 메모리 사용량이 낮습니다.
- 외부 npm 런타임 의존성 없음. 시크릿(토큰/앱 암호)은 Windows 자격 증명 관리자에 저장됩니다.
- 디자인: 리퀴드 글래스(반투명 유리). 일반 창 모드에서는 Windows 아크릴로 뒤 화면이 실제로 블러됩니다.

## 기능

| 영역 | 내용 |
| --- | --- |
| 월 보기 | Google 캘린더 스타일 그리드, 종일/여러 날 일정 바, 시간 일정 점+시간, `N개 더보기`, 일정 상세 팝오버 |
| 계정 | Google(OAuth 2.0 PKCE, 읽기+쓰기), Apple iCloud(CalDAV, 앱 암호, 읽기+쓰기), iCal 구독 URL(로그인 불필요, 읽기 전용) |
| 일정 편집 | 상단 `일정 추가`, 날짜 칸 더블클릭, 상세 팝오버의 수정/삭제. Google·iCloud 캘린더에서 동작 |
| 창 모드 | 일반 창 / 바탕화면 위젯(아이콘 위, 조작 가능, Win+D에도 유지) / 배경화면(아이콘 뒤, 보기 전용). 탐색기 재시작 시 자동 재부착 |
| 알림 | 일정 알림 시각(제공자 알림 설정 또는 기본값)에 Windows 토스트. 캘린더별 알림 끄기, 종일 일정 알림 시각 지정 |
| 기타 | 트레이 메뉴, 시작 시 자동 실행, 백그라운드 동기화 주기, 라이트/다크 테마, 배경 불투명도, 주 시작 요일 |

## 개발

```bash
npm install
npm run tauri dev      # 앱 실행 (Rust 첫 빌드는 수 분 소요)
npm run dev            # 브라우저에서 UI만 (샘플 데이터 mock 사용)
npm run tauri build    # src-tauri/target/release/bundle/nsis/ 에 설치 파일 생성
cd src-tauri && cargo test
```

## 계정 연결 설정

### Google (OAuth) — 일정 읽기/쓰기

Google Calendar API는 무료입니다(프로젝트당 하루 100만 쿼리, 결제 등록 불필요).

1. [Google Cloud Console](https://console.cloud.google.com/) → 프로젝트 생성 → **API 및 서비스 → 라이브러리**에서
   *Google Calendar API* 사용 설정.
2. **OAuth 동의 화면** 구성(외부, 테스트 사용자에 본인 계정 추가). 범위: `calendar.readonly`, `calendar.events`, `email`.
3. **사용자 인증 정보 → OAuth 클라이언트 ID → 애플리케이션 유형: 데스크톱 앱** 생성.
   데스크톱 앱 유형은 `http://127.0.0.1:<포트>` 루프백 리디렉션이 자동 허용되므로 리디렉션 URI를 등록할 필요가 없습니다.
   ("웹 애플리케이션" 유형으로 만들면 안 됩니다.)
4. 클라이언트 ID/시크릿은 둘 중 한 곳에 넣습니다.
   - 프로젝트 루트 `.env` (git 제외, 빌드 시 앱에 내장):
     ```
     DESKCAL_GOOGLE_CLIENT_ID=xxxx.apps.googleusercontent.com
     DESKCAL_GOOGLE_CLIENT_SECRET=GOCSPX-xxxx
     ```
   - 또는 앱 설정 → 계정 → *고급: OAuth 클라이언트* 입력 (저장 위치 `%APPDATA%\com.sangmok.deskcal\settings.json`).
5. 앱 설정 → 계정 → **Google로 로그인** (기본 브라우저에서 진행).

> 주의: OAuth 동의 화면이 **"테스트" 게시 상태**이면 Google이 리프레시 토큰을 7일 뒤 만료시켜 매주 다시 로그인해야 합니다.
> 동의 화면에서 **"앱 게시(프로덕션)"** 로 바꾸면(검증 없이도 가능, 로그인 시 "확인되지 않은 앱" 경고만 뜸) 만료되지 않습니다.
> Google Workspace 계정이라면 사용자 유형을 "내부"로 두면 이 제한이 없습니다.

### Google — 로그인 없이 보기만 (iCal 구독)

OAuth 설정이 번거로우면 Google 캘린더 웹 → 설정 → 해당 캘린더 → **"비공개 주소(iCal 형식)"** URL을 복사해
앱 설정 → 계정 → *iCal 구독(URL)* 에 붙여넣으세요. 로그인·API 키 없이 읽기 전용으로 표시됩니다.
(공휴일 캘린더 등 아무 `.ics`/`webcal://` 피드도 됩니다.)

### Apple (iCloud) — 일정 읽기/쓰기

"Apple로 로그인(Sign in with Apple)"은 신원 확인만 제공하고 캘린더 데이터 접근 권한을 주지 않습니다.
iCloud 캘린더는 CalDAV로만 접근할 수 있으므로 **앱 암호**가 필요합니다.

1. <https://appleid.apple.com> → 로그인 및 보안 → **앱 암호** → 생성.
2. 앱의 설정 → 계정 → Apple ID와 앱 암호 입력 → 연결.

## 데이터 위치

- 설정/캐시: `%APPDATA%\com.sangmok.deskcal\` (`settings.json`, `accounts.json`, `cache.json`, `window.json`)
- 로그: `%LOCALAPPDATA%\com.sangmok.deskcal\logs\deskcal.log`
- 시크릿: Windows 자격 증명 관리자 (`DeskCal` 항목)

## 구조

```
src/                     React UI (월 그리드, 일정 편집기, 설정 모달, 팝오버)
src/lib/types.ts         프론트-백엔드 공용 데이터 계약
src/lib/api.ts           Tauri 커맨드/이벤트 래퍼 (브라우저에서는 mock)
src-tauri/build.rs       .env의 DESKCAL_* 값을 빌드에 내장
src-tauri/src/
  model.rs               데이터 모델 (types.ts와 1:1)
  commands.rs            프론트에 노출되는 커맨드 (동기화, 계정, 일정 CRUD, iCal 피드)
  google.rs              OAuth PKCE + Calendar API v3 (읽기/쓰기)
  apple.rs               iCloud CalDAV 탐색·조회·PUT/DELETE
  feed.rs                iCal 구독 URL (읽기 전용)
  ics.rs / vevent.rs     iCalendar 파서, RRULE 로컬 전개, VEVENT 직렬화
  sync.rs                제공자 통합 동기화 + 백그라운드 주기 동기화
  notify.rs              알림 스케줄러 (시스템 시계 기준, 캘린더별 on/off)
  window_mode.rs         Win32 창 모드 (Progman / WorkerW 재부모화, 아크릴, 라운드 코너)
  tray.rs                트레이 아이콘·메뉴
  store.rs / state.rs    JSON 저장소, 키링, 앱 상태
```

## 알려진 제한

- 배경화면 모드에서는 Windows 아이콘 레이어가 입력을 받으므로 캘린더를 클릭할 수 없습니다. 트레이 메뉴로 모드를 바꾸세요.
- iCloud 반복 일정은 앱에서 수정할 수 없습니다(단일 일정만). Google은 반복 일정의 개별 회차 수정이 됩니다.
- iCal 구독은 읽기 전용이며, 반복 규칙은 DAILY/WEEKLY/MONTHLY/YEARLY의 기본 형태만 전개합니다.
- 일정을 다른 캘린더로 옮기는 기능은 없습니다.
