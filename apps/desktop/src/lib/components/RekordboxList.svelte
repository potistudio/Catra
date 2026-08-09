<script lang="ts">
  import type { RekordboxCheck, RekordboxContent } from "$lib/types";
  import {
    displayArtist,
    displayTitle,
    displayValue,
    formatBitrate,
    formatBpm,
    formatDuration,
    formatRating,
  } from "$lib/format";
  import {
    filterRekordboxContent,
    sortRekordboxContent,
    type RekordboxSortColumn,
    type SortDirection,
  } from "$lib/rekordboxListView";

  const SORT_LABELS: Record<RekordboxSortColumn, string> = {
    title: "タイトル",
    artist: "アーティスト",
    album: "アルバム",
    bpm: "BPM",
    bitRate: "ビットレート",
    key: "キー",
    genre: "ジャンル",
    rating: "レート",
    lengthSecs: "時間",
  };

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

  let queryInput = $state("");
  let query = $state("");
  let sortColumn = $state<RekordboxSortColumn>("artist");
  let sortDirection = $state<SortDirection>("asc");

  $effect(() => {
    const value = queryInput;
    const timer = setTimeout(() => {
      query = value;
    }, 150);

    return () => clearTimeout(timer);
  });

  let filtered = $derived(filterRekordboxContent(tracks, query));
  let sorted = $derived(sortRekordboxContent(filtered, sortColumn, sortDirection));

  function toggleSort(column: RekordboxSortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
      return;
    }

    sortColumn = column;
    sortDirection = "asc";
  }

  function sortIndicator(column: RekordboxSortColumn): string {
    if (sortColumn !== column) return "";
    return sortDirection === "asc" ? " ↑" : " ↓";
  }

  function durationMs(track: RekordboxContent): number | null {
    return track.lengthSecs != null ? track.lengthSecs * 1000 : null;
  }
</script>

<div class="rekordbox-list">
  <div class="rekordbox-list-header">
    <input
      class="search"
      type="search"
      placeholder="Rekordbox トラックを検索..."
      bind:value={queryInput}
      disabled={loading}
    />
    <span class="count">{sorted.length} tracks</span>
    <button
      type="button"
      class="refresh-btn"
      onclick={() => onrefresh()}
      disabled={loading}
    >
      {loading ? "読み込み中..." : "再読み込み"}
    </button>
  </div>

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
  {:else if sorted.length === 0}
    <div class="empty">
      {#if tracks.length === 0}
        <p>Rekordbox にトラックがありません</p>
        <p class="hint">Rekordbox に曲を登録してから再読み込みしてください</p>
      {:else}
        <p>検索に一致するトラックがありません</p>
      {/if}
    </div>
  {:else}
    <div class="table-wrap" role="grid" aria-label="Rekordbox library">
      <div class="table-inner">
        <div class="table-header" role="row">
          {#each Object.entries(SORT_LABELS) as [column, label] (column)}
            <button
              type="button"
              class="column-header"
              class:active={sortColumn === column}
              role="columnheader"
              aria-sort={sortColumn === column
                ? sortDirection === "asc"
                  ? "ascending"
                  : "descending"
                : "none"}
              onclick={() => toggleSort(column as RekordboxSortColumn)}
            >
              {label}{sortIndicator(column as RekordboxSortColumn)}
            </button>
          {/each}
        </div>

        {#each sorted as track (track.id)}
          <div
            class="table-row"
            class:selected={selectedId === track.id}
            role="row"
            tabindex="0"
            onclick={() => onselect(track)}
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onselect(track);
              }
            }}
          >
            <span class="cell title" role="gridcell">{displayTitle({ title: track.title, path: track.folderPath })}</span>
            <span class="cell" role="gridcell">{displayArtist(track)}</span>
            <span class="cell muted" role="gridcell">{displayValue(track.album)}</span>
            <span class="cell mono" role="gridcell">{formatBpm(track.bpm)}</span>
            <span class="cell mono" role="gridcell">{formatBitrate(track.bitRate)}</span>
            <span class="cell mono" role="gridcell">{displayValue(track.key)}</span>
            <span class="cell muted" role="gridcell">{displayValue(track.genre)}</span>
            <span
              class="cell rating"
              role="gridcell"
              title={track.rating ? `${track.rating}/255` : ""}
            >
              {formatRating(track.rating)}
            </span>
            <span class="cell mono" role="gridcell">{formatDuration(durationMs(track))}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .rekordbox-list {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .rekordbox-list-header {
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

  .hint {
    font-size: 0.85rem;
    opacity: 0.7;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .table-inner {
    min-width: 64rem;
  }

  .table-header {
    display: grid;
    grid-template-columns:
      minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem;
    gap: 0.6rem;
    align-items: center;
    padding: 0.5rem 1rem;
    position: sticky;
    top: 0;
    z-index: 1;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .column-header {
    padding: 0;
    border: none;
    background: transparent;
    font: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .column-header:hover {
    color: var(--text);
  }

  .column-header.active {
    color: var(--accent);
  }

  .table-row {
    display: grid;
    grid-template-columns:
      minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem;
    gap: 0.6rem;
    align-items: center;
    padding: 0.45rem 1rem;
    border-bottom: 1px solid var(--border-subtle);
    cursor: pointer;
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
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.85rem;
  }

  .cell.title {
    font-weight: 500;
  }

  .cell.muted {
    color: var(--text-muted);
  }

  .cell.mono {
    font-variant-numeric: tabular-nums;
    font-size: 0.8rem;
  }

  .cell.rating {
    color: #f0c040;
    font-size: 0.75rem;
  }
</style>
