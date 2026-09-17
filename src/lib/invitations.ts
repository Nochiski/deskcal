import type { ResponseStatus } from "./types";

export const RESPONSE_LABEL: Record<ResponseStatus, string> = {
  needsAction: "응답 대기",
  accepted: "수락",
  declined: "거절",
  tentative: "미정",
};

export const RESPONSE_ICON: Record<ResponseStatus, string> = {
  needsAction: "○",
  accepted: "✓",
  declined: "×",
  tentative: "?",
};
