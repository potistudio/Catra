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
  return track.artist ?? "Unknown Artist";
}
