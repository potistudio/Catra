<script lang="ts">
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onDestroy, onMount } from "svelte";
import TrackArtwork from "$lib/components/TrackArtwork.svelte";
import {
	displayArtist,
	displayTitle,
	displayValue,
	formatBitrate,
	formatBpm,
	formatDuration,
	formatRating,
} from "$lib/format";
import type { PreviewableTrack } from "$lib/types";

interface Props {
	track: PreviewableTrack | null;
	/** 変わるたびに、選択中のトラックをすぐ再生する合図。 */
	autoplayToken?: number;
}

let { track, autoplayToken = 0 }: Props = $props();

let audioEl: HTMLAudioElement | undefined = $state();
let isPlaying = $state(false);
let currentTime = $state(0);
let duration = $state(0);
let audioSrc = $state<string | null>(null);
let pendingAutoplay = $state(false);
let playbackRate = $state(1);
let masterTempo = $state(true);
let volume = $state(1);
let isMuted = $state(false);
let lastAutoplayToken = 0;
let lastPath: string | null = null;
let unlistenNativeVolumeDrag: UnlistenFn | null = null;
let usesNativeVolumeDrag = false;
let nextVolumeDragId = 0;
interface VolumeDrag {
	control: HTMLButtonElement;
	dragId: number;
	pointerId: number;
	rawVolume: number;
	startPromise: Promise<boolean>;
}
let volumeDrag: VolumeDrag | null = null;

const MIN_PLAYBACK_RATE = 0.25;
const MAX_PLAYBACK_RATE = 4;
const VOLUME_DRAG_DISTANCE = 480;

let playbackBpm = $derived(
	track?.bpm != null && track.bpm > 0 ? track.bpm * playbackRate : null,
);
let isEffectivelyMuted = $derived(isMuted || volume === 0);

$effect(() => {
	if (!track) {
		audioSrc = null;
		isPlaying = false;
		currentTime = 0;
		duration = 0;
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
		currentTime = 0;
		duration = track.durationMs ? track.durationMs / 1000 : 0;
		pendingAutoplay = shouldAutoplay;
		return;
	}

	if (shouldAutoplay && audioEl) {
		currentTime = 0;
		audioEl.currentTime = 0;
		void audioEl.play();
	}
});

