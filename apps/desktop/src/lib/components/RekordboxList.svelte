<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    rekordboxAddContent,
    rekordboxAddToPlaylist,
    rekordboxCreatePlaylist,
    rekordboxCreatePlaylistFolder,
    rekordboxDeleteContent,
    rekordboxDeletePlaylist,
    rekordboxGetPlaylistContent,
    rekordboxListPlaylists,
    rekordboxMoveSongInPlaylist,
    rekordboxRemoveFromPlaylist,
    rekordboxRenamePlaylist,
    rekordboxUpdateContent,
  } from "$lib/api";
  import TrackList from "$lib/components/TrackList.svelte";
  import {
    isPlaylistFolder,
    playlistTreeRows,
    rekordboxContentToTrack,
  } from "$lib/rekordboxListView";
  import type {
    RekordboxCheck,
    RekordboxContent,
    RekordboxContentUpdate,
    RekordboxPlaylist,
    Track,
  } from "$lib/types";

  type BrowseMode = "all" | "playlist";

  interface Props {
    tracks: RekordboxContent[];
    selectedId: string | null;
    loading: boolean;
    status: RekordboxCheck | null;
    onselect: (track: RekordboxContent) => void;
    onrefresh: () => void | Promise<void>;
  }

  let {
    tracks,
    selectedId,
    loading,
    status,
    onselect,
    onrefresh,
  }: Props = $props();

  let browseMode = $state<BrowseMode>("all");
  let playlists = $state<RekordboxPlaylist[]>([]);
  let selectedPlaylistId = $state<string | null>(null);
  let selectedFolderId = $state<string | null>(null);
  let playlistTracks = $state<RekordboxContent[]>([]);
  let playlistLoading = $state(false);
  let actionError = $state<string | null>(null);
  let busy = $state(false);

  let editOpen = $state(false);
  let editTarget = $state<RekordboxContent | null>(null);
  let editForm = $state({
    title: "",
    artist: "",
    album: "",
    genre: "",
    comment: "",
    bpm: "",
    key: "",
    rating: "",
  });

  let playlistRows = $derived(playlistTreeRows(playlists));
  let activeTracks = $derived(browseMode === "all" ? tracks : playlistTracks);
  let displayTracks = $derived(activeTracks.map(rekordboxContentToTrack));
  let selectedDisplayId = $derived(
    selectedId == null
      ? null
      : activeTracks.findIndex((track) => track.id === selectedId),
  );
  let selectedListId = $derived(
    selectedDisplayId != null && selectedDisplayId >= 0
      ? selectedDisplayId
      : null,
  );
  let listLoading = $derived(
    browseMode === "playlist" ? playlistLoading || loading : loading,
  );
  let writable = $derived(!!status?.dbPath && !status.rekordboxRunning);
  let normalPlaylists = $derived(
    playlists.filter((playlist) => playlist.attribute === 0),
  );

  $effect(() => {
    if (status?.dbPath) {
      void loadPlaylists();
    } else {
      playlists = [];
      selectedPlaylistId = null;
      selectedFolderId = null;
      playlistTracks = [];
    }
  });

  async function loadPlaylists() {
    try {
      playlists = await rekordboxListPlaylists();
      if (
        selectedPlaylistId &&
        !playlists.some((playlist) => playlist.id === selectedPlaylistId)
      ) {
        selectedPlaylistId = null;
        playlistTracks = [];
      }
      if (
        selectedFolderId &&
        !playlists.some(
          (playlist) =>
            playlist.id === selectedFolderId && isPlaylistFolder(playlist),
        )
      ) {
        selectedFolderId = null;
      }
    } catch {
      playlists = [];
    }
  }

  async function loadPlaylistTracks(playlistId: string) {
    playlistLoading = true;
    try {
      playlistTracks = await rekordboxGetPlaylistContent(playlistId);
    } catch {
      playlistTracks = [];
    } finally {
      playlistLoading = false;
    }
  }

  function setBrowseMode(mode: BrowseMode) {
    browseMode = mode;
    if (mode === "all") {
      selectedPlaylistId = null;
      playlistTracks = [];
      return;
    }
    if (selectedPlaylistId) {
      void loadPlaylistTracks(selectedPlaylistId);
    }
  }

  function selectPlaylist(playlist: RekordboxPlaylist) {
    if (isPlaylistFolder(playlist)) {
      selectedFolderId = playlist.id;
      return;
    }
    browseMode = "playlist";
    selectedPlaylistId = playlist.id;
    selectedFolderId = playlist.parentId;
    void loadPlaylistTracks(playlist.id);
  }

  async function handleRefresh() {
    actionError = null;
    await onrefresh();
    await loadPlaylists();
    if (browseMode === "playlist" && selectedPlaylistId) {
      await loadPlaylistTracks(selectedPlaylistId);
    }
  }

  function handleSelect(track: Track) {
    const content = activeTracks[track.id];
    if (content) onselect(content);
  }

  function contentAt(index: number): RekordboxContent | null {
    return activeTracks[index] ?? null;
  }

  async function runAction(action: () => Promise<void>) {
    if (!writable || busy) return;
    busy = true;
    actionError = null;
    try {
      await action();
    } catch (error) {
      actionError = error instanceof Error ? error.message : String(error);
    } finally {
      busy = false;
    }
  }

  function parentForCreate(): string | null {
    return selectedFolderId;
  }

  async function handleCreatePlaylist() {
    const name = prompt("プレイリスト名");
    if (!name?.trim()) return;
    await runAction(async () => {
      const created = await rekordboxCreatePlaylist(
        name.trim(),
        parentForCreate(),
      );
      await loadPlaylists();
      selectPlaylist(created);
    });
  }

  async function handleCreateFolder() {
    const name = prompt("フォルダ名");
    if (!name?.trim()) return;
    await runAction(async () => {
      const created = await rekordboxCreatePlaylistFolder(
        name.trim(),
        parentForCreate(),
      );
      await loadPlaylists();
      selectedFolderId = created.id;
    });
  }

  async function handleRenamePlaylist(playlist: RekordboxPlaylist) {
    const name = prompt("新しい名前", playlist.name);
    if (!name?.trim() || name.trim() === playlist.name) return;
    await runAction(async () => {
      await rekordboxRenamePlaylist(playlist.id, name.trim());
      await loadPlaylists();
    });
  }

  async function handleDeletePlaylist(playlist: RekordboxPlaylist) {
    const label = isPlaylistFolder(playlist) ? "フォルダ" : "プレイリスト";
    if (
      !confirm(
        `${label}「${playlist.name}」を削除しますか？子要素もすべて削除されます。`,
      )
    ) {
      return;
    }
    await runAction(async () => {
      await rekordboxDeletePlaylist(playlist.id);
      if (selectedPlaylistId === playlist.id) {
        selectedPlaylistId = null;
        playlistTracks = [];
        browseMode = "all";
      }
      if (selectedFolderId === playlist.id) {
        selectedFolderId = null;
      }
      await loadPlaylists();
    });
  }

  async function handleAddFiles() {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Audio",
          extensions: ["mp3", "wav", "flac", "aiff", "aif", "m4a", "aac"],
        },
      ],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    await runAction(async () => {
      for (const path of paths) {
        await rekordboxAddContent(path);
      }
      await handleRefresh();
    });
  }

  async function handleRemoveTrack(track: Track) {
    const content = contentAt(track.id);
    if (!content) return;

    if (browseMode === "playlist" && selectedPlaylistId) {
      const songId = content.songPlaylistId;
      if (!songId) {
        actionError = "プレイリスト会員IDが見つかりません";
        return;
      }
      if (!confirm("このプレイリストから除外しますか？")) return;
      await runAction(async () => {
        await rekordboxRemoveFromPlaylist(selectedPlaylistId!, songId);
        await loadPlaylistTracks(selectedPlaylistId!);
      });
      return;
    }

    if (!confirm(`「${content.title ?? content.fileName ?? content.id}」をコレクションから削除しますか？`)) {
      return;
    }
    await runAction(async () => {
      await rekordboxDeleteContent(content.id);
      await handleRefresh();
    });
  }

  async function handleBulkRemove(indices: number[]) {
    await runAction(async () => {
      if (browseMode === "playlist" && selectedPlaylistId) {
        for (const index of indices) {
          const content = contentAt(index);
          if (!content?.songPlaylistId) continue;
          await rekordboxRemoveFromPlaylist(
            selectedPlaylistId,
            content.songPlaylistId,
          );
        }
        await loadPlaylistTracks(selectedPlaylistId);
        return;
      }

      for (const index of indices) {
        const content = contentAt(index);
        if (!content) continue;
        await rekordboxDeleteContent(content.id);
      }
      await handleRefresh();
    });
  }

  async function handleAddSelectedToPlaylist(content: RekordboxContent) {
    if (normalPlaylists.length === 0) {
      actionError = "追加先のプレイリストがありません";
      return;
    }
    const choices = normalPlaylists
      .map((playlist, index) => `${index + 1}. ${playlist.name}`)
      .join("\n");
    const answer = prompt(`追加先の番号を入力してください\n${choices}`);
    if (!answer) return;
    const index = Number.parseInt(answer, 10) - 1;
    const target = normalPlaylists[index];
    if (!target) {
      actionError = "無効な番号です";
      return;
    }
    await runAction(async () => {
      await rekordboxAddToPlaylist(target.id, content.id);
      if (selectedPlaylistId === target.id) {
        await loadPlaylistTracks(target.id);
      }
    });
  }

  async function handleMoveSong(delta: -1 | 1) {
    if (!selectedPlaylistId || selectedDisplayId == null || selectedDisplayId < 0) {
      return;
    }
    const content = contentAt(selectedDisplayId);
    if (!content?.songPlaylistId) return;
    const newTrackNo = selectedDisplayId + 1 + delta;
    if (newTrackNo < 1 || newTrackNo > playlistTracks.length) return;
    await runAction(async () => {
      await rekordboxMoveSongInPlaylist(
        selectedPlaylistId!,
        content.songPlaylistId!,
        newTrackNo,
      );
      await loadPlaylistTracks(selectedPlaylistId!);
    });
  }

  function openEdit(content: RekordboxContent) {
    editTarget = content;
    editForm = {
      title: content.title ?? "",
      artist: content.artist ?? "",
      album: content.album ?? "",
      genre: content.genre ?? "",
      comment: content.comment ?? "",
      bpm: content.bpm != null ? String(content.bpm) : "",
      key: content.key ?? "",
      rating: content.rating != null ? String(Math.round(content.rating / 51)) : "",
    };
    editOpen = true;
  }

  async function saveEdit() {
    if (!editTarget) return;
    const fields: RekordboxContentUpdate = {
      title: editForm.title,
      artist: editForm.artist,
      album: editForm.album,
      genre: editForm.genre,
      comment: editForm.comment,
      key: editForm.key,
    };
    if (editForm.bpm.trim()) {
      const bpm = Number.parseFloat(editForm.bpm);
      if (!Number.isNaN(bpm)) fields.bpm = bpm;
    }
    if (editForm.rating.trim()) {
      const rating = Number.parseInt(editForm.rating, 10);
      if (!Number.isNaN(rating)) fields.rating = rating;
    }
    await runAction(async () => {
      await rekordboxUpdateContent(editTarget!.id, fields);
      editOpen = false;
      editTarget = null;
      await handleRefresh();
    });
  }

  function selectedContent(): RekordboxContent | null {
    if (selectedDisplayId == null || selectedDisplayId < 0) return null;
    return contentAt(selectedDisplayId);
  }
