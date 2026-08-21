import { displayTitle } from "$lib/format";
import type { Track } from "$lib/types";

export const TRACK_ROW_HEIGHT = 48;
export const TRACK_GRID_MIN_CARD_WIDTH = 148;
export const TRACK_GRID_GAP = 12;
export const TRACK_GRID_ROW_HEIGHT = 196;
export const VIRTUAL_OVERSCAN = 12;

export type ViewMode = "list" | "grid";

export type SortColumn =
  | "position"
  | "title"
  | "artist"
  | "album"
  | "bpm"
  | "bitrateKbps"
  | "key"
  | "genre"
  | "rating"
  | "durationMs"
  | "addedAt";

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

/**
 * 一覧の1行。静的プレイリストは**列**なので、同じ曲が2回現れる。
 * そのとき `key` は要素 ID になり、2つの行は別物として扱われる。
 * 列でない一覧（ライブラリ、スマート、フォルダ）では `entryId` は null で、
 * `key` はトラック ID。
 */
export interface TrackListRow {
  key: number;
  entryId: number | null;
  position: number;
  track: Track;
}

/** 列を持たない一覧を行に包む。並び順がそのまま position になる。 */
export function rowsFromTracks(tracks: Track[]): TrackListRow[] {
  return tracks.map((track, index) => ({
    key: track.id,
    entryId: null,
    position: index,
    track,
  }));
}

function sortKeyFor(row: TrackListRow, column: SortColumn): SortKey {
  const track = row.track;
  switch (column) {
    case "position":
      return row.position;
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
    case "addedAt":
      return track.addedAt > 0 ? track.addedAt : Number.POSITIVE_INFINITY;
  }
}

export function filterRows(rows: TrackListRow[], query: string): TrackListRow[] {
  const trimmed = query.trim();
  if (!trimmed) return rows;

  const q = trimmed.toLowerCase();
  return rows.filter(({ track }) => {
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

export function sortRows(
  items: TrackListRow[],
  column: SortColumn,
  direction: SortDirection,
): TrackListRow[] {
  if (items.length <= 1) return items;

  const mult = direction === "asc" ? 1 : -1;
  const decorated = items.map((row) => ({
    row,
    key: sortKeyFor(row, column),
    titleKey: displayTitle(row.track),
  }));

  decorated.sort((a, b) => {
    const cmp = compareSortKeys(a.key, b.key);
    if (cmp !== 0) return cmp * mult;
    // 同点は位置で決める。列の中で同じ曲が2回あっても行が入れ替わらない。
    const tie = compareStrings(a.titleKey, b.titleKey);
    if (tie !== 0) return tie * mult;
    return a.row.position - b.row.position;
  });

  return decorated.map(({ row }) => row);
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
