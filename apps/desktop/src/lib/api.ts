import { invoke } from "@tauri-apps/api/core";
import type { ScanResult, Track } from "./types";

export async function listTracks(): Promise<Track[]> {
  return invoke<Track[]>("library_list_tracks");
}

export async function scanFolder(folder: string): Promise<ScanResult> {
  return invoke<ScanResult>("library_scan_folder", { folder });
}

export async function removeTrack(id: number): Promise<void> {
  return invoke("library_remove_track", { id });
}

export async function removeTracks(ids: number[]): Promise<number> {
  return invoke<number>("library_remove_tracks", { ids });
}
