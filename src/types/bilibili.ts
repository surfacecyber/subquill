export interface SubtitleSegment {
  start_ms: number;
  end_ms: number;
  text: string;
}

export interface BilibiliSubtitleResult {
  title: string;
  part_title?: string;
  bvid: string;
  aid: number;
  cid: number;
  p: number;
  page_count: number;
  duration_ms?: number;
  language: string;
  segments: SubtitleSegment[];
}

/** Lightweight metadata from `preview_video` (no subtitle body). */
export interface VideoPreview {
  title: string;
  platform: string;
  video_id: string;
  duration_ms: number;
  p: number;
  page_count: number;
  part_title?: string | null;
  has_subtitles: boolean;
  auth_required: boolean;
}

export type BilibiliErrorCode =
  | "VALIDATION_ERROR"
  | "SHORT_LINK_FAILED"
  | "VIDEO_NOT_FOUND"
  | "API_REJECTED"
  | "PART_OUT_OF_RANGE"
  | "AUTH_REQUIRED"
  | "RATE_LIMITED"
  | "NO_SUBTITLE"
  | "SUBTITLE_CORRUPT"
  | "NETWORK_ERROR";
