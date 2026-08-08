<script lang="ts">
  import TrackArtwork from "$lib/components/TrackArtwork.svelte";
  import TrackSourceBadge from "$lib/components/TrackSourceBadge.svelte";
  import {
    displayArtist,
    displayTitle,
    displayValue,
    formatBitrate,
    formatBpm,
    formatDuration,
  } from "$lib/format";
  import { isTrackSource } from "$lib/trackSource";
  import type { DuplicateFoundPayload } from "$lib/types";

  const DURATION_TOLERANCE_MS = 3000;

  interface Props {
    payload: DuplicateFoundPayload;
    onchoose: (choice: "existing" | "new") => void | Promise<void>;
  }

  let { payload, onchoose }: Props = $props();

  const existingTitle = displayTitle(payload.existing);
  const candidateTitle = displayTitle(payload.candidate);

  function normalizeField(value: string | null | undefined): string {
    return value?.trim().toLowerCase() ?? "";
  }

  function hasValue(value: string | null | undefined): boolean {
    return Boolean(value?.trim());
  }

  const titleMatches =
    hasValue(payload.existing.title) &&
    hasValue(payload.candidate.title) &&
    normalizeField(payload.existing.title) === normalizeField(payload.candidate.title);

  const artistMatches =
    hasValue(payload.existing.artist) &&
    hasValue(payload.candidate.artist) &&
    normalizeField(payload.existing.artist) === normalizeField(payload.candidate.artist);

  const durationMatches = (() => {
    const existing = payload.existing.durationMs;
    const candidate = payload.candidate.durationMs;
    if (existing == null || candidate == null) return false;
    return Math.abs(existing - candidate) <= DURATION_TOLERANCE_MS;
  })();
</script>

<div class="backdrop" role="presentation">
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label="同じ楽曲の重複。残すトラックを選択"
  >
    <div class="choices">
      <button
        type="button"
        class="choice"
        aria-label={`既存のトラックを残す: ${existingTitle}`}
        onclick={() => onchoose("existing")}
      >
        <span class="role-badge existing">既存</span>
        <div class="choice-body">
          <TrackArtwork
            artworkPath={payload.existing.artworkPath}
            title={existingTitle}
            size={72}
          />
          <div class="meta">
            <div class="title-row">
              <span class="track-title" class:matched={titleMatches}>{existingTitle}</span>
              {#if isTrackSource(payload.existing.source)}
                <TrackSourceBadge source={payload.existing.source} size={14} />
              {/if}
            </div>
            <span class="field artist" class:matched={artistMatches}>
              {displayArtist(payload.existing)}
            </span>
            <span class="field detail">{displayValue(payload.existing.album)}</span>
            <span class="field detail mono" class:matched={durationMatches}>
              {formatDuration(payload.existing.durationMs)}
            </span>
            <span class="field detail mono">{formatBitrate(payload.existing.bitrateKbps)}</span>
            <span class="field detail mono">{formatBpm(payload.existing.bpm)}</span>
            <span class="path" title={payload.existing.path}>{payload.existing.path}</span>
          </div>
        </div>
      </button>

      <div class="relation" aria-hidden="true">
        <div class="relation-icon">
          <span class="relation-square back"></span>
          <span class="relation-square front"></span>
          <span class="relation-equals">=</span>
        </div>
      </div>

      <button
        type="button"
        class="choice"
        aria-label={`新規のトラックで置き換え: ${candidateTitle}`}
        onclick={() => onchoose("new")}
      >
        <span class="role-badge new">新規</span>
        <div class="choice-body">
          <TrackArtwork artworkPath={null} title={candidateTitle} size={72} />
          <div class="meta">
            <div class="title-row">
              <span class="track-title" class:matched={titleMatches}>{candidateTitle}</span>
              {#if isTrackSource(payload.candidate.source)}
                <TrackSourceBadge source={payload.candidate.source} size={14} />
              {/if}
            </div>
            <span class="field artist" class:matched={artistMatches}>
              {displayArtist(payload.candidate)}
            </span>
            <span class="field detail">{displayValue(payload.candidate.album)}</span>
            <span class="field detail mono" class:matched={durationMatches}>
              {formatDuration(payload.candidate.durationMs)}
            </span>
            <span class="field detail mono">{formatBitrate(payload.candidate.bitrateKbps)}</span>
            <span class="field detail mono">{formatBpm(payload.candidate.bpm)}</span>
            <span class="path" title={payload.candidate.path}>{payload.candidate.path}</span>
          </div>
        </div>
      </button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    background: rgba(0, 0, 0, 0.65);
    backdrop-filter: blur(4px);
  }

  .panel {
    width: 100%;
    max-width: 920px;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.45);
    box-sizing: border-box;
    overflow: hidden;
  }

  .choices {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    gap: 0.75rem;
    align-items: center;
    padding: 1.25rem 1.5rem 1.5rem;
    box-sizing: border-box;
  }

  .choice {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
    width: 100%;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    box-sizing: border-box;
    appearance: none;
    transition:
      border-color 0.15s ease,
      background 0.15s ease;
  }

  .choice:hover {
    border-color: var(--accent);
    background: var(--accent-subtle);
  }

  .choice:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .role-badge {
    align-self: flex-start;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    font-size: 0.6875rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    line-height: 1.2;
  }

  .role-badge.existing {
    background: var(--surface-hover);
    color: var(--text-muted);
  }

  .role-badge.new {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
  }

  .relation {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 0.15rem;
  }

  .relation-icon {
    position: relative;
    width: 2.5rem;
    height: 2.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .relation-square {
    position: absolute;
    width: 1.35rem;
    height: 1.35rem;
    border-radius: 4px;
    border: 1.5px solid var(--accent);
    background: var(--surface);
  }

  .relation-square.back {
    top: 0.15rem;
    left: 0;
    opacity: 0.55;
  }

  .relation-square.front {
    bottom: 0.15rem;
    right: 0;
    background: var(--accent-subtle);
  }

  .relation-equals {
    position: relative;
    z-index: 1;
    font-size: 0.875rem;
    font-weight: 700;
    color: var(--accent);
    line-height: 1;
  }

  .choice-body {
    display: flex;
    gap: 0.875rem;
    align-items: flex-start;
    min-width: 0;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
    flex: 1;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
  }

  .track-title {
    min-width: 0;
    font-size: 0.95rem;
    font-weight: 600;
    line-height: 1.3;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .field {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border-radius: 3px;
    padding: 0 0.2rem;
    margin: 0 -0.2rem;
  }

  .field.matched {
    background: var(--accent-subtle);
    color: var(--text);
  }

  .artist {
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  .detail {
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  .mono {
    font-variant-numeric: tabular-nums;
  }

  .path {
    margin-top: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    opacity: 0.8;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0;
  }

  @media (max-width: 720px) {
    .choices {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto auto auto;
    }

    .relation {
      padding: 0.25rem 0;
    }

    .relation-icon {
      transform: rotate(90deg);
    }
  }
</style>
