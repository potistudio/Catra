import { displayTitle } from "$lib/format";
import type { Track } from "$lib/types";

export const TRACK_ROW_HEIGHT = 48;
export const TRACK_GRID_MIN_CARD_WIDTH = 148;
export const TRACK_GRID_GAP = 12;
export const TRACK_GRID_ROW_HEIGHT = 196;
export const VIRTUAL_OVERSCAN = 12;

export type ViewMode = "list" | "grid";

export type SortColumn =
  | "title"
  | "artist"
  | "album"
  | "bpm"
  | "bitrateKbps"
  | "key"
  | "genre"
  | "rating"
  | "durationMs";

export type SortDirection = "asc" | "desc";

type SortKey = string | number;

function compareStrings(a: string | null, b: string | null): number {
  const aVal = a?.trim() ?? "";
  const bVal = b?.trim() ?? "";
  if (!aVal && !bVal) return 0;
  if (!aVal) return 1;
  if (!bVal) return -1;
  return aVal.localeCompare(bVal, undefined, { sensitivity: "base", numeric: true });
}

function compareSortKeys(a: SortKey, b: SortKey): number {
  if (typeof a === "number" && typeof b === "number") {
    if (a === b) return 0;
    return a < b ? -1 : 1;
  }

  return compareStrings(String(a), String(b));
}

function sortKeyFor(track: Track, column: SortColumn): SortKey {
  switch (column) {
    case "title":
      return displayTitle(track);
    case "artist":
      return track.artist?.trim() ?? "";
    case "album":
      return track.album?.trim() ?? "";
    case "bpm":
      return track.bpm ?? Number.POSITIVE_INFINITY;
    case "bitrateKbps":
      return track.bitrateKbps ?? Number.POSITIVE_INFINITY;
    case "key":
      return track.key?.trim() ?? "";
    case "genre":
      return track.genre?.trim() ?? "";
    case "rating":
      return track.rating ?? -1;
    case "durationMs":
      return track.durationMs ?? Number.POSITIVE_INFINITY;
  }
}

export function filterTracks(tracks: Track[], query: string): Track[] {
  const trimmed = query.trim();
  if (!trimmed) return tracks;

  const q = trimmed.toLowerCase();
  return tracks.filter((track) => {
    const fields = [
      track.title,
      track.artist,
      track.album,
      track.genre,
      track.key,
      track.path,
    ];
    return fields.some((field) => (field ?? "").toLowerCase().includes(q));
  });
}

export function sortTracks(
  items: Track[],
  column: SortColumn,
  direction: SortDirection,
): Track[] {
  if (items.length <= 1) return items;

  const mult = direction === "asc" ? 1 : -1;
  const decorated = items.map((track) => ({
    track,
    key: sortKeyFor(track, column),
    titleKey: displayTitle(track),
  }));

  decorated.sort((a, b) => {
    const cmp = compareSortKeys(a.key, b.key);
    if (cmp !== 0) return cmp * mult;
    return compareStrings(a.titleKey, b.titleKey) * mult;
  });

  return decorated.map(({ track }) => track);
}

export function getVisibleTrackRange(
  scrollTop: number,
  viewportHeight: number,
  totalCount: number,
): { start: number; end: number } {
  if (totalCount === 0) return { start: 0, end: 0 };

  const start = Math.max(
    0,
    Math.floor(scrollTop / TRACK_ROW_HEIGHT) - VIRTUAL_OVERSCAN,
  );
  const visibleCount =
    Math.ceil(viewportHeight / TRACK_ROW_HEIGHT) + VIRTUAL_OVERSCAN * 2;
  const end = Math.min(totalCount, start + visibleCount);

  return { start, end };
}

export function getGridColumnCount(containerWidth: number): number {
  if (containerWidth <= 0) return 1;
  return Math.max(
    1,
    Math.floor(
      (containerWidth + TRACK_GRID_GAP) /
        (TRACK_GRID_MIN_CARD_WIDTH + TRACK_GRID_GAP),
    ),
  );
}

export function getVisibleGridRange(
  scrollTop: number,
  viewportHeight: number,
  columnCount: number,
  totalCount: number,
): { start: number; end: number; startRow: number } {
  if (totalCount === 0 || columnCount <= 0) {
    return { start: 0, end: 0, startRow: 0 };
  }

  const rowCount = Math.ceil(totalCount / columnCount);
  const startRow = Math.max(
    0,
    Math.floor(scrollTop / TRACK_GRID_ROW_HEIGHT) - VIRTUAL_OVERSCAN,
  );
  const visibleRows =
    Math.ceil(viewportHeight / TRACK_GRID_ROW_HEIGHT) + VIRTUAL_OVERSCAN * 2;
  const endRow = Math.min(rowCount, startRow + visibleRows);

  return {
    start: startRow * columnCount,
    end: Math.min(totalCount, endRow * columnCount),
    startRow,
  };
}
