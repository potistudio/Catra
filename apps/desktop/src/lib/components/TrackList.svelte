<script lang="ts">
  import TrackRow from "$lib/components/TrackRow.svelte";
  import type { Track } from "$lib/types";
  import {
    filterTracks,
    getVisibleTrackRange,
    sortTracks,
    TRACK_ROW_HEIGHT,
    type SortColumn,
    type SortDirection,
  } from "$lib/trackListView";

  interface Props {
    tracks: Track[];
    selectedId: number | null;
    onselect: (track: Track) => void;
    onremove: (track: Track) => void;
  }

  let { tracks, selectedId, onselect, onremove }: Props = $props();

  let queryInput = $state("");
  let query = $state("");
  let sortColumn = $state<SortColumn>("artist");
  let sortDirection = $state<SortDirection>("asc");
  let scrollTop = $state(0);
  let viewportHeight = $state(0);

  let tableWrap = $state<HTMLDivElement | null>(null);

  $effect(() => {
    const value = queryInput;
    const timer = setTimeout(() => {
      query = value;
    }, 150);

    return () => clearTimeout(timer);
  });

  $effect(() => {
    const element = tableWrap;
    if (!element) return;

    const observer = new ResizeObserver(([entry]) => {
      viewportHeight = entry.contentRect.height;
    });

    observer.observe(element);
    viewportHeight = element.clientHeight;

    return () => observer.disconnect();
  });

  let filtered = $derived(filterTracks(tracks, query));
  let sorted = $derived(sortTracks(filtered, sortColumn, sortDirection));

  let visibleRange = $derived(
    getVisibleTrackRange(scrollTop, viewportHeight, sorted.length),
  );

  let visibleTracks = $derived(sorted.slice(visibleRange.start, visibleRange.end));
  let totalBodyHeight = $derived(sorted.length * TRACK_ROW_HEIGHT);
  let bodyOffsetY = $derived(visibleRange.start * TRACK_ROW_HEIGHT);

  function handleScroll(event: Event) {
    scrollTop = (event.currentTarget as HTMLDivElement).scrollTop;
  }

  function toggleSort(column: SortColumn) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
      return;
    }

    sortColumn = column;
    sortDirection = "asc";
  }

  function sortIndicator(column: SortColumn): string {
    if (sortColumn !== column) return "";
    return sortDirection === "asc" ? " ↑" : " ↓";
  }
</script>

<div class="track-list">
  <div class="track-list-header">
    <input
      class="search"
      type="search"
      placeholder="トラックを検索..."
      bind:value={queryInput}
    />
    <span class="count">{sorted.length} tracks</span>
  </div>

  {#if sorted.length === 0}
    <div class="empty">
      {#if tracks.length === 0}
        <p>ライブラリにトラックがありません</p>
        <p class="hint">フォルダを追加して音楽をスキャンしてください</p>
      {:else}
        <p>検索に一致するトラックがありません</p>
      {/if}
    </div>
  {:else}
    <div
      class="table-wrap"
      role="grid"
      aria-label="Track library"
      bind:this={tableWrap}
      onscroll={handleScroll}
    >
      <div class="table-inner">
        <div class="table-header" role="row">
          <span role="columnheader">ジャケット</span>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "title"}
            role="columnheader"
            aria-sort={sortColumn === "title" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("title")}
          >
            タイトル{sortIndicator("title")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "artist"}
            role="columnheader"
            aria-sort={sortColumn === "artist" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("artist")}
          >
            アーティスト{sortIndicator("artist")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "album"}
            role="columnheader"
            aria-sort={sortColumn === "album" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("album")}
          >
            アルバム{sortIndicator("album")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "bpm"}
            role="columnheader"
            aria-sort={sortColumn === "bpm" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("bpm")}
          >
            BPM{sortIndicator("bpm")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "bitrateKbps"}
            role="columnheader"
            aria-sort={sortColumn === "bitrateKbps" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("bitrateKbps")}
          >
            ビットレート{sortIndicator("bitrateKbps")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "key"}
            role="columnheader"
            aria-sort={sortColumn === "key" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("key")}
          >
            キー{sortIndicator("key")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "genre"}
            role="columnheader"
            aria-sort={sortColumn === "genre" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("genre")}
          >
            ジャンル{sortIndicator("genre")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "rating"}
            role="columnheader"
            aria-sort={sortColumn === "rating" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("rating")}
          >
            レート{sortIndicator("rating")}
          </button>
          <button
            type="button"
            class="column-header"
            class:active={sortColumn === "durationMs"}
            role="columnheader"
            aria-sort={sortColumn === "durationMs" ? (sortDirection === "asc" ? "ascending" : "descending") : "none"}
            onclick={() => toggleSort("durationMs")}
          >
            時間{sortIndicator("durationMs")}
          </button>
          <span role="columnheader"></span>
        </div>

        <div class="virtual-body" style:height="{totalBodyHeight}px">
          <div class="virtual-window" style:transform="translateY({bodyOffsetY}px)">
            {#each visibleTracks as track (track.id)}
              <TrackRow
                {track}
                selected={selectedId === track.id}
                {onselect}
                {onremove}
              />
            {/each}
          </div>
        </div>
      </div>
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
    min-height: 0;
  }

  .table-inner {
    min-width: 72rem;
  }

  .table-header {
    display: grid;
    grid-template-columns:
      3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 2rem;
    gap: 0.6rem;
    align-items: center;
    padding: 0 1rem;
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

  .column-header:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: 2px;
  }

  .virtual-body {
    position: relative;
  }

  .virtual-window {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }
</style>
