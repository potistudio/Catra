import type { RekordboxContent } from "./types";

/** Normalize filesystem paths for Catra ↔ Rekordbox membership matching on Windows. */
export function normalizePath(path: string): string {
	return path.replace(/\//g, "\\").toLowerCase();
}

/** Map normalized FolderPath → Rekordbox content ID. */
export function buildRekordboxPathIndex(
	contents: RekordboxContent[],
): Map<string, string> {
	const index = new Map<string, string>();
	for (const content of contents) {
		index.set(normalizePath(content.folderPath), content.id);
	}
	return index;
}

export function buildRekordboxIdSet(contents: RekordboxContent[]): Set<string> {
	return new Set(contents.map((content) => content.id));
}

export function contentIdForPath(
	trackPath: string,
	index: Map<string, string>,
): string | undefined {
	return index.get(normalizePath(trackPath));
}

export function isInRekordbox(
	track: { path: string; rekordboxContentId?: string | null },
	index: Map<string, string>,
	contentIds?: Set<string>,
): boolean {
	if (track.rekordboxContentId) {
		if (contentIds) return contentIds.has(track.rekordboxContentId);
		for (const id of index.values()) {
			if (id === track.rekordboxContentId) return true;
		}
		return false;
	}
	return contentIdForPath(track.path, index) != null;
}
