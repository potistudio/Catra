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
  addedAt: number;
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
}

export type DuplicateChoice = "existing" | "new";

export interface ScanResult {
  added: number;
  skipped: number;
}

export interface ScanProgress {
  processed: number;
  added: number;
  skipped: number;
  currentPath: string;
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
}
