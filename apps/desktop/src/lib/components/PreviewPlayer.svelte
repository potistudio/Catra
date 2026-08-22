<script lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { PreviewableTrack } from "$lib/types";

interface Props {
	track: PreviewableTrack | null;
	/** 変わるたびに、選択中のトラックをすぐ再生する合図。 */
	autoplayToken?: number;
}

let { track, autoplayToken = 0 }: Props = $props();

let audioEl: HTMLAudioElement | undefined = $state();
let isPlaying = $state(false);
let _currentTime = $state(0);
let _duration = $state(0);
let audioSrc = $state<string | null>(null);
let pendingAutoplay = $state(false);
let lastAutoplayToken = 0;
let lastPath: string | null = null;

$effect(() => {
	if (!track) {
		audioSrc = null;
		isPlaying = false;
		_currentTime = 0;
		_duration = 0;
		pendingAutoplay = false;
		lastPath = null;
		return;
	}

	const trackChanged = track.path !== lastPath;
	lastPath = track.path;

	const shouldAutoplay = autoplayToken !== lastAutoplayToken;
	lastAutoplayToken = autoplayToken;

	if (trackChanged) {
		// 曲そのものが変わるときだけ src を入れ替える。同じ曲なら src は
		// 文字列として同一なので DOM は更新されず、loadedmetadata も
		// 発火しない。だから即再生できるケースはここを通さない。
		audioSrc = convertFileSrc(track.path);
		isPlaying = false;
		_currentTime = 0;
		_duration = track.durationMs ? track.durationMs / 1000 : 0;
		pendingAutoplay = shouldAutoplay;
		return;
	}

	if (shouldAutoplay && audioEl) {
		_currentTime = 0;
		audioEl.currentTime = 0;
		void audioEl.play();
	}
});

function _togglePlay() {
	if (!audioEl || !audioSrc) return;

	if (isPlaying) {
		audioEl.pause();
	} else {
		void audioEl.play();
	}
}

function _handleTimeUpdate() {
	if (!audioEl) return;
	_currentTime = audioEl.currentTime;
}

function _handleLoadedMetadata() {
	if (!audioEl) return;
	if (Number.isFinite(audioEl.duration)) {
		_duration = audioEl.duration;
	}
	if (pendingAutoplay) {
		pendingAutoplay = false;
		void audioEl.play();
	}
}

function _handleSeek(event: Event) {
	if (!audioEl) return;
	const input = event.target as HTMLInputElement;
	audioEl.currentTime = Number(input.value);
	_currentTime = audioEl.currentTime;
}

function _handleEnded() {
	isPlaying = false;
	_currentTime = 0;
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

    <TrackArtwork
      artworkPath={track.artworkPath ?? null}
      title={displayTitle(track)}
      size={56}
    />

    <div class="preview-info">
      <span class="preview-title">{displayTitle(track)}</span>
      <span class="preview-artist">{displayArtist(track)}</span>
      <span class="preview-album">{displayValue(track.album)}</span>
      <div class="preview-meta">
        <span>BPM {formatBpm(track.bpm)}</span>
        <span>{formatBitrate(track.bitrateKbps)}</span>
        <span>Key {displayValue(track.key)}</span>
        <span>{displayValue(track.genre)}</span>
        <span class="rating">{formatRating(track.rating)}</span>
      </div>
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
    <p class="preview-empty">トラックを選択してプレビュー</p>
  {/if}
</footer>

<style>
  .preview {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    border-top: 1px solid var(--border);
    background: var(--surface-raised);
    min-height: 80px;
  }

  .preview-empty {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .preview-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex: 0 0 280px;
    width: 280px;
    min-width: 0;
    overflow: hidden;
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

  .preview-album {
    font-size: 0.8rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preview-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.75rem;
    margin-top: 0.15rem;
    font-size: 0.72rem;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .preview-meta .rating {
    color: #f0c040;
  }

  .preview-controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
  }

  .play-btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .play-btn:hover {
    background: var(--accent-hover);
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
