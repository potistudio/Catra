/**
 * 曲リストから外へ曲を運ぶときの荷札。
 * 中身はトラック ID の配列で、重複はそのまま持っていく。
 * 列に落とすなら、同じ曲が2回入っているのは正当な操作である。
 */
export const TRACK_DRAG_MIME = "application/x-catra-track-ids";

export function writeTrackDrag(event: DragEvent, trackIds: number[]) {
  event.dataTransfer?.setData(TRACK_DRAG_MIME, JSON.stringify(trackIds));
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
    return parsed.filter((id): id is number => typeof id === "number" && Number.isInteger(id));
  } catch {
    return [];
  }
}
