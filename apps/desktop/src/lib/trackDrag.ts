/** プレイリストの重複要素を保持するため、ドラッグデータの順序と重複を維持する。 */
export const TRACK_DRAG_MIME = "application/x-catra-track-ids";

export const TRACK_PATHS_DRAG_MIME = "application/x-catra-track-paths";

export function writeTrackDrag(
	event: DragEvent,
	trackIds: number[],
	paths: string[] = [],
) {
	event.dataTransfer?.setData(TRACK_DRAG_MIME, JSON.stringify(trackIds));
	if (paths.length > 0) {
		event.dataTransfer?.setData(TRACK_PATHS_DRAG_MIME, JSON.stringify(paths));
	}
}

export function hasTrackPathsDrag(event: DragEvent): boolean {
	return !!event.dataTransfer?.types.includes(TRACK_PATHS_DRAG_MIME);
}

export function readTrackPathsDrag(event: DragEvent): string[] {
	const raw = event.dataTransfer?.getData(TRACK_PATHS_DRAG_MIME);
	if (!raw) return [];
	try {
		const parsed: unknown = JSON.parse(raw);
		if (!Array.isArray(parsed)) return [];
		return parsed.filter(
			(path): path is string =>
				typeof path === "string" && path.trim().length > 0,
		);
	} catch {
		return [];
	}
}

export function audioFilePaths(paths: string[]): string[] {
	return paths.filter((path) => /\.(mp3|wav|flac|aiff?|m4a|aac)$/i.test(path));
}

export function hasTrackDrag(event: DragEvent): boolean {
	return !!event.dataTransfer?.types.includes(TRACK_DRAG_MIME);
}

export function readTrackDrag(event: DragEvent): number[] {
	const raw = event.dataTransfer?.getData(TRACK_DRAG_MIME);
	if (!raw) return [];
	try {
		const parsed: unknown = JSON.parse(raw);
		if (!Array.isArray(parsed)) return [];
		return parsed.filter(
			(id): id is number => typeof id === "number" && Number.isInteger(id),
		);
	} catch {
		return [];
	}
}
