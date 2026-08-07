<script lang="ts">
  import type { Track } from "$lib/types";
  import { displayArtist, displayTitle, formatDuration } from "$lib/format";

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
      const title = (track.title ?? "").toLowerCase();
      const artist = (track.artist ?? "").toLowerCase();
      const album = (track.album ?? "").toLowerCase();
      const path = track.path.toLowerCase();
      return title.includes(q) || artist.includes(q) || album.includes(q) || path.includes(q);
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
      placeholder="Search tracks..."
      bind:value={query}
    />
    <span class="count">{filtered.length} tracks</span>
  </div>

  {#if filtered.length === 0}
    <div class="empty">
      {#if tracks.length === 0}
        <p>No tracks in library</p>
        <p class="hint">Add a folder to scan your music collection</p>
      {:else}
        <p>No tracks match your search</p>
      {/if}
    </div>
  {:else}
    <div class="table-wrap" role="grid" aria-label="Track library">
      <div class="table-header" role="row">
        <span role="columnheader">Title</span>
        <span role="columnheader">Artist</span>
        <span role="columnheader">Album</span>
        <span role="columnheader">Duration</span>
        <span role="columnheader">BPM</span>
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
          <span class="cell title" role="gridcell">{displayTitle(track)}</span>
          <span class="cell" role="gridcell">{displayArtist(track)}</span>
          <span class="cell muted" role="gridcell">{track.album ?? "—"}</span>
          <span class="cell mono" role="gridcell">{formatDuration(track.durationMs)}</span>
          <span class="cell mono" role="gridcell">
            {track.bpm ? track.bpm.toFixed(1) : "—"}
          </span>
          <span class="cell actions" role="gridcell">
            <button
              class="remove-btn"
              onclick={(e) => {
                e.stopPropagation();
                onremove(track);
              }}
              aria-label="Remove from library"
              title="Remove from library"
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
    overflow-y: auto;
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 2fr 1.5fr 1.5fr 5rem 4rem 2.5rem;
    gap: 0.75rem;
    align-items: center;
    padding: 0 1.25rem;
  }

  .table-header {
    position: sticky;
    top: 0;
    z-index: 1;
    padding-top: 0.5rem;
    padding-bottom: 0.5rem;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .table-row {
    padding-top: 0.45rem;
    padding-bottom: 0.45rem;
    font-size: 0.9rem;
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

  .title {
    font-weight: 500;
  }

  .muted {
    color: var(--text-muted);
  }

  .mono {
    font-variant-numeric: tabular-nums;
    font-size: 0.85rem;
    color: var(--text-muted);
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
