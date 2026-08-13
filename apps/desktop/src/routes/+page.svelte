<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { pushActivityLog, pushActivityLogPayload } from "$lib/activityLog.svelte";
  import {
    importFromRekordbox,
    listTracks,
    rekordboxAddContent,
    rekordboxCheck,
    rekordboxDeleteContent,
    rekordboxGetContent,
    removeTrack,
    removeTracks,
    resolveDuplicate,
    scanFolder,
  } from "$lib/api";
  import ActivityConsole from "$lib/components/ActivityConsole.svelte";
  import DuplicateTrackDialog from "$lib/components/DuplicateTrackDialog.svelte";
  import PreviewPlayer from "$lib/components/PreviewPlayer.svelte";
  import RekordboxList from "$lib/components/RekordboxList.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import {
    buildRekordboxPathIndex,
    contentIdForPath,
  } from "$lib/rekordboxMembership";
  import { rekordboxContentToPreview } from "$lib/rekordboxListView";
  import type {
    ActivityLogPayload,
    DownloadProgress,
    DuplicateFoundPayload,
    PreviewableTrack,
    RekordboxCheck,
    RekordboxContent,
    ScanProgress,
    ScanResult,
    Track,
  } from "$lib/types";

  type AppTab = "library" | "rekordbox";

  /** Rekordbox may start or quit after the initial check; keep the write lock aligned. */
  const REKORDBOX_RUNNING_POLL_MS = 2000;

  let activeTab = $state<AppTab>("library");
  let tracks = $state<Track[]>([]);
  let selectedTrack = $state<Track | null>(null);
  let rekordboxTracks = $state<RekordboxContent[]>([]);
  let selectedRekordboxId = $state<string | null>(null);
  let rekordboxStatus = $state<RekordboxCheck | null>(null);
  let rekordboxLoading = $state(false);
  let rekordboxBusy = $state(false);
  let loading = $state(false);
  let scanning = $state(false);
  let scanKind = $state<"folder" | "rekordbox" | null>(null);
  let scanProgress = $state<ScanProgress | null>(null);
  let duplicatePayload = $state<DuplicateFoundPayload | null>(null);
  let downloadProgress = $state<DownloadProgress | null>(null);

  let scanStatusLabel = $derived.by((): string => {
    if (duplicatePayload) return "重複の確認待ち";
    if (!scanning) return "";
    if (scanKind === "rekordbox") return "Rekordbox からインポート中";
    if (scanKind === "folder") return "フォルダをスキャン中";
    return "ライブラリ取り込み中";
  });

  let scanStatusCount = $derived.by((): string | null => {
    if (!scanProgress) return null;
    if (scanProgress.total != null && scanProgress.total > 0) {
      return `${scanProgress.processed}/${scanProgress.total}`;
    }
    return `${scanProgress.processed}`;
  });

  let scanStatusDetail = $derived.by((): string | null => {
    if (!scanProgress) return null;
    return `追加 ${scanProgress.added} · スキップ ${scanProgress.skipped}`;
  });

  let rekordboxPathIndex = $derived(buildRekordboxPathIndex(rekordboxTracks));
  let rekordboxWritable = $derived(
    !!rekordboxStatus?.dbPath && !rekordboxStatus.rekordboxRunning,
  );
  let rekordboxLockedHint = $derived.by((): string | null => {
    if (!rekordboxStatus?.dbPath) {
      return "Rekordbox ライブラリが見つかりません";
    }
    if (rekordboxStatus.rekordboxRunning) {
      return "Rekordbox を終了すると編集できます";
    }
    return null;
  });

  let previewTrack = $derived.by((): PreviewableTrack | null => {
    if (activeTab === "library") {
      return selectedTrack;
    }

    if (!selectedRekordboxId) return null;
    const content = rekordboxTracks.find((track) => track.id === selectedRekordboxId);
    return content ? rekordboxContentToPreview(content) : null;
  });

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

  function rekordboxLockUnchanged(
    prev: RekordboxCheck | null,
    next: RekordboxCheck,
  ): boolean {
    return (
      prev != null &&
      prev.rekordboxRunning === next.rekordboxRunning &&
      prev.dbPath === next.dbPath
    );
  }

  async function refreshRekordboxStatus(): Promise<RekordboxCheck | null> {
    try {
      const next = await rekordboxCheck();
      const prev = rekordboxStatus;
      if (rekordboxLockUnchanged(prev, next)) {
        return prev;
      }

      rekordboxStatus = next;

      if (prev && prev.rekordboxRunning !== next.rekordboxRunning) {
        if (next.rekordboxRunning) {
          pushActivityLog("warning", "Rekordbox が起動したため編集をロックしました");
        } else if (next.dbPath) {
          pushActivityLog("info", "Rekordbox が終了したため編集できます");
        }
      }

      return next;
    } catch {
      return rekordboxStatus;
    }
  }

  async function loadRekordbox(silent = true) {
    rekordboxLoading = true;
    try {
      rekordboxStatus = await rekordboxCheck();
      rekordboxTracks = await rekordboxGetContent();
      if (selectedRekordboxId) {
        const updated = rekordboxTracks.find((track) => track.id === selectedRekordboxId);
        if (!updated) {
          selectedRekordboxId = null;
        }
      }
      if (!silent) {
        pushActivityLog(
          "info",
          `Rekordbox を読み込みました (${rekordboxTracks.length} 曲)`,
        );
      }
    } catch (e) {
      pushActivityLog("error", "Rekordbox の読み込みに失敗しました", String(e));
    } finally {
      rekordboxLoading = false;
    }
  }

  function switchTab(tab: AppTab) {
    activeTab = tab;
    if (tab === "rekordbox" && rekordboxTracks.length === 0 && !rekordboxLoading) {
      void loadRekordbox(false);
    }
  }

  function handleSelectRekordbox(track: RekordboxContent) {
    selectedRekordboxId = track.id;
  }

  async function ensureRekordboxWritable(): Promise<boolean> {
    await refreshRekordboxStatus();
    if (!rekordboxStatus?.dbPath) {
      pushActivityLog("warning", "Rekordbox ライブラリが見つかりません");
      return false;
    }
    if (rekordboxStatus.rekordboxRunning) {
      pushActivityLog(
        "warning",
        "Rekordbox が起動中です。終了してから編集してください",
      );
      return false;
    }
    return true;
  }

  async function handleAddToRekordbox(selected: Track[]) {
    if (selected.length === 0 || rekordboxBusy) return;
    if (!(await ensureRekordboxWritable())) return;

    rekordboxBusy = true;
    let added = 0;
    let skipped = 0;
    let failed = 0;
    try {
      for (const track of selected) {
        if (contentIdForPath(track.path, rekordboxPathIndex)) {
          skipped += 1;
          continue;
        }
        try {
          await rekordboxAddContent(track.path, track.title);
          added += 1;
        } catch (e) {
          failed += 1;
          pushActivityLog(
            "error",
            `Rekordbox への追加に失敗: ${track.title ?? track.path}`,
            String(e),
          );
        }
      }
      await loadRekordbox(true);
      if (added > 0) {
        pushActivityLog("success", `${added} 曲を Rekordbox に追加しました`);
      }
      if (skipped > 0) {
        pushActivityLog("info", `${skipped} 曲は既に Rekordbox に登録済みです`);
      }
      if (failed > 0 && added === 0) {
        pushActivityLog("error", `${failed} 曲の追加に失敗しました`);
      }
    } finally {
      rekordboxBusy = false;
    }
  }

  async function handleRemoveFromRekordbox(selected: Track[]) {
    if (selected.length === 0 || rekordboxBusy) return;
    if (!(await ensureRekordboxWritable())) return;

    rekordboxBusy = true;
    let removed = 0;
    let failed = 0;
    try {
      for (const track of selected) {
        const contentId = contentIdForPath(track.path, rekordboxPathIndex);
        if (!contentId) continue;
        try {
          await rekordboxDeleteContent(contentId);
          removed += 1;
        } catch (e) {
          failed += 1;
          pushActivityLog(
            "error",
            `Rekordbox からの削除に失敗: ${track.title ?? track.path}`,
            String(e),
          );
        }
      }
      await loadRekordbox(true);
      if (removed > 0) {
        pushActivityLog("success", `${removed} 曲を Rekordbox から削除しました`);
      }
      if (failed > 0 && removed === 0) {
        pushActivityLog("error", `${failed} 曲の削除に失敗しました`);
      }
    } finally {
      rekordboxBusy = false;
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
    scanKind = "folder";
    scanProgress = null;
    pushActivityLog("info", "フォルダをスキャン中...", selected);

    try {
      await scanFolder(selected);
    } catch (e) {
      scanning = false;
      scanKind = null;
      scanProgress = null;
      pushActivityLog("error", "スキャンに失敗しました", String(e));
    }
  }

  async function handleImportFromRekordbox() {
    if (!rekordboxStatus?.dbPath) {
      pushActivityLog("error", "Rekordbox ライブラリが見つかりません");
      return;
    }

    scanning = true;
    scanKind = "rekordbox";
    scanProgress = {
      processed: 0,
      added: 0,
      skipped: 0,
      currentPath: "",
      total: rekordboxTracks.length > 0 ? rekordboxTracks.length : null,
    };
    pushActivityLog("info", "Rekordbox からライブラリへインポート中...");

    try {
      await importFromRekordbox();
    } catch (e) {
      scanning = false;
      scanKind = null;
      scanProgress = null;
      pushActivityLog("error", "Rekordbox からのインポートに失敗しました", String(e));
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
    void loadRekordbox(true);

    let rekordboxPollInFlight = false;
    const rekordboxPollId = window.setInterval(() => {
      if (rekordboxPollInFlight || rekordboxLoading) return;
      rekordboxPollInFlight = true;
      void refreshRekordboxStatus().finally(() => {
        rekordboxPollInFlight = false;
      });
    }, REKORDBOX_RUNNING_POLL_MS);

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

    void listen<ScanResult>("library-scan-complete", (event) => {
      scanning = false;
      scanKind = null;
      scanProgress = null;
      if (event.payload.added > 0) {
        activeTab = "library";
      }
    }).then((unlisten) => {
      unlistenScanComplete = unlisten;
    });

    void listen<string>("library-scan-error", (event) => {
      scanning = false;
      scanKind = null;
      scanProgress = null;
      pushActivityLog("error", "ライブラリの取り込みに失敗しました", event.payload);
    }).then((unlisten) => {
      unlistenScanError = unlisten;
    });

    void listen<ScanProgress>("library-scan-progress", (event) => {
      // Clone into a new object so Svelte 5 always sees a state change.
      const next = event.payload;
      scanProgress = {
        processed: next.processed,
        added: next.added,
        skipped: next.skipped,
        currentPath: next.currentPath,
        total: next.total ?? null,
      };
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
      window.clearInterval(rekordboxPollId);
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
    <div class="tabs" role="tablist" aria-label="ライブラリ切替">
      <button
        type="button"
        class="tab"
        class:active={activeTab === "library"}
        role="tab"
        aria-selected={activeTab === "library"}
        onclick={() => switchTab("library")}
      >
        Library
      </button>
      <button
        type="button"
        class="tab"
        class:active={activeTab === "rekordbox"}
        role="tab"
        aria-selected={activeTab === "rekordbox"}
        onclick={() => switchTab("rekordbox")}
      >
        Rekordbox
      </button>
    </div>
    {#if scanning || duplicatePayload}
      <div class="download-status" aria-live="polite">
        <span class="download-message">
          {scanStatusLabel}
          {#if !duplicatePayload && scanProgress?.currentPath}
            — {scanProgress.currentPath}
          {/if}
        </span>
        {#if scanStatusDetail}
          <span class="download-percent">{scanStatusDetail}</span>
        {/if}
        {#if scanStatusCount}
          <span class="download-percent">{scanStatusCount}</span>
        {/if}
        {#if scanProgress?.total != null && scanProgress.total > 0}
          <progress
            class="download-bar"
            max={scanProgress.total}
            value={scanProgress.processed}
          ></progress>
        {/if}
      </div>
    {:else if downloadProgress}
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
    <button
      class="btn primary"
      onclick={handleAddFolder}
      disabled={loading || scanning || activeTab !== "library"}
    >
      {scanning && scanKind === "folder" ? "スキャン中..." : "Add Folder"}
    </button>
    <button
      class="btn primary"
      onclick={handleImportFromRekordbox}
      disabled={loading || scanning || activeTab !== "rekordbox" || !rekordboxStatus?.dbPath}
    >
      {scanning && scanKind === "rekordbox" ? "インポート中..." : "ライブラリにインポート"}
    </button>
  </header>

  <div class="body">
    <main class="content">
      <div
        class="tab-panel"
        hidden={activeTab !== "library"}
        inert={activeTab !== "library" ? true : undefined}
        aria-hidden={activeTab !== "library"}
      >
        <TrackList
          {tracks}
          selectedId={selectedTrack?.id ?? null}
          {rekordboxPathIndex}
          {rekordboxWritable}
          {rekordboxBusy}
          {rekordboxLockedHint}
          onselect={handleSelect}
          onremove={handleRemove}
          onbulkremove={handleBulkRemove}
          onAddToRekordbox={handleAddToRekordbox}
          onRemoveFromRekordbox={handleRemoveFromRekordbox}
        />
      </div>
      <div
        class="tab-panel"
        hidden={activeTab !== "rekordbox"}
        inert={activeTab !== "rekordbox" ? true : undefined}
        aria-hidden={activeTab !== "rekordbox"}
      >
        <RekordboxList
          tracks={rekordboxTracks}
          selectedId={selectedRekordboxId}
          loading={rekordboxLoading}
          status={rekordboxStatus}
          onselect={handleSelectRekordbox}
          onrefresh={() => loadRekordbox(false)}
          onensurewritable={ensureRekordboxWritable}
        />
      </div>
    </main>
    <ActivityConsole />
  </div>

  <StatusBar />
  <PreviewPlayer track={previewTrack} />
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

  .tabs {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .tab {
    padding: 0.4rem 0.85rem;
    border: none;
    background: var(--surface);
    color: var(--text-muted);
    font-size: 0.82rem;
    font-weight: 500;
    cursor: pointer;
  }

  .tab:not(:last-child) {
    border-right: 1px solid var(--border);
  }

  .tab:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .tab.active {
    background: var(--surface-selected);
    color: var(--accent);
  }

  .tab.active:hover {
    background: var(--surface-selected-hover);
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
    border-color: var(--surface-active);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .content {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .tab-panel {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .tab-panel[hidden] {
    display: none;
  }
</style>
