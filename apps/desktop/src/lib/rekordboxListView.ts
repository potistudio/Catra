import type { PreviewableTrack, RekordboxContent } from "$lib/types";

export type RekordboxSortColumn =
  | "title"
  | "artist"
  | "album"
  | "bpm"
  | "bitRate"
  | "key"
  | "genre"
  | "rating"
  | "lengthSecs";

export type SortDirection = "asc" | "desc";

export function filterRekordboxContent(
  tracks: RekordboxContent[],
  query: string,
): RekordboxContent[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return tracks;

  return tracks.filter((track) => {
    const haystack = [
      track.title,
      track.artist,
      track.album,
      track.genre,
      track.key,
      track.remixer,
      track.label,
      track.composer,
      track.folderPath,
      track.fileName,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();

    return haystack.includes(normalized);
  });
}

function compareNullableStrings(
  left: string | null,
  right: string | null,
  direction: SortDirection,
): number {
  const a = (left ?? "").toLocaleLowerCase();
  const b = (right ?? "").toLocaleLowerCase();
  const result = a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });
  return direction === "asc" ? result : -result;
}

function compareNullableNumbers(
  left: number | null,
  right: number | null,
  direction: SortDirection,
): number {
  const a = left ?? -1;
  const b = right ?? -1;
  const result = a - b;
  return direction === "asc" ? result : -result;
}

export function sortRekordboxContent(
  tracks: RekordboxContent[],
  column: RekordboxSortColumn,
  direction: SortDirection,
): RekordboxContent[] {
  const sorted = [...tracks];

  sorted.sort((left, right) => {
    switch (column) {
      case "title":
        return compareNullableStrings(left.title, right.title, direction);
      case "artist":
        return compareNullableStrings(left.artist, right.artist, direction);
      case "album":
        return compareNullableStrings(left.album, right.album, direction);
      case "bpm":
        return compareNullableNumbers(left.bpm, right.bpm, direction);
      case "bitRate":
        return compareNullableNumbers(left.bitRate, right.bitRate, direction);
      case "key":
        return compareNullableStrings(left.key, right.key, direction);
      case "genre":
        return compareNullableStrings(left.genre, right.genre, direction);
      case "rating":
        return compareNullableNumbers(left.rating, right.rating, direction);
      case "lengthSecs":
        return compareNullableNumbers(left.lengthSecs, right.lengthSecs, direction);
    }
  });

  return sorted;
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
    artworkPath: null,
  };
}
