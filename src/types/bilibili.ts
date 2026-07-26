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
  language: string;
  segments: SubtitleSegment[];
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
