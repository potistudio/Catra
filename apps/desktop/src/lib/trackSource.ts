import type { TrackSource } from "$lib/types";

export function isTrackSource(
	value: string | null | undefined,
): value is TrackSource {
	return value === "soundcloud" || value === "bandcamp";
}
