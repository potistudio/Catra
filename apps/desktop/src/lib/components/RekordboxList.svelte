<script lang="ts">
  import { rekordboxGetPlaylistContent, rekordboxListPlaylists } from "$lib/api";
  import TrackList from "$lib/components/TrackList.svelte";
  import {
    isPlaylistFolder,
    playlistTreeRows,
    rekordboxContentToTrack,
  } from "$lib/rekordboxListView";
  import type {
    RekordboxCheck,
    RekordboxContent,
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
  let playlistTracks = $state<RekordboxContent[]>([]);
  let playlistLoading = $state(false);

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
    browseMode === "all" ? loading : playlistLoading || loading,
  );

  $effect(() => {
    if (status?.dbPath) {
      void loadPlaylists();
    } else {
      playlists = [];
      selectedPlaylistId = null;
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
    if (isPlaylistFolder(playlist)) return;
    browseMode = "playlist";
    selectedPlaylistId = playlist.id;
    void loadPlaylistTracks(playlist.id);
  }

  async function handleRefresh() {
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
</script>

<div class="rekordbox-list">
  {#if status}
    <div class="status-bar" class:warning={status.rekordboxRunning}>
      {#if status.rekordboxRunning}
        <span>Rekordbox が起動中です。データベースがロックされる場合があります。</span>
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

      <div class="playlist-scroll">
        {#if playlistRows.length === 0}
          <p class="sidebar-empty">プレイリストがありません</p>
        {:else}
          <ul class="playlist-tree">
            {#each playlistRows as playlist (playlist.id)}
              <li>
                {#if isPlaylistFolder(playlist)}
                  <div
                    class="playlist-item folder"
                    style={`padding-left: ${0.65 + playlist.depth * 0.75}rem`}
                  >
                    {playlist.name}
                  </div>
                {:else}
                  <button
                    type="button"
                    class="playlist-item"
                    class:active={browseMode === "playlist" &&
                      selectedPlaylistId === playlist.id}
                    style={`padding-left: ${0.65 + playlist.depth * 0.75}rem`}
                    onclick={() => selectPlaylist(playlist)}
                  >
                    {playlist.name}
                  </button>
                {/if}
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
          readonly
          searchPlaceholder="Rekordbox トラックを検索..."
          emptyTitle={browseMode === "playlist"
            ? "このプレイリストにトラックがありません"
            : "Rekordbox にトラックがありません"}
          emptyHint={browseMode === "playlist"
            ? "別のプレイリストを選ぶか、Rekordbox で曲を追加してください"
            : "Rekordbox に曲を登録してから再読み込みしてください"}
          onselect={handleSelect}
        >
          {#snippet headerExtra()}
            <button
              type="button"
              class="refresh-btn"
              onclick={() => handleRefresh()}
              disabled={listLoading}
            >
              {listLoading ? "読み込み中..." : "再読み込み"}
            </button>
          {/snippet}
        </TrackList>
      {/if}
    </div>
  </div>
</div>

<style>
  .rekordbox-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .status-bar {
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
    width: 220px;
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

  .playlist-item {
    display: block;
    width: 100%;
    padding: 0.4rem 0.65rem;
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

  .playlist-item.active {
    background: var(--accent-subtle);
    color: var(--accent);
  }

  .playlist-item.folder {
    color: var(--text-muted);
    font-weight: 600;
    cursor: default;
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

  .refresh-btn {
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .refresh-btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .refresh-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
