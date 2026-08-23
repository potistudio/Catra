<script lang="ts">
interface Props {
	source: string;
	currentTime: number;
	duration: number;
	onseek: (seconds: number) => void;
}

let { source, currentTime, duration, onseek }: Props = $props();

const BAR_COUNT = 120;
const PLACEHOLDER_PEAKS = Array.from(
	{ length: BAR_COUNT },
	(_, index) => 0.12 + Math.abs(Math.sin(index * 0.37)) * 0.16,
);

let peaks = $state<number[]>([]);
let isLoading = $state(false);
let loadFailed = $state(false);
let activePointerId: number | null = null;
let displayedPeaks = $derived(peaks.length > 0 ? peaks : PLACEHOLDER_PEAKS);
let progress = $derived(
	duration > 0 ? Math.min(1, Math.max(0, currentTime / duration)) : 0,
);

$effect(() => {
	const requestedSource = source;
	const controller = new AbortController();
	peaks = [];
	isLoading = true;
	loadFailed = false;

	void createPeaks(requestedSource, controller.signal)
		.then((nextPeaks) => {
			if (controller.signal.aborted) return;
			peaks = nextPeaks;
		})
		.catch(() => {
			if (!controller.signal.aborted) loadFailed = true;
		})
		.finally(() => {
			if (!controller.signal.aborted) isLoading = false;
		});

	return () => controller.abort();
});

async function createPeaks(
	requestedSource: string,
	signal: AbortSignal,
): Promise<number[]> {
	const response = await fetch(requestedSource, { signal });
	if (!response.ok)
		throw new Error(`音声の読み込みに失敗しました: ${response.status}`);

	const audioData = await response.arrayBuffer();
	if (signal.aborted) throw new DOMException("Aborted", "AbortError");

	const audioContext = new AudioContext();
	try {
		const audioBuffer = await audioContext.decodeAudioData(audioData);
		return samplePeaks(audioBuffer);
	} finally {
		void audioContext.close();
	}
}

function samplePeaks(audioBuffer: AudioBuffer): number[] {
	const channelData = Array.from(
		{ length: audioBuffer.numberOfChannels },
		(_, channel) => audioBuffer.getChannelData(channel),
	);
	const samplesPerBar = Math.max(1, Math.floor(audioBuffer.length / BAR_COUNT));
	const sampleStride = Math.max(1, Math.floor(samplesPerBar / 800));
	const nextPeaks = Array.from({ length: BAR_COUNT }, (_, barIndex) => {
		const start = barIndex * samplesPerBar;
		const end = Math.min(audioBuffer.length, start + samplesPerBar);
		let peak = 0;

		for (let sample = start; sample < end; sample += sampleStride) {
			for (const channel of channelData) {
				peak = Math.max(peak, Math.abs(channel[sample] ?? 0));
			}
		}

		return peak;
	});
	const loudestPeak = Math.max(...nextPeaks, 0.01);

	return nextPeaks.map((peak) => Math.max(0.08, peak / loudestPeak));
}

function handleSeek(event: Event) {
	const input = event.currentTarget as HTMLInputElement;
	onseek(Number(input.value));
}

function handleSeekPointerDown(event: PointerEvent) {
	if (event.button !== 0 || duration <= 0) return;

	const input = event.currentTarget as HTMLInputElement;
	event.preventDefault();
	activePointerId = event.pointerId;
	input.focus();
	input.setPointerCapture(event.pointerId);
	seekToPointer(input, event.clientX);
}

function handleSeekPointerMove(event: PointerEvent) {
	if (activePointerId !== event.pointerId) return;
	event.preventDefault();
	seekToPointer(event.currentTarget as HTMLInputElement, event.clientX);
}

function handleSeekPointerEnd(event: PointerEvent) {
	if (activePointerId !== event.pointerId) return;
	activePointerId = null;
}

function seekToPointer(input: HTMLInputElement, clientX: number) {
	const bounds = input.getBoundingClientRect();
	const ratio = Math.min(
		1,
		Math.max(0, (clientX - bounds.left) / bounds.width),
	);
	onseek(ratio * duration);
}
</script>

<div
	class="waveform"
	class:loading={isLoading}
	class:failed={loadFailed}
	title={loadFailed ? "波形を読み込めませんでした" : "クリックまたはドラッグでシーク"}
>
	<svg class="waveform-layer waveform-base" viewBox="0 0 100 32" preserveAspectRatio="none" aria-hidden="true">
		{#each displayedPeaks as peak, index}
			{@const barHeight = peak * 28}
			<rect
				x={(index * 100) / BAR_COUNT}
				y={(32 - barHeight) / 2}
				width={65 / BAR_COUNT}
				height={barHeight}
				rx="0.22"
			/>
		{/each}
	</svg>

	<svg
		class="waveform-layer waveform-played"
		viewBox="0 0 100 32"
		preserveAspectRatio="none"
		style={`clip-path: inset(0 ${(1 - progress) * 100}% 0 0)`}
		aria-hidden="true"
	>
		{#each displayedPeaks as peak, index}
			{@const barHeight = peak * 28}
			<rect
				x={(index * 100) / BAR_COUNT}
				y={(32 - barHeight) / 2}
				width={65 / BAR_COUNT}
				height={barHeight}
				rx="0.22"
			/>
		{/each}
	</svg>

	<input
		class="waveform-input"
		type="range"
		min="0"
		max={duration || 0}
		step="0.1"
		value={currentTime}
		oninput={handleSeek}
		onpointerdown={handleSeekPointerDown}
		onpointermove={handleSeekPointerMove}
		onpointerup={handleSeekPointerEnd}
		onpointercancel={handleSeekPointerEnd}
		disabled={!duration}
		aria-label="再生位置"
	/>
</div>

<style>
	.waveform {
		position: relative;
		flex: 1;
		min-width: 5rem;
		height: 2.5rem;
		border-radius: 0.25rem;
		overflow: hidden;
	}

	.waveform-layer {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.waveform-base {
		fill: color-mix(in srgb, var(--text-muted) 58%, transparent);
	}

	.waveform-played {
		fill: var(--accent);
	}

	.waveform-input {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		margin: 0;
		cursor: pointer;
		opacity: 0;
		touch-action: none;
	}

	.waveform:has(.waveform-input:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}

	.waveform:has(.waveform-input:disabled) {
		cursor: not-allowed;
		opacity: 0.55;
	}

	.waveform.loading .waveform-layer {
		animation: waveform-pulse 1.1s ease-in-out infinite alternate;
	}

	.waveform.failed .waveform-base {
		fill: color-mix(in srgb, var(--text-muted) 35%, transparent);
	}

	@keyframes waveform-pulse {
		from {
			opacity: 0.35;
		}
		to {
			opacity: 0.75;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.waveform.loading .waveform-layer {
			animation: none;
		}
	}
</style>