$effect(() => {
	if (!audioEl) return;
	audioEl.playbackRate = playbackRate;
	audioEl.preservesPitch = masterTempo;
	audioEl.volume = volume;
	audioEl.muted = isMuted;
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
	if (pendingAutoplay) {
		pendingAutoplay = false;
		void audioEl.play();
	}
}

function handleSeek(event: Event) {
	if (!audioEl) return;
	const input = event.target as HTMLInputElement;
	audioEl.currentTime = Number(input.value);
	currentTime = audioEl.currentTime;
}

function handlePlaybackRateChange(event: Event) {
	const input = event.target as HTMLInputElement;
	const requestedRate = Number.parseFloat(input.value);

	if (!Number.isFinite(requestedRate)) {
		input.value = String(playbackRate);
		return;
	}

	playbackRate = Math.min(
		MAX_PLAYBACK_RATE,
		Math.max(MIN_PLAYBACK_RATE, requestedRate),
	);
	input.value = String(playbackRate);
}

function handlePlaybackBpmChange(event: Event) {
	const input = event.target as HTMLInputElement;
	const requestedBpm = Number.parseFloat(input.value);
	const sourceBpm = track?.bpm;

	if (!Number.isFinite(requestedBpm) || sourceBpm == null || sourceBpm <= 0) {
		input.value = playbackBpm === null ? "" : formatBpm(playbackBpm);
		return;
	}

	playbackRate = Math.min(
		MAX_PLAYBACK_RATE,
		Math.max(MIN_PLAYBACK_RATE, requestedBpm / sourceBpm),
	);
	input.value = formatBpm(sourceBpm * playbackRate);
}

function handleVolumeChange(event: Event) {
	const input = event.target as HTMLInputElement;
	setVolume(Number(input.value));
}

function setVolume(nextVolume: number) {
	volume = Math.min(1, Math.max(0, Math.round(nextVolume * 100) / 100));
	if (volume > 0) isMuted = false;
}

function handleVolumePointerDown(event: PointerEvent) {
	if (event.button !== 0) return;
	const control = event.currentTarget as HTMLButtonElement;
	const dragId = ++nextVolumeDragId;
	const startPromise = invoke<boolean>("begin_native_volume_drag", {
		dragId,
	}).catch(() => false);
	volumeDrag = {
		control,
		dragId,
		pointerId: event.pointerId,
		rawVolume: volume,
		startPromise,
	};
	control.focus();
	control.setPointerCapture(event.pointerId);
	document.documentElement.classList.add("volume-knob-dragging");
	void startPromise.then((enabled) => {
		if (volumeDrag?.dragId === dragId) {
			usesNativeVolumeDrag = enabled;
		}
	});
}

function handleVolumePointerMove(event: PointerEvent) {
	const drag = volumeDrag;
	if (drag?.pointerId !== event.pointerId) return;
	event.preventDefault();
	if (!usesNativeVolumeDrag && event.movementY !== 0) {
		applyVolumeDragDelta(-event.movementY);
	}
}

function handleVolumePointerEnd(event: PointerEvent) {
	if (volumeDrag?.pointerId !== event.pointerId) return;
	finishVolumeDrag();
}

function finishVolumeDrag() {
	if (!volumeDrag) return;
	const drag = volumeDrag;
	const { control, dragId, pointerId, startPromise } = drag;
	volumeDrag = null;
	usesNativeVolumeDrag = false;
	if (control.hasPointerCapture(pointerId)) {
		control.releasePointerCapture(pointerId);
	}
	document.documentElement.classList.remove("volume-knob-dragging");
	void startPromise.then(() =>
		invoke("end_native_volume_drag", { dragId }).catch(() => undefined),
	);
}

function applyVolumeDragDelta(deltaY: number) {
	if (!volumeDrag || deltaY === 0) return;
	volumeDrag.rawVolume = Math.min(
		1,
		Math.max(0, volumeDrag.rawVolume + deltaY / VOLUME_DRAG_DISTANCE),
	);
	setVolume(volumeDrag.rawVolume);
}

function toggleMute() {
	if (isMuted) {
		isMuted = false;
	} else if (volume === 0) {
		volume = 0.5;
	} else {
		isMuted = true;
	}
}

function handleEnded() {
	isPlaying = false;
	currentTime = 0;
	if (audioEl) {
		audioEl.currentTime = 0;
	}
}

onMount(() => {
	void listen<{ deltaY: number; dragId: number }>(
		"native-volume-drag",
		({ payload }) => {
			if (volumeDrag?.dragId === payload.dragId) {
				applyVolumeDragDelta(payload.deltaY);
			}
		},
	).then((unlisten) => {
		unlistenNativeVolumeDrag = unlisten;
	});
});

onDestroy(() => {
	unlistenNativeVolumeDrag?.();
	if (typeof document !== "undefined") {
		finishVolumeDrag();
		document.documentElement.classList.remove("volume-knob-dragging");
	}
});
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

      <div class="rate-control">
        <label class="rate-field">
          <span class="rate-label">速度</span>
          <input
            class="rate-input"
            type="number"
            min={MIN_PLAYBACK_RATE}
            max={MAX_PLAYBACK_RATE}
            step="0.05"
            value={playbackRate}
            onchange={handlePlaybackRateChange}
            aria-label="再生速度"
          />
          <span>×</span>
        </label>

        <label class="bpm-field" title={playbackBpm === null ? "元BPMがないため指定できません" : "再生BPMを直接指定"}>
          <input
            class="bpm-input"
            type="number"
            min={track.bpm ? track.bpm * MIN_PLAYBACK_RATE : undefined}
            max={track.bpm ? track.bpm * MAX_PLAYBACK_RATE : undefined}
            step="0.1"
            value={playbackBpm === null ? "" : playbackBpm.toFixed(1)}
            onchange={handlePlaybackBpmChange}
            placeholder="—"
            disabled={playbackBpm === null}
            aria-label="再生BPM"
          />
          <span>BPM</span>
        </label>
      </div>

      <button
        class="master-tempo"
        class:active={masterTempo}
        type="button"
        onclick={() => (masterTempo = !masterTempo)}
        aria-label="音程維持"
        aria-pressed={masterTempo}
        title={masterTempo ? "Master Tempo オン（音程を維持）" : "Master Tempo オフ（速度に応じて音程を変更）"}
      >
        MT
      </button>

      <div class="volume-knob">
        <span class="volume-value" aria-hidden="true">{Math.round(volume * 100)}%</span>
        <svg
          class="knob-ring"
          class:muted={isEffectivelyMuted}
          viewBox="0 0 56 56"
          aria-hidden="true"
        >
          <path class="knob-track" pathLength="100" d="M11.03 44.97A24 24 0 1 1 44.97 44.97"></path>
          <path
            class="knob-level"
            pathLength="100"
            stroke-dasharray={`${volume * 100} 100`}
            d="M11.03 44.97A24 24 0 1 1 44.97 44.97"
          ></path>
        </svg>
        <input
          class="volume-knob-input"
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={volume}
          oninput={handleVolumeChange}
          aria-label="音量"
          aria-valuetext={`${Math.round(volume * 100)}%`}
          title="上下ドラッグまたは矢印キーで音量調整"
        />
        <button
          class="mute-btn"
          type="button"
          onclick={toggleMute}
          aria-label={isEffectivelyMuted ? "ミュート解除" : "ミュート"}
          aria-pressed={isEffectivelyMuted}
          title={isEffectivelyMuted ? "ミュート解除" : "ミュート"}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M11 5 6 9H2v6h4l5 4V5Z"></path>
            <path
              class="sound-wave near"
              class:visible={!isEffectivelyMuted}
              d="M15.5 8.5a5 5 0 0 1 0 7"
            ></path>
            <path
              class="sound-wave far"
              class:visible={!isEffectivelyMuted && volume >= 0.5}
              d="M19 5a10 10 0 0 1 0 14"
            ></path>
            <path class="mute-mark" class:visible={isEffectivelyMuted} d="m16 9 6 6"></path>
            <path class="mute-mark" class:visible={isEffectivelyMuted} d="m22 9-6 6"></path>
          </svg>
        </button>
        <button
          class="volume-drag-handle"
          type="button"
          onpointerdown={handleVolumePointerDown}
          onpointermove={handleVolumePointerMove}
          onpointerup={handleVolumePointerEnd}
          onpointercancel={handleVolumePointerEnd}
          aria-label="音量調整"
          title="上下ドラッグで音量調整"
        >
          <svg viewBox="0 0 16 20" aria-hidden="true">
            <path d="M6 3.5h4"></path>
            <path d="M4.5 6.75h7"></path>
            <path d="M3.5 10h9"></path>
            <path d="M4.5 13.25h7"></path>
            <path d="M6 16.5h4"></path>
          </svg>
        </button>
      </div>
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

  .rate-control {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }

  .rate-field,
  .bpm-field {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .rate-label {
    font-size: 0.72rem;
  }

  .rate-input,
  .bpm-input {
    width: 3.75rem;
    padding: 0.25rem 0.3rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .bpm-input {
    width: 4.5rem;
  }

  .rate-input:focus,
  .bpm-input:focus {
    border-color: var(--accent);
    outline: none;
  }

  .bpm-input:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .master-tempo {
    padding: 0.3rem 0.45rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface);
    color: var(--text-muted);
    font-size: 0.72rem;
    font-weight: 700;
    cursor: pointer;
  }

  .master-tempo:hover {
    border-color: var(--accent);
  }

  .master-tempo.active {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }

  .volume-knob {
    position: relative;
    width: 3.5rem;
    height: 3.5rem;
    flex-shrink: 0;
  }

  .mute-btn {
    position: absolute;
    top: 50%;
    left: 50%;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.15rem;
    height: 2.15rem;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--surface-raised);
    color: var(--text-muted);
    cursor: pointer;
    transform: translate(-50%, -50%);
  }

  .mute-btn:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .mute-btn svg {
    width: 1.2rem;
    height: 1.2rem;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2;
  }

  .mute-btn .sound-wave,
  .mute-btn .mute-mark {
    opacity: 0;
    transition: opacity 140ms ease;
  }

  .mute-btn .sound-wave.far {
    transition-delay: 40ms;
  }

  .mute-btn .sound-wave.visible,
  .mute-btn .mute-mark.visible {
    opacity: 1;
  }

  .volume-drag-handle {
    position: absolute;
    top: 50%;
    right: -0.05rem;
    z-index: 3;
    display: grid;
    width: 1.35rem;
    height: 1.35rem;
    padding: 0;
    border: 0;
    place-items: center;
    background: transparent;
    color: var(--text-muted);
    cursor: ns-resize;
    touch-action: none;
    transform: translateY(-50%);
  }

  .volume-drag-handle svg {
    width: 0.9rem;
    height: 1.15rem;
    overflow: visible;
    fill: none;
    filter: drop-shadow(0 1px 1px rgb(0 0 0 / 35%));
    stroke: currentColor;
    stroke-linecap: round;
    stroke-width: 1.75;
    transition:
      color 120ms ease,
      filter 120ms ease,
      transform 160ms cubic-bezier(0.2, 1.65, 0.3, 1);
  }

  .volume-drag-handle:hover svg,
  .volume-drag-handle:focus-visible svg {
    color: var(--accent);
    filter:
      drop-shadow(0 1px 1px rgb(0 0 0 / 35%))
      drop-shadow(0 0 2px var(--accent));
    transform: scale(1.12);
  }

  .volume-drag-handle:active svg {
    transform: scale(0.94);
  }

  .knob-ring {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: none;
    opacity: 0;
    pointer-events: none;
    stroke-linecap: round;
    stroke-width: 4;
    transform: scale(0.68);
    transform-box: fill-box;
    transform-origin: center;
    transition:
      opacity 100ms ease-out,
      transform 240ms cubic-bezier(0.2, 1.65, 0.3, 1);
  }

  .knob-track {
    stroke: var(--border);
  }

  .knob-level {
    stroke: var(--accent);
  }

  .volume-knob:has(.volume-drag-handle:hover) .knob-ring {
    opacity: 1;
    transform: scale(1);
  }

  .volume-knob:has(.volume-drag-handle:hover) .knob-ring.muted {
    opacity: 0.45;
  }

  .volume-value {
    position: absolute;
    bottom: calc(100% + 0.35rem);
    left: 50%;
    z-index: 3;
    padding: 0.2rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    opacity: 0;
    pointer-events: none;
    transform: translate(-50%, 0.2rem);
    transition:
      opacity 100ms ease,
      transform 100ms ease;
  }

  .volume-knob:has(.volume-drag-handle:hover) .volume-value {
    opacity: 1;
    transform: translate(-50%, 0);
  }

  .volume-knob-input {
    position: absolute;
    inset: 0;
    z-index: 1;
    width: 100%;
    height: 100%;
    margin: 0;
    cursor: ns-resize;
    opacity: 0;
    pointer-events: none;
    touch-action: none;
  }

  .volume-knob:focus-within .knob-ring {
    filter: drop-shadow(0 0 2px var(--accent));
  }

  :global(html.volume-knob-dragging),
  :global(html.volume-knob-dragging *) {
    cursor: none !important;
  }

  @media (prefers-reduced-motion: reduce) {
    .mute-btn svg,
    .mute-btn .sound-wave,
    .mute-btn .mute-mark,
    .volume-drag-handle svg,
    .knob-ring {
      transition: none;
    }
  }
</style>
