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
    track: Track;
    selected: boolean;
    onselect: (track: Track) => void;
    onremove: (track: Track) => void;
  }

  let { track, selected, onselect, onremove }: Props = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onselect(track);
    }
  }
</script>

<div
  class="table-row"
  class:selected
  role="row"
  tabindex="0"
  onclick={() => onselect(track)}
  onkeydown={handleKeydown}
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

<style>
  .table-row {
    display: grid;
    grid-template-columns:
      3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem 2rem;
    gap: 0.6rem;
    align-items: center;
    height: 48px;
    box-sizing: border-box;
    padding: 0 1rem;
    min-width: 72rem;
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
