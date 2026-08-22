export function formatDuration(ms: number | null): string {
	if (ms === null || ms <= 0) return "--:--";

	const totalSeconds = Math.floor(ms / 1000);
	const minutes = Math.floor(totalSeconds / 60);
	const seconds = totalSeconds % 60;

	return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export function displayTitle(track: {
	title: string | null;
	path: string;
}): string {
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

	// 1-5: star count (Rekordbox master.db). Otherwise: 0-255 (ID3 POPM / XML).
	const stars = Math.max(
		1,
		Math.min(
			5,
			rating <= 5 ? Math.round(rating) : Math.round((rating / 255) * 5),
		),
	);
	return `${"★".repeat(stars)}${"☆".repeat(5 - stars)}`;
}

export function displayValue(value: string | null): string {
	return value?.trim() ? value : "—";
}

/** Formats library `added_at` (Unix seconds). */
export function formatAddedAt(unixSeconds: number): string {
	if (!unixSeconds || unixSeconds <= 0) return "—";

	return new Date(unixSeconds * 1000).toLocaleString("ja-JP", {
		year: "numeric",
		month: "2-digit",
		day: "2-digit",
		hour: "2-digit",
		minute: "2-digit",
	});
}

const AUDIO_FORMAT_ALIASES: Record<string, string> = {
	aif: "AIFF",
	aiff: "AIFF",
	m4a: "AAC",
};

/** File extension label for converted-track badges. */
export function audioFormatLabel(path: string): string {
	const filename = path.split(/[/\\]/).pop() ?? "";
	const dot = filename.lastIndexOf(".");
	if (dot < 0 || dot === filename.length - 1) return "";
	const ext = filename.slice(dot + 1).toLowerCase();
	return AUDIO_FORMAT_ALIASES[ext] ?? ext.toUpperCase();
}