</script>

<div class="rekordbox-list">
  {#if status}
    <div class="status-bar" class:warning={status.rekordboxRunning}>
      {#if status.rekordboxRunning}
        <span>Rekordbox が起動中のため編集できません。終了してから操作してください。</span>
      {:else if status.dbPath}
        <span>master.db: {status.dbPath}</span>
      {:else}
        <span>Rekordbox の master.db が見つかりません。</span>
      {/if}
      {#if status.version}
        <span class="version">v{status.version}</span>
      {/if}
    </div>
  {/if}

  {#if actionError}
    <div class="error-bar">{actionError}</div>
  {/if}

  <div class="body">
    <aside class="sidebar">
      <div class="mode-switch" role="tablist" aria-label="表示切替">
        <button
          type="button"
          class="mode-btn"
          class:active={browseMode === "all"}
          role="tab"
          aria-selected={browseMode === "all"}
          onclick={() => setBrowseMode("all")}
        >
          全て
        </button>
        <button
          type="button"
          class="mode-btn"
          class:active={browseMode === "playlist"}
          role="tab"
          aria-selected={browseMode === "playlist"}
          onclick={() => setBrowseMode("playlist")}
        >
          プレイリスト
        </button>
      </div>

      <div class="sidebar-actions">
        <button
          type="button"
          class="side-btn"
          disabled={!writable || busy}
          onclick={() => handleCreatePlaylist()}
        >
          ＋ リスト
        </button>
        <button
          type="button"
          class="side-btn"
          disabled={!writable || busy}
          onclick={() => handleCreateFolder()}
        >
          ＋ フォルダ
        </button>
      </div>

      <div class="playlist-scroll">
        {#if playlistRows.length === 0}
          <p class="sidebar-empty">プレイリストがありません</p>
        {:else}
          <ul class="playlist-tree">
            {#each playlistRows as playlist (playlist.id)}
              <li>
                <div
                  class="playlist-row"
                  class:folder={isPlaylistFolder(playlist)}
                  class:active={browseMode === "playlist" &&
                    selectedPlaylistId === playlist.id}
                  class:folder-selected={selectedFolderId === playlist.id &&
                    isPlaylistFolder(playlist)}
                  style={`padding-left: ${0.65 + playlist.depth * 0.75}rem`}
                >
                  {#if isPlaylistFolder(playlist)}
                    <button
                      type="button"
                      class="playlist-item folder"
                      onclick={() => selectPlaylist(playlist)}
                    >
                      {playlist.name}
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="playlist-item"
                      onclick={() => selectPlaylist(playlist)}
                    >
                      {playlist.name}
                    </button>
                  {/if}
                  <div class="playlist-ops">
                    <button
                      type="button"
                      class="icon-btn"
                      title="改名"
                      disabled={!writable || busy}
                      onclick={() => handleRenamePlaylist(playlist)}
                    >
                      ✎
                    </button>
                    <button
                      type="button"
                      class="icon-btn"
                      title="削除"
                      disabled={!writable || busy}
                      onclick={() => handleDeletePlaylist(playlist)}
                    >
                      ✕
                    </button>
                  </div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </aside>

    <div class="main">
      {#if listLoading && activeTracks.length === 0}
        <div class="empty">
          <p>
            {browseMode === "playlist" && !selectedPlaylistId
              ? "プレイリストを選択してください"
              : "Rekordbox ライブラリを読み込み中..."}
          </p>
        </div>
      {:else if browseMode === "playlist" && !selectedPlaylistId}
        <div class="empty">
          <p>プレイリストを選択してください</p>
        </div>
      {:else}
        <TrackList
          tracks={displayTracks}
          selectedId={selectedListId}
          readonly={!writable}
          searchPlaceholder="Rekordbox トラックを検索..."
          emptyTitle={browseMode === "playlist"
            ? "このプレイリストにトラックがありません"
            : "Rekordbox にトラックがありません"}
          emptyHint={browseMode === "playlist"
            ? "曲を追加するか、別のプレイリストを選んでください"
            : "ファイルを追加するか、Rekordbox で曲を登録してください"}
          bulkRemoveConfirmMessage={browseMode === "playlist"
            ? "曲をこのプレイリストから除外しますか？"
            : "曲をコレクションから削除しますか？"}
          removeTitle={browseMode === "playlist"
            ? "プレイリストから除外"
            : "コレクションから削除"}
          onselect={handleSelect}
          onremove={writable ? handleRemoveTrack : undefined}
          onbulkremove={writable ? handleBulkRemove : undefined}
        >
          {#snippet headerExtra()}
            <button
              type="button"
              class="refresh-btn"
              onclick={() => handleRefresh()}
              disabled={listLoading || busy}
            >
              {listLoading ? "読み込み中..." : "再読み込み"}
            </button>
            {#if writable}
              <button
                type="button"
                class="refresh-btn"
                disabled={busy}
                onclick={() => handleAddFiles()}
              >
                ファイル追加
              </button>
              {#if selectedContent()}
                <button
                  type="button"
                  class="refresh-btn"
                  disabled={busy}
                  onclick={() => openEdit(selectedContent()!)}
                >
                  編集
                </button>
                <button
                  type="button"
                  class="refresh-btn"
                  disabled={busy}
                  onclick={() => handleAddSelectedToPlaylist(selectedContent()!)}
                >
                  リストへ追加
                </button>
              {/if}
              {#if browseMode === "playlist" && selectedPlaylistId}
                <button
                  type="button"
                  class="refresh-btn"
                  disabled={busy || selectedDisplayId == null || selectedDisplayId <= 0}
                  onclick={() => handleMoveSong(-1)}
                >
                  ↑
                </button>
                <button
                  type="button"
                  class="refresh-btn"
                  disabled={busy ||
                    selectedDisplayId == null ||
                    selectedDisplayId < 0 ||
                    selectedDisplayId >= playlistTracks.length - 1}
                  onclick={() => handleMoveSong(1)}
                >
                  ↓
                </button>
              {/if}
            {/if}
          {/snippet}
        </TrackList>
      {/if}
    </div>
  </div>
</div>

{#if editOpen && editTarget}
  <div class="modal-backdrop" role="presentation" onclick={() => (editOpen = false)}>
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      aria-label="トラック編集"
      onclick={(event) => event.stopPropagation()}
    >
      <h3>トラック編集</h3>
      <label>
        タイトル
        <input bind:value={editForm.title} />
      </label>
      <label>
        アーティスト
        <input bind:value={editForm.artist} />
      </label>
      <label>
        アルバム
        <input bind:value={editForm.album} />
      </label>
      <label>
        ジャンル
        <input bind:value={editForm.genre} />
      </label>
      <label>
        キー
        <input bind:value={editForm.key} />
      </label>
      <label>
        BPM
        <input bind:value={editForm.bpm} />
      </label>
      <label>
        レート (0-5)
        <input bind:value={editForm.rating} />
      </label>
      <label>
        コメント
        <input bind:value={editForm.comment} />
      </label>
      <div class="modal-actions">
        <button type="button" class="refresh-btn" onclick={() => (editOpen = false)}>
          キャンセル
        </button>
        <button
          type="button"
          class="refresh-btn primary"
          disabled={busy}
          onclick={() => saveEdit()}
        >
          保存
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .rekordbox-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .status-bar,
  .error-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 1.25rem;
    font-size: 0.78rem;
    color: var(--text-muted);
    background: var(--surface-raised);
    border-bottom: 1px solid var(--border);
    overflow: hidden;
  }

  .status-bar.warning {
    color: #f0c040;
    background: rgba(240, 192, 64, 0.08);
  }

  .error-bar {
    color: #f08080;
    background: rgba(240, 80, 80, 0.08);
  }

  .status-bar span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .version {
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    width: 240px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: var(--surface-raised);
    min-height: 0;
  }

  .mode-switch {
    display: flex;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .mode-btn {
    flex: 1;
    padding: 0.55rem 0.4rem;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }

  .mode-btn:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .mode-btn.active {
    color: var(--accent);
    background: var(--accent-subtle);
  }

  .sidebar-actions {
    display: flex;
    gap: 0.35rem;
    padding: 0.45rem;
    border-bottom: 1px solid var(--border);
  }

  .side-btn,
  .refresh-btn,
  .icon-btn {
    padding: 0.35rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .side-btn {
    flex: 1;
  }

  .side-btn:hover:not(:disabled),
  .refresh-btn:hover:not(:disabled),
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .side-btn:disabled,
  .refresh-btn:disabled,
  .icon-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .refresh-btn.primary {
    background: var(--accent-subtle);
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
  }

  .playlist-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }

  .playlist-tree {
    list-style: none;
    margin: 0;
    padding: 0.35rem 0;
  }

  .playlist-row {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    min-width: 0;
  }

  .playlist-row.active,
  .playlist-row.folder-selected {
    background: var(--accent-subtle);
  }

  .playlist-item {
    display: block;
    flex: 1;
    min-width: 0;
    padding: 0.4rem 0.35rem;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .playlist-item:hover {
    background: var(--surface-hover);
  }

  .playlist-item.folder {
    color: var(--text-muted);
    font-weight: 600;
  }

  .playlist-ops {
    display: flex;
    flex-shrink: 0;
    opacity: 0;
  }

  .playlist-row:hover .playlist-ops {
    opacity: 1;
  }

  .icon-btn {
    padding: 0.15rem 0.3rem;
    font-size: 0.7rem;
  }

  .sidebar-empty {
    margin: 0;
    padding: 1rem 0.75rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-muted);
    gap: 0.25rem;
  }

  .empty p {
    margin: 0;
  }

  .modal-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 20;
  }

  .modal {
    width: min(28rem, calc(100% - 2rem));
    max-height: calc(100% - 2rem);
    overflow: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .modal h3 {
    margin: 0 0 0.25rem;
    font-size: 1rem;
  }

  .modal label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .modal input {
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.85rem;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.45rem;
    margin-top: 0.35rem;
  }
</style>
