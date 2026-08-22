import { invoke } from "@tauri-apps/api/core";
import type {
	ConvertOptions,
	DuplicateChoice,
	FacetField,
	FacetValue,
	HealthReport,
	PlaylistDeleteImpact,
	PlaylistEntry,
	PlaylistKind,
	PlaylistNode,
	RekordboxCheck,
	RekordboxContent,
	RekordboxContentUpdate,
	RekordboxDbStatus,
	RekordboxPlaylist,
	Rule,
	SortRule,
	Tag,
	TagAxis,
	TagSelection,
	Track,
	TrackTags,
} from "./types";

export type {
	RekordboxCheck,
	RekordboxContent,
	RekordboxContentUpdate,
	RekordboxDbStatus,
	RekordboxPlaylist,
};

export async function listTracks(): Promise<Track[]> {
	return invoke<Track[]>("library_list_tracks");
}

export async function listTrashed(): Promise<Track[]> {
	return invoke<Track[]>("library_list_trashed");
}

export async function scanFolder(folder: string): Promise<void> {
	return invoke("library_scan_folder", { folder });
}

export async function importFromRekordbox(): Promise<void> {
	return invoke("library_import_from_rekordbox");
}

export async function importPaths(paths: string[]): Promise<void> {
	return invoke("library_import_paths", { paths });
}

export async function removeTrack(id: number): Promise<void> {
	return invoke("library_remove_track", { id });
}

export async function removeTracks(ids: number[]): Promise<number> {
	return invoke<number>("library_remove_tracks", { ids });
}

export async function restoreTracks(ids: number[]): Promise<number> {
	return invoke<number>("library_restore_tracks", { ids });
}

export async function deleteTracksPermanently(ids: number[]): Promise<number> {
	return invoke<number>("library_delete_permanently", { ids });
}

export async function emptyTrash(): Promise<number> {
	return invoke<number>("library_empty_trash");
}

export async function checkLibraryHealth(): Promise<HealthReport> {
	return invoke<HealthReport>("library_check_health");
}

export async function libraryAddToRekordbox(id: number): Promise<void> {
	return invoke("library_add_to_rekordbox", { id });
}

export async function libraryRemoveFromRekordbox(id: number): Promise<boolean> {
	return invoke<boolean>("library_remove_from_rekordbox", { id });
}

export async function resolveDuplicate(choice: DuplicateChoice): Promise<void> {
	return invoke("library_resolve_duplicate", { choice });
}

export async function convertTracks(
	ids: number[],
	options: ConvertOptions,
): Promise<void> {
	return invoke("library_convert_tracks", { ids, options });
}

export async function rekordboxCheck(): Promise<RekordboxCheck> {
	return invoke<RekordboxCheck>("rekordbox_check");
}

export async function rekordboxDbStatus(): Promise<RekordboxDbStatus> {
	return invoke<RekordboxDbStatus>("rekordbox_db_status");
}

export async function rekordboxGetContent(
	id?: string,
): Promise<RekordboxContent[]> {
	return invoke<RekordboxContent[]>("rekordbox_get_content", { id });
}

export async function rekordboxListPlaylists(): Promise<RekordboxPlaylist[]> {
	return invoke<RekordboxPlaylist[]>("rekordbox_list_playlists");
}

export async function rekordboxGetPlaylistContent(
	playlistId: string,
): Promise<RekordboxContent[]> {
	return invoke<RekordboxContent[]>("rekordbox_get_playlist_content", {
		playlistId,
	});
}

export async function rekordboxCreatePlaylist(
	name: string,
	parentId?: string | null,
): Promise<RekordboxPlaylist> {
	return invoke<RekordboxPlaylist>("rekordbox_create_playlist", {
		name,
		parentId: parentId ?? null,
	});
}

export async function rekordboxCreatePlaylistFolder(
	name: string,
	parentId?: string | null,
): Promise<RekordboxPlaylist> {
	return invoke<RekordboxPlaylist>("rekordbox_create_playlist_folder", {
		name,
		parentId: parentId ?? null,
	});
}

export async function rekordboxRenamePlaylist(
	id: string,
	name: string,
): Promise<RekordboxPlaylist> {
	return invoke<RekordboxPlaylist>("rekordbox_rename_playlist", { id, name });
}

export async function rekordboxDeletePlaylist(id: string): Promise<void> {
	return invoke("rekordbox_delete_playlist", { id });
}

export async function rekordboxAddToPlaylist(
	playlistId: string,
	contentId: string,
	trackNo?: number | null,
): Promise<string> {
	return invoke<string>("rekordbox_add_to_playlist", {
		playlistId,
		contentId,
		trackNo: trackNo ?? null,
	});
}

export async function rekordboxRemoveFromPlaylist(
	playlistId: string,
	songPlaylistId: string,
): Promise<void> {
	return invoke("rekordbox_remove_from_playlist", {
		playlistId,
		songPlaylistId,
	});
}

export async function rekordboxMoveSongInPlaylist(
	playlistId: string,
	songPlaylistId: string,
	newTrackNo: number,
): Promise<void> {
	return invoke("rekordbox_move_song_in_playlist", {
		playlistId,
		songPlaylistId,
		newTrackNo,
	});
}

