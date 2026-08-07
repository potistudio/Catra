<script lang="ts">
  import TrackArtwork from "$lib/components/TrackArtwork.svelte";
  import type { Track } from "$lib/types";
  import { displayArtist, displayTitle, formatDuration } from "$lib/format";

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
  class="track-card"
  class:selected
  role="button"
  tabindex="0"
  onclick={() => onselect(track)}
  onkeydown={handleKeydown}
  ondblclick={() => onselect(track)}
>
  <div class="artwork-wrap">
    <TrackArtwork
      artworkPath={track.artworkPath}
      title={displayTitle(track)}
      size={148}
    />
    {#if track.durationMs}
      <span class="duration">{formatDuration(track.durationMs)}</span>
    {/if}
    <button
      type="button"
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
  </div>
  <span class="title">{displayTitle(track)}</span>
  <span class="artist">{displayArtist(track)}</span>
</div>

<style>
  .track-card {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    outline: none;
    width: 100%;
    min-width: 0;
  }

  .track-card:hover {
    background: var(--surface-hover);
  }

  .track-card.selected {
    background: var(--accent-subtle);
    border-color: var(--accent);
  }

  .track-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .artwork-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 6px;
    overflow: hidden;
  }

  .artwork-wrap :global(.artwork) {
    width: 100% !important;
    height: 100% !important;
  }

  .duration {
    position: absolute;
    bottom: 0.35rem;
    right: 0.35rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.65);
    color: #fff;
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    line-height: 1.4;
  }

  .remove-btn {
    position: absolute;
    top: 0.35rem;
    right: 0.35rem;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    cursor: pointer;
    font-size: 0.75rem;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .track-card:hover .remove-btn,
  .track-card:focus-within .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    background: var(--danger);
  }

  .title {
    font-size: 0.85rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 0.25rem;
  }

  .artist {
    font-size: 0.75rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 0.25rem;
  }
</style>
