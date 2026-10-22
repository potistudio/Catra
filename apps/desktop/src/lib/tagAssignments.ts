import type { TrackTags } from "./types";

export type TagAssignmentIndex = ReadonlyMap<number, readonly number[]>;

export function mergeTagAssignments(
	current: TagAssignmentIndex,
	rows: readonly TrackTags[],
): TagAssignmentIndex {
	const next = new Map(current);
	for (const row of rows) next.set(row.trackId, [...row.tagIds]);
	return next;
}

export function summarizeTagAssignments(
	trackIds: readonly number[],
	assignments: TagAssignmentIndex,
): { ready: boolean; counts: ReadonlyMap<number, number> } {
	const ids = new Set(trackIds);
	const counts = new Map<number, number>();
	for (const id of ids) {
		const tags = assignments.get(id);
		if (!tags) return { ready: false, counts: new Map() };
		for (const tagId of new Set(tags)) {
			counts.set(tagId, (counts.get(tagId) ?? 0) + 1);
		}
	}
	return { ready: true, counts };
}
