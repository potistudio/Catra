<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { pushActivityLog, pushActivityLogPayload } from "$lib/activityLog.svelte";
  import { listTracks, removeTrack, scanFolder } from "$lib/api";
  import ActivityConsole from "$lib/components/ActivityConsole.svelte";
  import PreviewPlayer from "$lib/components/PreviewPlayer.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import type { ActivityLogPayload, Track } from "$lib/types";

  let tracks = $state<Track[]>([]);
  let selectedTrack = $state<Track | null>(null);
  let loading = $state(false);

  async function loadTracks(silent = true) {
    loading = true;
    try {
      tracks = await listTracks();
      if (selectedTrack) {
        const updated = tracks.find((t) => t.id === selectedTrack!.id);
        selectedTrack = updated ?? null;
      }
      if (!silent) {
        pushActivityLog("info", `ライブラリを読み込みました (${tracks.length} 曲)`);
      }
    } catch (e) {
      pushActivityLog("error", "ライブラリの読み込みに失敗しました", String(e));
    } finally {
      loading = false;
    }
  }

  async function handleAddFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select music folder",
    });

    if (!selected || typeof selected !== "string") return;

    loading = true;
    pushActivityLog("info", "フォルダをスキャン中...", selected);

    try {
      const result = await scanFolder(selected);
      pushActivityLog(
        "success",
        `スキャン完了: ${result.added} 曲を追加 (${result.skipped} 曲は既存)`,
        selected,
      );
      await loadTracks();
    } catch (e) {
      pushActivityLog("error", "スキャンに失敗しました", String(e));
    } finally {
      loading = false;
    }
  }

  function handleSelect(track: Track) {
    selectedTrack = track;
  }

  async function handleRemove(track: Track) {
    try {
      await removeTrack(track.id);
      if (selectedTrack?.id === track.id) {
        selectedTrack = null;
      }
      await loadTracks();
      pushActivityLog(
        "success",
        `ライブラリから削除: ${track.title ?? track.path}`,
        track.path,
      );
    } catch (e) {
      pushActivityLog("error", "トラックの削除に失敗しました", String(e));
    }
  }

  onMount(() => {
    pushActivityLog("info", "Catra を起動しました");
    void loadTracks(false);

    let unlistenUpdated: (() => void) | undefined;
    let unlistenActivity: (() => void) | undefined;

    void listen("library-updated", () => {
      void loadTracks();
    }).then((unlisten) => {
      unlistenUpdated = unlisten;
    });

    void listen<ActivityLogPayload>("activity-log", (event) => {
      pushActivityLogPayload(event.payload);
    }).then((unlisten) => {
      unlistenActivity = unlisten;
    });

    return () => {
      unlistenUpdated?.();
      unlistenActivity?.();
    };
  });
</script>

<div class="app">
  <header class="toolbar">
    <h1 class="logo">Catra</h1>
    <button class="btn primary" onclick={handleAddFolder} disabled={loading}>
      Add Folder
    </button>
  </header>

  <main class="content">
    <TrackList
      {tracks}
      selectedId={selectedTrack?.id ?? null}
      onselect={handleSelect}
      onremove={handleRemove}
    />
  </main>

  <ActivityConsole />
  <PreviewPlayer track={selectedTrack} />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .logo {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .btn {
    padding: 0.45rem 1rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .content {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
