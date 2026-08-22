<script lang="ts">
import type { ConvertFormat, ConvertOptions } from "$lib/types";

const FORMATS: { value: ConvertFormat; label: string }[] = [
	{ value: "mp3", label: "MP3" },
	{ value: "aac", label: "AAC (M4A)" },
	{ value: "flac", label: "FLAC" },
	{ value: "wav", label: "WAV" },
	{ value: "aiff", label: "AIFF" },
	{ value: "ogg", label: "OGG Vorbis" },
	{ value: "opus", label: "Opus" },
];

const BITRATES = [128, 192, 256, 320];
const LOSSY_FORMATS = new Set<ConvertFormat>(["mp3", "aac", "ogg", "opus"]);

interface Props {
	trackCount: number;
	rekordboxWritable?: boolean;
	rekordboxLockedHint?: string | null;
	oncancel: () => void;
	onconfirm: (options: ConvertOptions) => void | Promise<void>;
}

let {
	trackCount,
	rekordboxWritable = false,
	rekordboxLockedHint = null,
	oncancel,
	onconfirm,
}: Props = $props();

let format = $state<ConvertFormat>("mp3");
let bitrateKbps = $state("320");
let bitDepth = $state("16");
let sampleRate = $state("keep");
let channels = $state("keep");
let addToRekordbox = $state(true);
let backdropDismissArmed = $state(false);

let lossy = $derived(LOSSY_FORMATS.has(format));

function onBackdropPointerDown(event: PointerEvent) {
	backdropDismissArmed = event.target === event.currentTarget;
}

function onBackdropPointerUp(event: PointerEvent) {
	if (backdropDismissArmed && event.target === event.currentTarget) {
		oncancel();
	}
	backdropDismissArmed = false;
}

function submit() {
	const options: ConvertOptions = {
		format,
		bitrateKbps: lossy ? Number(bitrateKbps) : null,
		bitDepth: lossy ? null : Number(bitDepth),
		sampleRate: sampleRate === "keep" ? null : Number(sampleRate),
		channels: channels === "keep" ? null : Number(channels),
		addToRekordbox: rekordboxWritable && addToRekordbox,
	};
	void onconfirm(options);
}
</script>

<div
  class="modal-backdrop"
  role="presentation"
  onpointerdown={onBackdropPointerDown}
  onpointerup={onBackdropPointerUp}
>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="convert-title">
    <h3 id="convert-title">楽曲を変換</h3>
    <p class="hint">
      {trackCount} 曲を変換します。元のファイルは残し、変換後のファイルをライブラリに追加します。
    </p>
    <label>
      フォーマット
      <select bind:value={format}>
        {#each FORMATS as item (item.value)}
          <option value={item.value}>{item.label}</option>
        {/each}
      </select>
    </label>
    {#if lossy}
      <label>
        ビットレート
        <select bind:value={bitrateKbps}>
          {#each BITRATES as rate (rate)}
            <option value={String(rate)}>{rate} kbps</option>
          {/each}
        </select>
      </label>
    {:else}
      <label>
        ビット深度
        <select bind:value={bitDepth}>
          <option value="16">16 bit</option>
          <option value="24">24 bit</option>
        </select>
      </label>
    {/if}
    <label>
      サンプルレート
      <select bind:value={sampleRate}>
        <option value="keep">元のまま</option>
        <option value="44100">44100 Hz</option>
        <option value="48000">48000 Hz</option>
      </select>
    </label>
    <label>
      チャンネル
      <select bind:value={channels}>
        <option value="keep">元のまま</option>
        <option value="2">ステレオ</option>
        <option value="1">モノラル</option>
      </select>
    </label>
    <label class="check-row">
      <input
        type="checkbox"
        checked={rekordboxWritable && addToRekordbox}
        disabled={!rekordboxWritable}
        onchange={(event) => {
          addToRekordbox = event.currentTarget.checked;
        }}
      />
      変換後のファイルを Rekordbox に追加
    </label>
    {#if !rekordboxWritable && rekordboxLockedHint}
      <p class="hint">{rekordboxLockedHint}</p>
    {/if}
    <div class="modal-actions">
      <button type="button" class="modal-btn" onclick={oncancel}>キャンセル</button>
      <button type="button" class="modal-btn primary" onclick={submit}>変換</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 20;
  }

  .modal {
    width: min(28rem, calc(100% - 2rem));
    max-height: calc(100% - 2rem);
    overflow: auto;
    background: var(--surface-overlay);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 1rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .modal h3 {
    margin: 0 0 0.25rem;
    font-size: 1rem;
  }

  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .modal label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .modal select {
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-raised);
    color: var(--text);
    font-size: 0.85rem;
  }

  .check-row {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    color: var(--text);
    font-size: 0.85rem;
  }

  .check-row input {
    margin: 0;
  }

  .check-row:has(input:disabled) {
    color: var(--text-muted);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.45rem;
    margin-top: 0.35rem;
  }

  .modal-btn {
    padding: 0.45rem 0.9rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
  }

  .modal-btn:hover {
    background: var(--surface-hover);
  }

  .modal-btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .modal-btn.primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }
</style>
