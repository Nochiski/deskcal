# DeskCal 개인정보처리방침 / Privacy Policy

최종 수정일: 2026-09-10

## 요약

DeskCal은 사용자의 PC에서만 동작하는 Windows 데스크톱 캘린더 앱입니다.
개발자는 어떤 서버도 운영하지 않으며, 사용자의 일정·계정 정보·사용 기록을 **수집, 저장, 전송, 판매하지 않습니다.**

## 앱이 다루는 데이터

| 데이터 | 저장 위치 | 외부 전송 |
| --- | --- | --- |
| Google 계정 OAuth 토큰 | 사용자 PC의 Windows 자격 증명 관리자 | Google 서버와의 인증 통신에만 사용 |
| Apple ID 및 앱 암호(iCloud) | 사용자 PC의 Windows 자격 증명 관리자 | Apple iCloud CalDAV 서버와의 인증 통신에만 사용 |
| 캘린더 목록 및 일정(제목, 시간, 장소, 설명, 알림) | 사용자 PC (`%APPDATA%\com.sangmok.deskcal\cache.json`) | 없음 |
| iCal 구독 URL | 사용자 PC (`%APPDATA%\com.sangmok.deskcal\accounts.json`) | 해당 URL의 서버에서 피드를 내려받을 때만 사용 |
| 앱 설정(창 모드, 테마, 알림 설정 등) | 사용자 PC (`%APPDATA%\com.sangmok.deskcal\settings.json`) | 없음 |
| 로그 | 사용자 PC (`%LOCALAPPDATA%\com.sangmok.deskcal\logs`) | 없음 |

## Google 사용자 데이터 사용

DeskCal은 Google Calendar API를 통해 다음 범위(scope)를 요청합니다.

- `calendar.readonly` — 캘린더 목록과 일정을 읽어 화면에 표시하고 알림을 띄우기 위해 사용합니다.
- `calendar.events` — 사용자가 앱에서 직접 생성·수정·삭제한 일정을 Google 캘린더에 반영하기 위해 사용합니다.
- `email` — 연결된 계정을 설정 화면에 표시하기 위해 이메일 주소를 읽습니다.

Google API로 받은 데이터는 사용자의 PC에만 보관되고, 앱의 캘린더 표시·알림·편집 기능 외의 목적으로 사용되지 않으며,
제3자에게 제공되거나 광고·분석에 이용되지 않습니다. DeskCal의 Google 사용자 데이터 사용은
[Google API 서비스 사용자 데이터 정책](https://developers.google.com/terms/api-services-user-data-policy)의
제한적 사용(Limited Use) 요건을 준수합니다.

## 데이터 삭제

- 앱 설정 → 계정 → **연결 해제**를 누르면 해당 계정의 토큰/암호가 자격 증명 관리자에서 삭제되고 캐시된 일정이 제거됩니다.
- 앱을 제거한 뒤 `%APPDATA%\com.sangmok.deskcal` 폴더를 삭제하면 모든 로컬 데이터가 없어집니다.
- Google 계정에 부여한 권한은 <https://myaccount.google.com/permissions> 에서 언제든 취소할 수 있습니다.

## 네트워크 통신

앱은 다음 서버와만 통신합니다: Google(accounts.google.com, oauth2.googleapis.com, www.googleapis.com),
Apple iCloud(caldav.icloud.com 및 계정별 pNN-caldav.icloud.com), 사용자가 직접 추가한 iCal 구독 URL.
텔레메트리, 크래시 리포트, 광고, 분석 도구는 포함되어 있지 않습니다.

## 문의

문의는 GitHub 저장소의 Issues를 이용해 주세요.

---

## Summary (English)

DeskCal is a Windows desktop calendar that runs entirely on your PC. The developer operates no servers and does
not collect, store, transmit, or sell your events, credentials, or usage data.

- Google OAuth tokens and the iCloud app-specific password are stored only in the Windows Credential Manager on your PC.
- Calendar data fetched from Google Calendar (`calendar.readonly`, `calendar.events`, `email` scopes) or iCloud CalDAV is
  cached locally under `%APPDATA%\com.sangmok.deskcal` and used solely to display events, fire reminders, and sync edits you
  make in the app. It is never shared with third parties or used for advertising or analytics. DeskCal's use of Google
  user data complies with the Google API Services User Data Policy, including the Limited Use requirements.
- The app talks only to Google, Apple iCloud, and any iCal feed URLs you add yourself. There is no telemetry.
- Disconnect an account in Settings to delete its credentials and cached events; delete `%APPDATA%\com.sangmok.deskcal`
  to remove all local data; revoke Google access at <https://myaccount.google.com/permissions>.
