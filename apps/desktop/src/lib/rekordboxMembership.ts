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

export function contentIdForPath(
  trackPath: string,
  index: Map<string, string>,
): string | undefined {
  return index.get(normalizePath(trackPath));
}

export function isInRekordbox(
  trackPath: string,
  index: Map<string, string>,
): boolean {
  return contentIdForPath(trackPath, index) != null;
}
