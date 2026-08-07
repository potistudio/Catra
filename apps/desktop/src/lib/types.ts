export interface Track {
  id: number;
  path: string;
  title: string | null;
  artist: string | null;
  album: string | null;
  durationMs: number | null;
  bpm: number | null;
  addedAt: number;
}

export interface ScanResult {
  added: number;
  skipped: number;
}
