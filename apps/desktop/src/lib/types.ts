export type TrackSource = "soundcloud" | "bandcamp";

export interface Track {
  id: number;
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  durationMs: number | null;
  bpm: number | null;
  bitrateKbps: number | null;
  genre: string | null;
  key: string | null;
  rating: number | null;
  artworkPath: string | null;
  source: TrackSource | null;
  converted: boolean;
  addedAt: number;
  contentHash?: string | null;
  trashedAt?: number | null;
  parentTrackId?: number | null;
  formatGroupId?: string | null;
  rekordboxContentId?: string | null;
}

export interface TrackCandidate {
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  durationMs: number | null;
  bpm: number | null;
  bitrateKbps: number | null;
  genre: string | null;
  key: string | null;
  rating: number | null;
  source: TrackSource | null;
}

export interface DuplicateFoundPayload {
  existing: Track;
  candidate: TrackCandidate;
  allowAltFormat: boolean;
}

export type DuplicateChoice = "existing" | "new" | "altFormat";

export interface ScanResult {
  added: number;
  skipped: number;
}

export interface ScanProgress {
  processed: number;
  added: number;
  skipped: number;
  currentPath: string;
  total?: number | null;
}

export type ActivityLogLevel = "info" | "success" | "warning" | "error";

export interface ActivityLogEntry {
  id: number;
  timestamp: number;
  level: ActivityLogLevel;
  message: string;
  detail?: string;
}

export interface DownloadProgress {
  status: string;
  phase: "idle" | "queued" | "downloading" | "importing";
  percent?: number | null;
  current?: number | null;
  total?: number | null;
  queuePosition?: number | null;
  message?: string | null;
}

export type ConvertFormat =
  | "mp3"
  | "aac"
  | "flac"
  | "wav"
  | "aiff"
  | "ogg"
  | "opus";

export interface ConvertOptions {
  format: ConvertFormat;
  bitrateKbps?: number | null;
  bitDepth?: number | null;
  sampleRate?: number | null;
  channels?: number | null;
  addToRekordbox?: boolean;
}

export interface ConvertProgress {
  processed: number;
  total: number;
  converted: number;
  skipped: number;
  failed: number;
  currentPath: string;
  percent?: number | null;
  message?: string | null;
}

export interface ConvertResult {
  converted: number;
  skipped: number;
  failed: number;
  rekordboxAdded: number;
  rekordboxFailed: number;
}

export interface ActivityLogPayload {
  level: ActivityLogLevel;
  message: string;
  detail?: string;
}

export interface PreviewableTrack {
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  durationMs: number | null;
  bpm: number | null;
  bitrateKbps: number | null;
  genre: string | null;
  key: string | null;
  rating: number | null;
  artworkPath?: string | null;
}

export interface RekordboxCheck {
  dbPath: string | null;
  analysisRoot: string | null;
  settingsRoot: string | null;
  installDir: string | null;
  version: string | null;
  rekordboxRunning: boolean;
}

export interface RekordboxDbStatus {
  trackCount: number;
  playlistCount: number;
}

export interface RekordboxPlaylist {
  id: string;
  name: string;
  /** 0: playlist, 1: folder, 4: smart playlist */
  attribute: number;
  parentId: string | null;
  seq: number;
}

export interface RekordboxContent {
  id: string;
  folderPath: string;
  fileName: string | null;
  title: string | null;
  artist: string | null;
  album: string | null;
  genre: string | null;
  bpm: number | null;
  lengthSecs: number | null;
  trackNo: number | null;
  bitRate: number | null;
  bitDepth: number | null;
  comment: string | null;
  fileType: number | null;
  rating: number | null;
  releaseYear: number | null;
  key: string | null;
  remixer: string | null;
  label: string | null;
  composer: string | null;
  fileSize: number | null;
  discNo: number | null;
  artworkPath: string | null;
  /** Present when the row comes from a playlist membership query. */
  songPlaylistId?: string | null;
}

export interface RekordboxContentUpdate {
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  genre?: string | null;
  comment?: string | null;
  bpm?: number | null;
  rating?: number | null;
  trackNo?: number | null;
  releaseYear?: number | null;
  key?: string | null;
}

export interface HealthIssue {
  trackId: number;
  kind: string;
  path: string;
}

export interface HealthReport {
  checked: number;
  missing: number;
  hashMismatch: number;
  issues: HealthIssue[];
}
