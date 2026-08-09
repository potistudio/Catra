import { invoke } from "@tauri-apps/api/core";
import type { DuplicateChoice, Track } from "./types";

export async function listTracks(): Promise<Track[]> {
  return invoke<Track[]>("library_list_tracks");
}

export async function scanFolder(folder: string): Promise<void> {
  return invoke("library_scan_folder", { folder });
}

export async function removeTrack(id: number): Promise<void> {
  return invoke("library_remove_track", { id });
}

export async function removeTracks(ids: number[]): Promise<number> {
  return invoke<number>("library_remove_tracks", { ids });
}

export async function resolveDuplicate(choice: DuplicateChoice): Promise<void> {
  return invoke("library_resolve_duplicate", { choice });
}

export interface RekordboxCheck {
  dbPath: string | null;
  analysisRoot: string | null;
  settingsRoot: string | null;
  installDir: string | null;
  version: string | null;
  rekordboxRunning: boolean;
}

export interface RekordboxDbStatus {
  trackCount: number;
  playlistCount: number;
}

export async function rekordboxCheck(): Promise<RekordboxCheck> {
  return invoke<RekordboxCheck>("rekordbox_check");
}

export async function rekordboxDbStatus(): Promise<RekordboxDbStatus> {
  return invoke<RekordboxDbStatus>("rekordbox_db_status");
}
