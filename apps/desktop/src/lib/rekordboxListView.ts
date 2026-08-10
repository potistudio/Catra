import type { PreviewableTrack, RekordboxContent, Track } from "$lib/types";

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
