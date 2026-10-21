type SelectionModifiers = Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">;

export function getCheckedTracks<T extends { id: number }>(
	rows: ReadonlyArray<{ key: number; track: T }>,
	checkedKeys: ReadonlySet<number>,
): T[] {
	const seen = new Set<number>();
	const tracks: T[] = [];
	for (const { key, track } of rows) {
		if (!checkedKeys.has(key) || seen.has(track.id)) continue;
		seen.add(track.id);
		tracks.push(track);
	}
	return tracks;
}

export function selectTrackKeys(
	visibleKeys: number[],
	checkedKeys: ReadonlySet<number>,
	key: number,
	anchorKey: number | null,
	event?: SelectionModifiers,
): Set<number> {
	const additive = event?.ctrlKey || event?.metaKey;
	const anchorIndex =
		event?.shiftKey && anchorKey != null ? visibleKeys.indexOf(anchorKey) : -1;
	const keyIndex = visibleKeys.indexOf(key);

	if (anchorIndex >= 0 && keyIndex >= 0) {
		const next = new Set(additive ? checkedKeys : []);
		for (const rangeKey of visibleKeys.slice(
			Math.min(anchorIndex, keyIndex),
			Math.max(anchorIndex, keyIndex) + 1,
		)) {
			next.add(rangeKey);
		}
		return next;
	}

	if (!additive) return new Set([key]);

	const next = new Set(checkedKeys);
	if (next.has(key)) {
		next.delete(key);
	} else {
		next.add(key);
	}
	return next;
}