export async function rekordboxAddContent(
	path: string,
	title?: string | null,
): Promise<RekordboxContent> {
	return invoke<RekordboxContent>("rekordbox_add_content", {
		path,
		title: title ?? null,
	});
}

export async function rekordboxUpdateContent(
	id: string,
	fields: RekordboxContentUpdate,
): Promise<RekordboxContent> {
	return invoke<RekordboxContent>("rekordbox_update_content", { id, fields });
}

export async function rekordboxDeleteContent(id: string): Promise<void> {
	return invoke("rekordbox_delete_content", { id });
}

// ---------------------------------------------------------------- 分類（プレイリストとタグ）

export async function playlistListTree(): Promise<PlaylistNode[]> {
	return invoke<PlaylistNode[]>("playlist_list_tree");
}

export async function playlistCreate(
	name: string,
	parentId: number | null,
	kind: PlaylistKind,
): Promise<PlaylistNode> {
	return invoke<PlaylistNode>("playlist_create", { name, parentId, kind });
}

export async function playlistRename(
	id: number,
	name: string,
): Promise<PlaylistNode> {
	return invoke<PlaylistNode>("playlist_rename", { id, name });
}

export async function playlistMove(
	id: number,
	parentId: number | null,
	position: number,
): Promise<void> {
	return invoke("playlist_move", { id, parentId, position });
}

export async function playlistDeleteImpact(
	id: number,
): Promise<PlaylistDeleteImpact> {
	return invoke<PlaylistDeleteImpact>("playlist_delete_impact", { id });
}

export async function playlistDelete(id: number): Promise<number> {
	return invoke<number>("playlist_delete", { id });
}

export async function playlistSetRule(
	id: number,
	rule: Rule | null,
	sortRule: SortRule | null,
): Promise<PlaylistNode> {
	return invoke<PlaylistNode>("playlist_set_rule", { id, rule, sortRule });
}

export async function playlistEntries(id: number): Promise<PlaylistEntry[]> {
	return invoke<PlaylistEntry[]>("playlist_entries", { id });
}

/** 既に入っている曲でも黙って足す。列なので重複は正当な操作である。 */
export async function playlistAddTracks(
	playlistId: number,
	trackIds: number[],
	position: number | null = null,
): Promise<number[]> {
	return invoke<number[]>("playlist_add_tracks", {
		playlistId,
		trackIds,
		position,
	});
}

/** この要素だけ消す。同じ曲の別の要素は残る。 */
export async function playlistRemoveEntries(
	playlistId: number,
	entryIds: number[],
): Promise<number> {
	return invoke<number>("playlist_remove_entries", { playlistId, entryIds });
}

/** 同じ曲の要素を全部消す。 */
export async function playlistRemoveTracks(
	playlistId: number,
	trackIds: number[],
): Promise<number> {
	return invoke<number>("playlist_remove_tracks", { playlistId, trackIds });
}

/** 要素 ID の完全な配列を渡す。トラック ID では列を指せない。 */
export async function playlistReorderEntries(
	playlistId: number,
	entryIds: number[],
): Promise<void> {
	return invoke("playlist_reorder_entries", { playlistId, entryIds });
}

export async function tagListAxes(): Promise<TagAxis[]> {
	return invoke<TagAxis[]>("tag_list_axes");
}

export async function tagCreateAxis(
	name: string,
	selection: TagSelection,
): Promise<TagAxis> {
	return invoke<TagAxis>("tag_create_axis", { name, selection });
}

export async function tagUpdateAxis(
	id: number,
	name: string | null,
	selection: TagSelection | null,
): Promise<TagAxis> {
	return invoke<TagAxis>("tag_update_axis", { id, name, selection });
}

/** multi から single に落とす前に、はみ出す曲が何曲あるかを訊く。 */
export async function tagAxisConflicts(id: number): Promise<number> {
	return invoke<number>("tag_axis_conflicts", { id });
}

export async function tagDeleteAxis(id: number): Promise<number> {
	return invoke<number>("tag_delete_axis", { id });
}

export async function tagCreate(axisId: number, name: string): Promise<Tag> {
	return invoke<Tag>("tag_create", { axisId, name });
}

export async function tagRename(id: number, name: string): Promise<Tag> {
	return invoke<Tag>("tag_rename", { id, name });
}

export async function tagDelete(id: number): Promise<void> {
	return invoke("tag_delete", { id });
}

export async function tagAssign(
	trackIds: number[],
	tagIds: number[],
): Promise<void> {
	return invoke("tag_assign", { trackIds, tagIds });
}

export async function tagUnassign(
	trackIds: number[],
	tagIds: number[],
): Promise<number> {
	return invoke<number>("tag_unassign", { trackIds, tagIds });
}

export async function tagOfTracks(trackIds: number[]): Promise<TrackTags[]> {
	return invoke<TrackTags[]>("tag_of_tracks", { trackIds });
}

/** タグでの絞り込み。複数のタグは AND で効く。 */
export async function browseTagTracks(tagIds: number[]): Promise<Track[]> {
	return invoke<Track[]>("browse_tag_tracks", { tagIds });
}

export async function browseFacet(field: FacetField): Promise<FacetValue[]> {
	return invoke<FacetValue[]>("browse_facet", { field });
}

export async function browseFacetTracks(
	field: FacetField,
	value: string | null,
): Promise<Track[]> {
	return invoke<Track[]>("browse_facet_tracks", { field, value });
}
