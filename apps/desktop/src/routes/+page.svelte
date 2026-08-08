<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { pushActivityLog, pushActivityLogPayload } from "$lib/activityLog.svelte";
  import { listTracks, removeTrack, removeTracks, resolveDuplicate, scanFolder } from "$lib/api";
  import ActivityConsole from "$lib/components/ActivityConsole.svelte";
  import DuplicateTrackDialog from "$lib/components/DuplicateTrackDialog.svelte";
  import PreviewPlayer from "$lib/components/PreviewPlayer.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import type {
    ActivityLogPayload,
    DownloadProgress,
    DuplicateFoundPayload,
    ScanProgress,
    ScanResult,
    Track,
  } from "$lib/types";

  let tracks = $state<Track[]>([]);
  let selectedTrack = $state<Track | null>(null);
  let loading = $state(false);
  let scanning = $state(false);
  let duplicatePayload = $state<DuplicateFoundPayload | null>(null);
  let downloadProgress = $state<DownloadProgress | null>(null);

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

    scanning = true;
    pushActivityLog("info", "フォルダをスキャン中...", selected);

    try {
      await scanFolder(selected);
    } catch (e) {
      scanning = false;
      pushActivityLog("error", "スキャンに失敗しました", String(e));
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

  async function handleBulkRemove(ids: number[]) {
    try {
      const count = await removeTracks(ids);
      if (selectedTrack && ids.includes(selectedTrack.id)) {
        selectedTrack = null;
      }
      await loadTracks();
      pushActivityLog("success", `${count} 曲をライブラリから削除しました`);
    } catch (e) {
      pushActivityLog("error", "トラックの一括削除に失敗しました", String(e));
    }
  }

  async function handleDuplicateChoice(choice: "existing" | "new") {
    try {
      await resolveDuplicate(choice);
      duplicatePayload = null;
      pushActivityLog(
        "info",
        choice === "existing" ? "既存のトラックを残しました" : "新しいトラックをライブラリに追加しました",
      );
    } catch (e) {
      pushActivityLog("error", "重複の解決に失敗しました", String(e));
    }
  }

  onMount(() => {
    pushActivityLog("info", "Catra を起動しました");
    void loadTracks(false);

    let unlistenUpdated: (() => void) | undefined;
    let unlistenActivity: (() => void) | undefined;
    let unlistenScanComplete: (() => void) | undefined;
    let unlistenScanError: (() => void) | undefined;
    let unlistenScanProgress: (() => void) | undefined;
    let unlistenDuplicate: (() => void) | undefined;
    let unlistenDownloadProgress: (() => void) | undefined;

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

    void listen<ScanResult>("library-scan-complete", () => {
      scanning = false;
    }).then((unlisten) => {
      unlistenScanComplete = unlisten;
    });

    void listen<string>("library-scan-error", (event) => {
      scanning = false;
      pushActivityLog("error", "スキャンに失敗しました", event.payload);
    }).then((unlisten) => {
      unlistenScanError = unlisten;
    });

    void listen<ScanProgress>("library-scan-progress", () => {
      // Progress events are available for future UI; avoid flooding the activity log.
    }).then((unlisten) => {
      unlistenScanProgress = unlisten;
    });

    void listen<DuplicateFoundPayload>("library-duplicate-found", (event) => {
      duplicatePayload = event.payload;
    }).then((unlisten) => {
      unlistenDuplicate = unlisten;
    });

    void listen<DownloadProgress>("download-progress", (event) => {
      const progress = event.payload;
      if (progress.status === "idle" && progress.percent == null && !progress.message) {
        downloadProgress = null;
        return;
      }

      downloadProgress = progress;
    }).then((unlisten) => {
      unlistenDownloadProgress = unlisten;
    });

    return () => {
      unlistenUpdated?.();
      unlistenActivity?.();
      unlistenScanComplete?.();
      unlistenScanError?.();
      unlistenScanProgress?.();
      unlistenDuplicate?.();
      unlistenDownloadProgress?.();
    };
  });
</script>

{#if duplicatePayload}
  <DuplicateTrackDialog payload={duplicatePayload} onchoose={handleDuplicateChoice} />
{/if}

<div class="app">
  <header class="toolbar">
    <h1 class="logo">Catra</h1>
    {#if downloadProgress}
      <div class="download-status" aria-live="polite">
        <span class="download-message">
          {downloadProgress.message ?? "ダウンロード中"}
        </span>
        {#if downloadProgress.percent != null}
          <span class="download-percent">{Math.round(downloadProgress.percent)}%</span>
        {:else if downloadProgress.current != null && downloadProgress.total != null}
          <span class="download-percent">
            {downloadProgress.current}/{downloadProgress.total}
          </span>
        {/if}
        {#if downloadProgress.percent != null}
          <progress
            class="download-bar"
            max="100"
            value={Math.round(downloadProgress.percent)}
          ></progress>
        {/if}
      </div>
    {/if}
    <button class="btn primary" onclick={handleAddFolder} disabled={loading || scanning}>
      Add Folder
    </button>
  </header>

  <main class="content">
    <TrackList
      {tracks}
      selectedId={selectedTrack?.id ?? null}
      onselect={handleSelect}
      onremove={handleRemove}
      onbulkremove={handleBulkRemove}
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
    flex-shrink: 0;
  }

  .download-status {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    min-width: 0;
    flex: 1;
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
  }

  .download-message {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .download-percent {
    flex-shrink: 0;
    font-size: 0.8rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }

  .download-bar {
    width: 120px;
    height: 6px;
    flex-shrink: 0;
    border: none;
    border-radius: 999px;
    overflow: hidden;
    background: var(--surface-hover);
    accent-color: var(--accent);
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
