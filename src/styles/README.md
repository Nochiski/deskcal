# DeskCal 스타일 체계

`src/main.tsx`는 `styles/index.css` 하나만 불러오고, 그 파일이 아래 순서로 나머지를 합칩니다.

```
tokens.css              모든 디자인 토큰(CSS 변수)의 기준값. 컴포넌트 규칙 없음.
themes/light.css        <html data-theme="light"> 일 때 색 토큰만 덮어씀
themes/dark.css         <html data-theme="dark">  일 때 색 토큰만 덮어씀
styles/glass.css        <html data-style="glass"> 리퀴드 글래스: 블러·광택·둥근 모양 토큰 + 블러 규칙 2개
styles/flat.css         <html data-style="flat">  플랫: 불투명 표면, 효과 끔, 각진 모양 (라이트/다크 값 모두 포함)
base.css                리셋, 타이포, 루트 패널(.app), .surface 프리미티브, 키프레임, 스크롤바
components/*.css        영역별 규칙 (buttons, topbar, grid, chips, popover, modal, settings, editor, responsive)
```

규칙은 하나입니다. **컴포넌트 CSS는 색·블러·반경·그림자를 직접 쓰지 않고 `var(--…)`만 읽습니다.**
그래서 테마 × 스타일 조합(light-glass, dark-glass, light-flat, dark-flat)이 자유롭게 섞입니다.

`App.tsx`가 런타임에 `<html>`에 세 속성을 찍습니다.

- `data-theme="light|dark"` — `settings.theme`에서 결정. `"system"`은 `prefers-color-scheme`으로 해석.
- `data-style="glass|flat"` — `settings.style`.
- `data-mode="floating|desktop|wallpaper"` — 창 모드. 고정 모드는 루트 모서리를 0으로, 틴트를 진하게.

## 토큰 작성 규칙

- 여러 값이 들어가는 토큰(`--gloss-*`, `--specular`, `--shadow-*`)은 항상 **유효한 값**이어야 합니다.
  쉼표 목록 안에 `none`이 들어가면 선언 전체가 무효가 되므로, "끔"은 `linear-gradient(transparent, transparent)` 또는
  `0 0 0 0 transparent`로 표현합니다.
- 색은 `--accent-rgb`처럼 `r, g, b` 형태의 토큰과 `rgba(var(--accent-rgb), .2)` 조합으로 투명도를 만듭니다.
- 새 컴포넌트를 만들면 `components/`에 파일을 추가하고 `index.css`에 한 줄 import 합니다.

## 새 테마 추가 (예: `sepia`)

1. `themes/sepia.css`를 만들고 `:root[data-theme="sepia"] { … }` 안에 색 토큰만 덮어씁니다. `dark.css`를 복사해 시작하면 됩니다.
2. `index.css`에 import를 추가합니다.
3. `src/lib/types.ts`의 `Settings.theme` 유니온과 `App.tsx`의 테마 해석, 설정 화면 선택지에 값을 추가합니다.

## 새 스타일 추가 (예: `neumorphic`)

1. `styles/neumorphic.css`를 만들고 `:root[data-style="neumorphic"] { … }` 안에 표면·효과·모양 토큰을 덮어씁니다.
   라이트/다크 값이 다르면 `:root[data-style="neumorphic"][data-theme="dark"]` 블록을 함께 둡니다.
2. 구조적으로 달라야 하는 규칙(예: backdrop-filter)만 같은 파일에 `:root[data-style="neumorphic"] .surface { … }` 형태로 둡니다.
3. `index.css` import, `Settings.style` 유니온, 설정 화면 선택지에 값을 추가합니다.
