<script lang="ts">
  import TrackArtwork from "$lib/components/TrackArtwork.svelte";
  import TrackConvertedBadge from "$lib/components/TrackConvertedBadge.svelte";
  import TrackSourceBadge from "$lib/components/TrackSourceBadge.svelte";
  import type { Track } from "$lib/types";
  import { isTrackSource } from "$lib/trackSource";
  import { displayArtist, displayTitle, formatDuration } from "$lib/format";

  interface Props {
    track: Track;
    selected: boolean;
    readonly?: boolean;
    inRekordbox?: boolean;
    showRowRemove?: boolean;
    onselect: (track: Track) => void;
    onremove?: (track: Track) => void;
    onplay?: (track: Track) => void;
  }

  let {
    track,
    selected,
    readonly = false,
    inRekordbox = false,
    showRowRemove = true,
    onselect,
    onremove,
    onplay,
  }: Props = $props();

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
    {#if isTrackSource(track.source)}
      <span class="source-badge-wrap">
        <TrackSourceBadge source={track.source} />
      </span>
    {/if}
    <div class="card-badges">
      {#if track.converted}
        <TrackConvertedBadge path={track.path} />
      {/if}
      {#if inRekordbox}
        <span class="rb-badge" title="Rekordbox に登録済み">Rekordbox</span>
      {/if}
    </div>
    {#if onplay}
      <button
        type="button"
        class="play-btn"
        onclick={(e) => {
          e.stopPropagation();
          onplay(track);
        }}
        aria-label={`${displayTitle(track)} を再生`}
        title="再生"
      >
        ▶
      </button>
    {/if}
    {#if showRowRemove && !readonly && onremove}
      <button
        type="button"
        class="remove-btn"
        onclick={(e) => {
          e.stopPropagation();
          onremove(track);
        }}
        aria-label="Remove from library"
        title="ライブラリから外す"
      >
        ✕
      </button>
    {/if}
  </div>
  <span class="title">{displayTitle(track)}</span>
  <span class="artist">{displayArtist(track)}</span>
</div>

<style>
  .track-card {
    position: relative;
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
    transition:
      background-color 0.12s ease,
      border-color 0.12s ease;
  }

  .track-card:hover {
    background: var(--surface-hover);
  }

  /* Preview: left accent bar + surface fill. */
  .track-card.selected {
    border-color: transparent;
    background: var(--surface-selected);
  }

  .track-card.selected::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 12px;
    border-radius: 8px 0 0 8px;
    background: var(--accent);
    pointer-events: none;
  }

  .track-card.selected:hover {
    background: var(--surface-selected-hover);
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
    background: var(--overlay);
    color: var(--text);
    font-size: 0.7rem;
    font-variant-numeric: tabular-nums;
    line-height: 1.4;
  }

  .source-badge-wrap {
    position: absolute;
    top: 0.35rem;
    right: 0.35rem;
    z-index: 1;
  }

  .card-badges {
    position: absolute;
    bottom: 0.35rem;
    left: 0.35rem;
    z-index: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    max-width: calc(100% - 4.5rem);
  }

  .rb-badge {
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    background: var(--overlay-strong);
    color: var(--text);
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    line-height: 1.3;
  }

  .play-btn {
    position: absolute;
    top: 50%;
    left: 50%;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border: none;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 1rem;
    cursor: pointer;
    opacity: 0;
    transform: translate(-50%, -50%) scale(0.85);
    transition:
      opacity 0.15s,
      transform 0.15s,
      background-color 0.12s ease;
  }

  .track-card:hover .play-btn,
  .track-card:focus-within .play-btn {
    opacity: 1;
    transform: translate(-50%, -50%) scale(1);
  }

  .play-btn:hover {
    background: var(--accent-hover);
  }

  .remove-btn {
    position: absolute;
    bottom: 0.35rem;
    left: 0.35rem;
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 4px;
    background: var(--overlay);
    color: var(--text);
    cursor: pointer;
    font-size: 0.75rem;
    opacity: 0;
    transition:
      opacity 0.15s,
      background-color 0.12s ease,
      color 0.12s ease;
  }

  .track-card:hover .remove-btn,
  .track-card:focus-within .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    background: var(--danger);
    color: var(--on-accent);
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
