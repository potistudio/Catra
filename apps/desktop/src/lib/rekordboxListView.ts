import type {
  PreviewableTrack,
  RekordboxContent,
  RekordboxPlaylist,
  Track,
} from "$lib/types";

/** Map Rekordbox rows onto Track for shared list/grid UI. `id` is the source array index. */
export function rekordboxContentToTrack(
  content: RekordboxContent,
  index: number,
): Track {
  return {
    id: index,
    path: content.folderPath,
    title: content.title,
    artist: content.artist,
    album: content.album,
    durationMs:
      content.lengthSecs != null ? content.lengthSecs * 1000 : null,
    bpm: content.bpm,
    bitrateKbps: content.bitRate,
    genre: content.genre,
    key: content.key,
    rating: content.rating,
    artworkPath: content.artworkPath,
    source: null,
    converted: false,
    addedAt: 0,
  };
}

export function rekordboxContentToPreview(content: RekordboxContent): PreviewableTrack {
  return {
    path: content.folderPath,
    title: content.title,
    artist: content.artist,
    album: content.album,
    durationMs:
      content.lengthSecs != null ? content.lengthSecs * 1000 : null,
    bpm: content.bpm,
    bitrateKbps: content.bitRate,
    genre: content.genre,
    key: content.key,
    rating: content.rating,
    artworkPath: content.artworkPath,
  };
}

export function isPlaylistFolder(playlist: RekordboxPlaylist): boolean {
  return playlist.attribute === 1;
}

/** Depth-first walk in Seq order for indented sidebar rows. */
export function playlistTreeRows(
  playlists: RekordboxPlaylist[],
): Array<RekordboxPlaylist & { depth: number }> {
  const byParent = new Map<string | null, RekordboxPlaylist[]>();
  for (const playlist of playlists) {
    const key = playlist.parentId;
    const siblings = byParent.get(key);
    if (siblings) siblings.push(playlist);
    else byParent.set(key, [playlist]);
  }

  const rows: Array<RekordboxPlaylist & { depth: number }> = [];

  function walk(parentId: string | null, depth: number) {
    const children = byParent.get(parentId);
    if (!children) return;
    for (const child of children) {
      rows.push({ ...child, depth });
      walk(child.id, depth + 1);
    }
  }

  walk(null, 0);
  return rows;
}
