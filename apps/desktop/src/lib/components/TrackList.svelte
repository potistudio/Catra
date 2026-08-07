<script lang="ts">
  import TrackArtwork from "$lib/components/TrackArtwork.svelte";
  import type { Track } from "$lib/types";
  import {
    displayArtist,
    displayTitle,
    displayValue,
    formatBitrate,
    formatBpm,
    formatDuration,
    formatRating,
  } from "$lib/format";

  interface Props {
    tracks: Track[];
    selectedId: number | null;
    onselect: (track: Track) => void;
    onremove: (track: Track) => void;
  }

  let { tracks, selectedId, onselect, onremove }: Props = $props();

  let query = $state("");

  let filtered = $derived(
    tracks.filter((track) => {
      if (!query.trim()) return true;
      const q = query.toLowerCase();
      const fields = [
        track.title,
        track.artist,
        track.album,
        track.genre,
        track.key,
        track.path,
      ];
      return fields.some((field) => (field ?? "").toLowerCase().includes(q));
    }),
  );

  function handleKeydown(event: KeyboardEvent, track: Track) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onselect(track);
    }
  }
</script>

<div class="track-list">
  <div class="track-list-header">
    <input
      class="search"
      type="search"
      placeholder="トラックを検索..."
      bind:value={query}
    />
    <span class="count">{filtered.length} tracks</span>
  </div>

  {#if filtered.length === 0}
    <div class="empty">
      {#if tracks.length === 0}
        <p>ライブラリにトラックがありません</p>
        <p class="hint">フォルダを追加して音楽をスキャンしてください</p>
      {:else}
        <p>検索に一致するトラックがありません</p>
      {/if}
    </div>
  {:else}
    <div class="table-wrap" role="grid" aria-label="Track library">
      <div class="table-header" role="row">
        <span role="columnheader">ジャケット</span>
        <span role="columnheader">タイトル</span>
        <span role="columnheader">アーティスト</span>
        <span role="columnheader">アルバム</span>
        <span role="columnheader">BPM</span>
        <span role="columnheader">ビットレート</span>
        <span role="columnheader">キー</span>
        <span role="columnheader">ジャンル</span>
        <span role="columnheader">レート</span>
        <span role="columnheader">時間</span>
        <span role="columnheader"></span>
      </div>

      {#each filtered as track (track.id)}
        <div
          class="table-row"
          class:selected={selectedId === track.id}
          role="row"
          tabindex="0"
          onclick={() => onselect(track)}
          onkeydown={(e) => handleKeydown(e, track)}
          ondblclick={() => onselect(track)}
        >
          <span class="cell artwork-cell" role="gridcell">
            <TrackArtwork artworkPath={track.artworkPath} title={displayTitle(track)} />
          </span>
          <span class="cell title" role="gridcell">{displayTitle(track)}</span>
          <span class="cell" role="gridcell">{displayArtist(track)}</span>
          <span class="cell muted" role="gridcell">{displayValue(track.album)}</span>
          <span class="cell mono" role="gridcell">{formatBpm(track.bpm)}</span>
          <span class="cell mono" role="gridcell">{formatBitrate(track.bitrateKbps)}</span>
          <span class="cell mono" role="gridcell">{displayValue(track.key)}</span>
          <span class="cell muted" role="gridcell">{displayValue(track.genre)}</span>
          <span class="cell rating" role="gridcell" title={track.rating ? `${track.rating}/255` : ""}>
            {formatRating(track.rating)}
          </span>
          <span class="cell mono" role="gridcell">{formatDuration(track.durationMs)}</span>
          <span class="cell actions" role="gridcell">
            <button
              class="remove-btn"
              onclick={(e) => {
                e.stopPropagation();
                onremove(track);
              }}
              aria-label="Remove from library"
              title="ライブラリから削除"
            >
              ✕
            </button>
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .track-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .track-list-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }

  .search {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.9rem;
  }

  .search:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  .count {
    font-size: 0.8rem;
    color: var(--text-muted);
    white-space: nowrap;
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

  .hint {
    font-size: 0.85rem;
    opacity: 0.7;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns:
      3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 2rem;
    gap: 0.6rem;
    align-items: center;
    padding: 0 1rem;
    min-width: 72rem;
  }

  .table-header {
    position: sticky;
    top: 0;
    z-index: 1;
    padding-top: 0.5rem;
    padding-bottom: 0.5rem;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .table-row {
    padding-top: 0.35rem;
    padding-bottom: 0.35rem;
    font-size: 0.85rem;
    border-bottom: 1px solid var(--border-subtle);
    cursor: pointer;
    outline: none;
  }

  .table-row:hover {
    background: var(--surface-hover);
  }

  .table-row.selected {
    background: var(--accent-subtle);
  }

  .table-row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .cell {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .artwork-cell {
    display: flex;
    justify-content: center;
    overflow: visible;
  }

  .title {
    font-weight: 500;
  }

  .muted {
    color: var(--text-muted);
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .rating {
    font-size: 0.75rem;
    color: #f0c040;
    letter-spacing: -0.05em;
  }

  .actions {
    display: flex;
    justify-content: center;
  }

  .remove-btn {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.75rem;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .table-row:hover .remove-btn,
  .table-row:focus-within .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    background: var(--danger-subtle);
    color: var(--danger);
  }
</style>
