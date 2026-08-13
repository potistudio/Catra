import { invoke } from "@tauri-apps/api/core";
import type {
  ConvertOptions,
  DuplicateChoice,
  RekordboxCheck,
  RekordboxContent,
  RekordboxContentUpdate,
  RekordboxDbStatus,
  RekordboxPlaylist,
  Track,
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

export async function scanFolder(folder: string): Promise<void> {
  return invoke("library_scan_folder", { folder });
}

export async function importFromRekordbox(): Promise<void> {
  return invoke("library_import_from_rekordbox");
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
