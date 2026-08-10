<script lang="ts">
  import TrackList from "$lib/components/TrackList.svelte";
  import { rekordboxContentToTrack } from "$lib/rekordboxListView";
  import type { RekordboxCheck, RekordboxContent, Track } from "$lib/types";

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

  let displayTracks = $derived(tracks.map(rekordboxContentToTrack));
  let selectedDisplayId = $derived(
    selectedId == null
      ? null
      : tracks.findIndex((track) => track.id === selectedId),
  );
  let selectedListId = $derived(
    selectedDisplayId != null && selectedDisplayId >= 0
      ? selectedDisplayId
      : null,
  );

  function handleSelect(track: Track) {
    const content = tracks[track.id];
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

  {#if loading && tracks.length === 0}
    <div class="empty">
      <p>Rekordbox ライブラリを読み込み中...</p>
    </div>
  {:else}
    <TrackList
      tracks={displayTracks}
      selectedId={selectedListId}
      readonly
      searchPlaceholder="Rekordbox トラックを検索..."
      emptyTitle="Rekordbox にトラックがありません"
      emptyHint="Rekordbox に曲を登録してから再読み込みしてください"
      onselect={handleSelect}
    >
      {#snippet headerExtra()}
        <button
          type="button"
          class="refresh-btn"
          onclick={() => onrefresh()}
          disabled={loading}
        >
          {loading ? "読み込み中..." : "再読み込み"}
        </button>
      {/snippet}
    </TrackList>
  {/if}
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
