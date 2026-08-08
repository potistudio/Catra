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

  interface Props {
    payload: DuplicateFoundPayload;
    onchoose: (choice: "existing" | "new") => void | Promise<void>;
  }

  let { payload, onchoose }: Props = $props();

  const existingTitle = displayTitle(payload.existing);
  const candidateTitle = displayTitle(payload.candidate);
</script>

<div class="backdrop" role="presentation">
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby="duplicate-title">
    <header class="header">
      <h2 id="duplicate-title">重複した楽曲が見つかりました</h2>
      <p class="subtitle">同じ楽曲と判断されたトラックがあります。</p>
    </header>

    <div class="choices">
      <button
        type="button"
        class="choice"
        aria-label={`ライブラリ内のトラックを残す: ${existingTitle}`}
        onclick={() => onchoose("existing")}
      >
        <span class="choice-label">ライブラリ内</span>
        <div class="choice-body">
          <TrackArtwork
            artworkPath={payload.existing.artworkPath}
            title={existingTitle}
            size={72}
          />
          <div class="meta">
            <div class="title-row">
              <span class="track-title">{existingTitle}</span>
              {#if isTrackSource(payload.existing.source)}
                <TrackSourceBadge source={payload.existing.source} size={14} />
              {/if}
            </div>
            <span class="artist">{displayArtist(payload.existing)}</span>
            <span class="detail">{displayValue(payload.existing.album)}</span>
            <span class="detail mono">
              {formatDuration(payload.existing.durationMs)} · {formatBitrate(payload.existing.bitrateKbps)} · {formatBpm(payload.existing.bpm)}
            </span>
            <span class="path" title={payload.existing.path}>{payload.existing.path}</span>
          </div>
        </div>
      </button>

      <button
        type="button"
        class="choice"
        aria-label={`新しいトラックを残す: ${candidateTitle}`}
        onclick={() => onchoose("new")}
      >
        <span class="choice-label">新規</span>
        <div class="choice-body">
          <TrackArtwork artworkPath={null} title={candidateTitle} size={72} />
          <div class="meta">
            <div class="title-row">
              <span class="track-title">{candidateTitle}</span>
              {#if isTrackSource(payload.candidate.source)}
                <TrackSourceBadge source={payload.candidate.source} size={14} />
              {/if}
            </div>
            <span class="artist">{displayArtist(payload.candidate)}</span>
            <span class="detail">{displayValue(payload.candidate.album)}</span>
            <span class="detail mono">
              {formatDuration(payload.candidate.durationMs)} · {formatBitrate(payload.candidate.bitrateKbps)} · {formatBpm(payload.candidate.bpm)}
            </span>
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

  .header {
    padding: 1.25rem 1.5rem 0.75rem;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .subtitle {
    margin: 0.35rem 0 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  .choices {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 1rem;
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

  .choice-label {
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
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

  .artist,
  .detail {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
  }

  @media (max-width: 720px) {
    .choices {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
