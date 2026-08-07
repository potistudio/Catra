<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { Track } from "$lib/types";
  import { displayArtist, displayTitle, formatDuration } from "$lib/format";

  interface Props {
    track: Track | null;
  }

  let { track }: Props = $props();

  let audioEl: HTMLAudioElement | undefined = $state();
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let audioSrc = $state<string | null>(null);

  $effect(() => {
    if (!track) {
      audioSrc = null;
      isPlaying = false;
      currentTime = 0;
      duration = 0;
      return;
    }

    audioSrc = convertFileSrc(track.path);
    isPlaying = false;
    currentTime = 0;
    duration = track.durationMs ? track.durationMs / 1000 : 0;
  });

  function togglePlay() {
    if (!audioEl || !audioSrc) return;

    if (isPlaying) {
      audioEl.pause();
    } else {
      void audioEl.play();
    }
  }

  function handleTimeUpdate() {
    if (!audioEl) return;
    currentTime = audioEl.currentTime;
  }

  function handleLoadedMetadata() {
    if (!audioEl) return;
    if (Number.isFinite(audioEl.duration)) {
      duration = audioEl.duration;
    }
  }

  function handleSeek(event: Event) {
    if (!audioEl) return;
    const input = event.target as HTMLInputElement;
    audioEl.currentTime = Number(input.value);
    currentTime = audioEl.currentTime;
  }

  function handleEnded() {
    isPlaying = false;
    currentTime = 0;
    if (audioEl) {
      audioEl.currentTime = 0;
    }
  }
</script>

<footer class="preview">
  {#if track && audioSrc}
    <audio
      bind:this={audioEl}
      src={audioSrc}
      onplay={() => (isPlaying = true)}
      onpause={() => (isPlaying = false)}
      ontimeupdate={handleTimeUpdate}
      onloadedmetadata={handleLoadedMetadata}
      onended={handleEnded}
    ></audio>

    <div class="preview-info">
      <span class="preview-title">{displayTitle(track)}</span>
      <span class="preview-artist">{displayArtist(track)}</span>
      {#if track.bpm}
        <span class="preview-bpm">{track.bpm.toFixed(1)} BPM</span>
      {/if}
    </div>

    <div class="preview-controls">
      <button class="play-btn" onclick={togglePlay} aria-label={isPlaying ? "Pause" : "Play"}>
        {isPlaying ? "⏸" : "▶"}
      </button>

      <span class="time">{formatDuration(currentTime * 1000)}</span>

      <input
        class="seek"
        type="range"
        min="0"
        max={duration || 0}
        step="0.1"
        value={currentTime}
        oninput={handleSeek}
        disabled={!duration}
      />

      <span class="time">{formatDuration(duration * 1000)}</span>
    </div>
  {:else}
    <p class="preview-empty">Select a track to preview</p>
  {/if}
</footer>

<style>
  .preview {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid var(--border);
    background: var(--surface-raised);
    min-height: 72px;
  }

  .preview-empty {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .preview-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 200px;
    max-width: 280px;
  }

  .preview-title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preview-artist {
    font-size: 0.85rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preview-bpm {
    font-size: 0.75rem;
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }

  .preview-controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
  }

  .play-btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: var(--accent);
    color: #fff;
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .play-btn:hover {
    filter: brightness(1.1);
  }

  .time {
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    min-width: 2.5rem;
    text-align: center;
  }

  .seek {
    flex: 1;
    accent-color: var(--accent);
    cursor: pointer;
  }
</style>
