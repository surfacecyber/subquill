import type { MessageKey, UiLocale } from "./index";
import { t } from "./index";

export type ErrorCategory =
  | "storage"
  | "validation"
  | "network"
  | "bilibili"
  | "llm"
  | "job"
  | "unknown";

const STORAGE_CODES = new Set(["STORAGE_ERROR", "SERIALIZATION_ERROR"]);

const VALIDATION_CODES = new Set(["VALIDATION_ERROR"]);

const NETWORK_CODES = new Set(["NETWORK_ERROR"]);

const BILIBILI_CODES = new Set([
  "SHORT_LINK_FAILED",
  "VIDEO_NOT_FOUND",
  "VIDEO_RESTRICTED",
  "API_REJECTED",
  "PART_OUT_OF_RANGE",
  "AUTH_REQUIRED",
  "RATE_LIMITED",
  "NO_SUBTITLE",
  "SUBTITLE_CORRUPT",
]);

const LLM_CODES = new Set([
  "LLM_AUTH_ERROR",
  "LLM_RATE_LIMITED",
  "LLM_TIMEOUT",
  "LLM_NETWORK_ERROR",
  "LLM_API_ERROR",
  "LLM_INVALID_RESPONSE",
]);

const JOB_CODES = new Set([
  "JOB_CANCELLED",
  "JOB_TIMEOUT",
  "JOB_ALREADY_RUNNING",
  "JOB_NOT_FOUND",
  "JOB_NOT_READY",
  "INTERNAL_ERROR",
]);

const SPECIFIC_ERROR_KEYS: Partial<Record<string, MessageKey>> = {
  NO_SUBTITLE: "error_NO_SUBTITLE",
  SUBTITLE_CORRUPT: "error_SUBTITLE_CORRUPT",
  VIDEO_NOT_FOUND: "error_VIDEO_NOT_FOUND",
  VIDEO_RESTRICTED: "error_VIDEO_RESTRICTED",
  AUTH_REQUIRED: "error_AUTH_REQUIRED",
  RATE_LIMITED: "error_RATE_LIMITED",
  PART_OUT_OF_RANGE: "error_PART_OUT_OF_RANGE",
  SHORT_LINK_FAILED: "error_SHORT_LINK_FAILED",
  API_REJECTED: "error_API_REJECTED",
  LLM_AUTH_ERROR: "error_LLM_AUTH_ERROR",
  LLM_RATE_LIMITED: "error_LLM_RATE_LIMITED",
  LLM_TIMEOUT: "error_LLM_TIMEOUT",
  LLM_INVALID_RESPONSE: "error_LLM_INVALID_RESPONSE",
  JOB_TIMEOUT: "error_JOB_TIMEOUT",
  JOB_CANCELLED: "error_JOB_CANCELLED",
  JOB_ALREADY_RUNNING: "error_JOB_ALREADY_RUNNING",
  NETWORK_ERROR: "error_NETWORK_ERROR",
};

const ERROR_MESSAGE_KEYS: Record<ErrorCategory, MessageKey> = {
  storage: "error_storage",
  validation: "error_validation",
  network: "error_NETWORK_ERROR",
  bilibili: "error_bilibili",
  llm: "error_llm",
  job: "error_job",
  unknown: "error_unknown",
};

export function categorizeErrorCode(code: string): ErrorCategory {
  if (STORAGE_CODES.has(code)) {
    return "storage";
  }
  if (VALIDATION_CODES.has(code)) {
    return "validation";
  }
  if (NETWORK_CODES.has(code)) {
    return "network";
  }
  if (BILIBILI_CODES.has(code)) {
    return "bilibili";
  }
  if (LLM_CODES.has(code)) {
    return "llm";
  }
  if (JOB_CODES.has(code)) {
    return "job";
  }
  return "unknown";
}

export function localizeError(
  locale: UiLocale,
  code: string,
  fallbackMessage?: string,
): { title: string; detail?: string } {
  const specificKey = SPECIFIC_ERROR_KEYS[code];
  if (specificKey) {
    return { title: t(locale, specificKey) };
  }

  const category = categorizeErrorCode(code);
  const title = t(locale, ERROR_MESSAGE_KEYS[category]);

  if (category === "unknown" && fallbackMessage) {
    return { title, detail: fallbackMessage };
  }

  return { title };
}
