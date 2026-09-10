# DeskCal

Windows용 초경량 데스크톱 캘린더. Google 캘린더와 iCloud(Apple) 캘린더를 한 화면에 월별로 보여주고,
작업표시줄에는 뜨지 않고 트레이 아이콘으로만 동작합니다. 바탕화면 위젯처럼 붙이거나 배경화면 뒤(아이콘 뒤)로
보낼 수 있고, 일정 시각에 맞춰 Windows 알림을 띄웁니다.

- Tauri v2 (Rust) + React/TypeScript. WebView2를 사용하므로 설치 용량이 작고 메모리 사용량이 낮습니다.
- 외부 npm 런타임 의존성 없음. 시크릿(토큰/앱 암호)은 Windows 자격 증명 관리자에 저장됩니다.

## 기능

| 영역 | 내용 |
| --- | --- |
| 월 보기 | Google 캘린더 스타일 그리드, 종일/여러 날 일정 바, 시간 일정 점+시간, `N개 더보기`, 일정 상세 팝오버 |
| 계정 | Google(OAuth 2.0 PKCE, 읽기 전용), Apple iCloud(CalDAV, 앱 암호) |
| 창 모드 | 일반 창 / 바탕화면 위젯(아이콘 위, 조작 가능, Win+D에도 유지) / 배경화면(아이콘 뒤, 보기 전용) |
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

### Google

1. [Google Cloud Console](https://console.cloud.google.com/) → 프로젝트 생성 → **API 및 서비스 → 라이브러리**에서
   *Google Calendar API* 사용 설정.
2. **OAuth 동의 화면** 구성(외부, 테스트 사용자에 본인 계정 추가) — 범위: `calendar.readonly`, `email`.
3. **사용자 인증 정보 → OAuth 클라이언트 ID → 애플리케이션 유형: 데스크톱 앱** 생성.
4. 앱의 설정 → 계정 → *고급: OAuth 클라이언트* 에 클라이언트 ID/시크릿 입력 후 **Google로 로그인**.
   빌드 시 `DESKCAL_GOOGLE_CLIENT_ID` / `DESKCAL_GOOGLE_CLIENT_SECRET` 환경 변수를 주면 기본값으로 포함됩니다.

로그인은 기본 브라우저에서 진행되며 `http://127.0.0.1:<임의 포트>` 루프백으로 인증 코드를 돌려받습니다.

### Apple (iCloud)

"Apple로 로그인(Sign in with Apple)"은 신원 확인만 제공하고 캘린더 데이터 접근 권한을 주지 않습니다.
iCloud 캘린더는 CalDAV로만 읽을 수 있으므로 **앱 암호**가 필요합니다.

1. <https://appleid.apple.com> → 로그인 및 보안 → **앱 암호** → 생성.
2. 앱의 설정 → 계정 → Apple ID와 앱 암호 입력 → 연결.

## 데이터 위치

- 설정/캐시: `%APPDATA%\com.sangmok.deskcal\` (`settings.json`, `accounts.json`, `cache.json`, `window.json`)
- 로그: `%LOCALAPPDATA%\com.sangmok.deskcal\logs\deskcal.log`
- 시크릿: Windows 자격 증명 관리자 (`DeskCal` 항목)

## 구조

```
src/                     React UI (월 그리드, 설정 모달, 팝오버)
src/lib/types.ts         프론트-백엔드 공용 데이터 계약
src/lib/api.ts           Tauri 커맨드/이벤트 래퍼 (브라우저에서는 mock)
src-tauri/src/
  model.rs               데이터 모델 (types.ts와 1:1)
  commands.rs            프론트에 노출되는 커맨드
  google.rs              OAuth PKCE + Calendar API v3
  apple.rs / ics.rs      iCloud CalDAV 탐색·조회, iCalendar 파서, RRULE 로컬 전개(폴백)
  sync.rs                제공자 통합 동기화 + 백그라운드 주기 동기화
  notify.rs              알림 스케줄러 (시스템 시계 기준, 캘린더별 on/off)
  window_mode.rs         Win32 창 모드 (Progman / WorkerW 재부모화)
  tray.rs                트레이 아이콘·메뉴
  store.rs / state.rs    JSON 저장소, 키링, 앱 상태
```

## 알려진 제한

- 배경화면 모드에서는 Windows 아이콘 레이어가 입력을 받으므로 캘린더를 클릭할 수 없습니다. 트레이 메뉴로 모드를 바꾸세요.
- 읽기 전용입니다(일정 생성/수정 없음).
- iCloud가 `expand`를 거부하면 반복 일정을 로컬에서 전개하는데, 이때는 DAILY/WEEKLY/MONTHLY/YEARLY의 기본 규칙만 지원합니다.
