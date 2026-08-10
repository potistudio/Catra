<script lang="ts">
  import SelectionCheckbox from "$lib/components/SelectionCheckbox.svelte";
  import TrackArtwork from "$lib/components/TrackArtwork.svelte";
  import TrackSourceBadge from "$lib/components/TrackSourceBadge.svelte";
  import type { Track } from "$lib/types";
  import { isTrackSource } from "$lib/trackSource";
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
    checked: boolean;
    readonly?: boolean;
    removeTitle?: string;
    inRekordbox?: boolean;
    showActions?: boolean;
    onselect: (track: Track) => void;
    onremove?: (track: Track) => void;
    ontogglecheck?: (track: Track) => void;
  }

  let {
    track,
    selected,
    checked,
    readonly = false,
    removeTitle = "ライブラリから削除",
    inRekordbox = false,
    showActions = true,
    onselect,
    onremove,
    ontogglecheck,
  }: Props = $props();

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
  class:checked
  class:no-actions={!showActions}
  role="row"
  tabindex="0"
  onclick={() => onselect(track)}
  onkeydown={handleKeydown}
  ondblclick={() => onselect(track)}
>
  <span class="cell checkbox-cell sticky-col" role="gridcell">
    {#if !readonly && ontogglecheck}
      <SelectionCheckbox
        {checked}
        label={`${displayTitle(track)} を選択`}
        onToggle={() => ontogglecheck(track)}
      />
    {/if}
  </span>
  <span class="cell artwork-cell" role="gridcell">
    <div class="artwork-wrap">
      <TrackArtwork artworkPath={track.artworkPath} title={displayTitle(track)} />
      {#if isTrackSource(track.source)}
        <span class="source-badge-wrap">
          <TrackSourceBadge source={track.source} size={12} />
        </span>
      {/if}
    </div>
  </span>
  <span class="cell title" role="gridcell">
    <span class="title-text">{displayTitle(track)}</span>
    {#if inRekordbox}
      <span class="rb-badge" title="Rekordbox に登録済み">Rekordbox</span>
    {/if}
  </span>
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
  {#if showActions}
    <span class="cell actions" role="gridcell">
      {#if !readonly && onremove}
        <button
          class="remove-btn"
          onclick={(e) => {
            e.stopPropagation();
            onremove(track);
          }}
          aria-label={removeTitle}
          title={removeTitle}
        >
          ✕
        </button>
      {/if}
    </span>
  {/if}
</div>

<style>
  .table-row {
    display: grid;
    grid-template-columns:
      2.5rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
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
    transition: background-color 0.12s ease;
  }

  .table-row.no-actions {
    grid-template-columns:
      2.5rem 3rem minmax(10rem, 1.4fr) minmax(8rem, 1.1fr) minmax(8rem, 1.1fr)
      3.5rem 5.5rem 3.5rem minmax(6rem, 1fr) 4.5rem 3.5rem;
    min-width: 70rem;
  }

  .table-row:hover {
    background: var(--surface-hover);
  }

  .table-row.selected {
    background: var(--accent-subtle);
  }

  .table-row.checked {
    background: color-mix(in srgb, var(--accent-subtle) 60%, transparent);
  }

  .table-row.selected.checked {
    background: var(--accent-subtle);
  }

  .table-row.selected:hover,
  .table-row.checked:hover,
  .table-row.selected.checked:hover {
    background: var(--accent-subtle-hover);
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

  .checkbox-cell {
    display: flex;
    justify-content: center;
    align-items: center;
    overflow: visible;
  }

  .sticky-col {
    position: sticky;
    left: 0;
    z-index: 1;
    background: inherit;
  }

  .table-row:hover .sticky-col,
  .table-row.selected .sticky-col,
  .table-row.checked .sticky-col {
    background: inherit;
  }

  .artwork-wrap {
    position: relative;
    width: 40px;
    height: 40px;
  }

  .source-badge-wrap {
    position: absolute;
    top: -4px;
    right: -4px;
    z-index: 1;
  }

  .source-badge-wrap :global(.source-badge) {
    width: 16px;
    height: 16px;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-weight: 500;
    min-width: 0;
  }

  .title-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rb-badge {
    flex-shrink: 0;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    background: var(--accent-subtle);
    color: var(--accent);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    line-height: 1.3;
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
    color: var(--warning);
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
    background: var(--danger-subtle-hover);
    color: var(--danger-hover);
  }
</style>
