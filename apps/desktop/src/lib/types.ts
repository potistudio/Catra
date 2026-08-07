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

export interface ScanResult {
  added: number;
  skipped: number;
}

export type ActivityLogLevel = "info" | "success" | "warning" | "error";

export interface ActivityLogEntry {
  id: number;
  timestamp: number;
  level: ActivityLogLevel;
  message: string;
  detail?: string;
}

export interface ActivityLogPayload {
  level: ActivityLogLevel;
  message: string;
  detail?: string;
}
