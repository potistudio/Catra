export function formatDuration(ms: number | null): string {
  if (ms === null || ms <= 0) return "--:--";

  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;

  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function displayTitle(track: { title: string | null; path: string }): string {
  if (track.title) return track.title;

  const filename = track.path.split(/[/\\]/).pop();
  return filename ?? "Unknown";
}

export function displayArtist(track: { artist: string | null }): string {
  return track.artist ?? "—";
}

export function formatBitrate(kbps: number | null): string {
  if (kbps === null || kbps <= 0) return "—";
  return `${kbps} kbps`;
}

export function formatBpm(bpm: number | null): string {
  if (bpm === null || bpm <= 0) return "—";
  return bpm.toFixed(1);
}

export function formatRating(rating: number | null): string {
  if (rating === null || rating === 0) return "—";

  const stars = Math.max(1, Math.min(5, Math.round((rating / 255) * 5)));
  return `${"★".repeat(stars)}${"☆".repeat(5 - stars)}`;
}

export function displayValue(value: string | null): string {
  return value && value.trim() ? value : "—";
}
