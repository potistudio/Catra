<script lang="ts">
import { onMount } from "svelte";
import {
	CONVERT_BITRATES,
	CONVERT_FORMATS,
	defaultSettings,
	isLossyFormat,
	STARTUP_TABS,
} from "$lib/settings";
import { appSettings, saveAppSettings } from "$lib/settings.svelte";
import type { RekordboxCheck } from "$lib/types";

interface Props {
	rekordboxStatus: RekordboxCheck | null;
	onclose: () => void;
}

let { rekordboxStatus, onclose }: Props = $props();

const SECTIONS = [
	{
		id: "general",
		label: "一般",
		description: "アプリの起動時の動作を設定します。",
	},
	{
		id: "playback",
		label: "プレビュー再生",
		description: "プレイヤーの初期設定を変更します。",
	},
	{
		id: "conversion",
		label: "音声変換",
		description: "楽曲を変換するときの初期値を設定します。",
	},
	{
		id: "integration",
		label: "連携",
		description: "外部アプリとの連携情報を確認できます。",
	},
] as const;

let section = $state<(typeof SECTIONS)[number]["id"]>("general");
let currentSection = $derived(SECTIONS.find((item) => item.id === section));
let draft = $state({ ...appSettings });
let saveError = $state("");
let dialog: HTMLDialogElement;
let backdropDismissArmed = false;

onMount(() => {
	dialog.showModal();
});

function save(event: SubmitEvent) {
	event.preventDefault();
	try {
		saveAppSettings(draft);
		dialog.close();
	} catch (error) {
		saveError = `設定を保存できませんでした。${String(error)}`;
	}
}
</script>

<dialog
  bind:this={dialog}
  class="settings-dialog"
  aria-labelledby="settings-title"
  onclose={onclose}
  onpointerdown={(event) => {
    backdropDismissArmed = event.target === event.currentTarget;
  }}
  onpointerup={(event) => {
    if (backdropDismissArmed && event.target === event.currentTarget) dialog.close();
    backdropDismissArmed = false;
  }}
>
  <form onsubmit={save}>
    <header>
      <div>
        <h2 id="settings-title">設定</h2>
        <p>使い方に合わせて Catra を設定</p>
      </div>
      <button type="button" class="close-button" aria-label="設定を閉じる" onclick={() => dialog.close()}>
        <svg viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="m6 6 12 12M18 6 6 18" />
        </svg>
      </button>
    </header>

    <div class="settings-body">
      <nav aria-label="設定カテゴリ">
        {#each SECTIONS as item (item.id)}
          <button
            type="button"
            class:active={section === item.id}
            aria-current={section === item.id ? "page" : undefined}
            onclick={() => (section = item.id)}
          >
            {item.label}
          </button>
        {/each}
        <span class="app-name">Catra</span>
      </nav>

      <section aria-labelledby="section-title">
        <h3 id="section-title">{currentSection?.label}</h3>
        <p class="section-description">{currentSection?.description}</p>

        {#if section === "general"}
          <div class="setting-card">
            <label class="setting-row" for="startup-tab">
              <span>
                <span class="setting-title">起動時に開くタブ</span>
                <span class="hint">次回の起動から適用されます。</span>
              </span>
              <select id="startup-tab" bind:value={draft.startupTab}>
                {#each STARTUP_TABS as tab (tab.value)}
                  <option value={tab.value}>{tab.label}</option>
                {/each}
              </select>
            </label>
          </div>
          <p class="note">設定はこの端末に保存され、次回の起動時も引き継がれます。</p>
        {:else if section === "playback"}
          <div class="setting-card">
            <div class="setting-row volume-row">
              <label for="preview-volume">
                <span class="setting-title">プレビュー音量</span>
                <span class="hint">初期音量を変更すると現在のプレイヤーにも反映されます。</span>
              </label>
              <div class="volume-control">
                <input id="preview-volume" type="range" min="0" max="100" step="1" bind:value={draft.previewVolume} />
                <output for="preview-volume">{draft.previewVolume}%</output>
              </div>
            </div>
            <label class="setting-row" for="master-tempo">
              <span>
                <span class="setting-title">マスターテンポ</span>
                <span class="hint">再生速度を変えても音程を維持します。</span>
              </span>
              <input id="master-tempo" type="checkbox" role="switch" bind:checked={draft.previewMasterTempo} />
            </label>
          </div>
          <p class="note">プレイヤー上での音量・マスターテンポの操作は、保存した初期値を変更しません。</p>
        {:else if section === "conversion"}
          <div class="setting-card">
            <label class="setting-row" for="convert-format">
              <span class="setting-title">フォーマット</span>
              <select id="convert-format" bind:value={draft.convertFormat}>
                {#each CONVERT_FORMATS as format (format.value)}
                  <option value={format.value}>{format.label}</option>
                {/each}
              </select>
            </label>
            <label class="setting-row" for="convert-bitrate">
              <span>
                <span class="setting-title">ビットレート</span>
                <span class="hint">MP3・AAC・OGG・Opus に適用されます。</span>
              </span>
              <select id="convert-bitrate" bind:value={draft.convertBitrateKbps} disabled={!isLossyFormat(draft.convertFormat)}>
                {#each CONVERT_BITRATES as bitrate (bitrate)}
                  <option value={bitrate}>{bitrate} kbps</option>
                {/each}
              </select>
            </label>
            <label class="setting-row" for="convert-rekordbox">
              <span>
                <span class="setting-title">変換後に Rekordbox に追加</span>
                <span class="hint">Rekordbox が終了していて、編集できる場合に追加します。</span>
              </span>
              <input id="convert-rekordbox" type="checkbox" role="switch" bind:checked={draft.convertAddToRekordbox} />
            </label>
          </div>
          <p class="note">次に開く変換ダイアログから適用されます。変換ごとに変更することもできます。</p>
        {:else if section === "integration"}
          <div class="setting-card">
            <div class="setting-row">
              <span class="setting-title">Rekordbox</span>
              <span class="status" class:available={!!rekordboxStatus?.dbPath}>
                {rekordboxStatus == null ? "検出情報なし" : rekordboxStatus.dbPath ? "ライブラリ検出済み" : "ライブラリ未検出"}
              </span>
            </div>
            {#if rekordboxStatus?.version}
              <div class="setting-row">
                <span class="hint">バージョン</span>
                <span>{rekordboxStatus.version}</span>
              </div>
            {/if}
            {#if rekordboxStatus?.dbPath}
              <div class="path-row">
                <span class="hint">データベース</span>
                <code>{rekordboxStatus.dbPath}</code>
              </div>
              <p class="integration-note" class:warning={rekordboxStatus.rekordboxRunning}>
                {rekordboxStatus.rekordboxRunning
                  ? "Rekordbox が起動中のため、書き込みはロックされています。"
                  : "Rekordbox は終了しています。ライブラリを編集できます。"}
              </p>
            {:else}
              <p class="integration-note">Rekordbox のインストールとライブラリの作成を確認してください。</p>
            {/if}
          </div>
          <div class="setting-card extension-card">
            <div class="path-row">
              <span class="setting-title">Chrome 拡張機能</span>
              <span class="hint">拡張機能は以下のローカル API に接続します。</span>
              <code>http://127.0.0.1:17340</code>
            </div>
          </div>
          <p class="note">ダウンロードを受け取るには Catra を起動しておいてください。</p>
        {/if}
      </section>
    </div>

    {#if saveError}
      <p class="save-error" role="alert">{saveError}</p>
    {/if}
    <footer>
      <button type="button" class="reset-button" onclick={() => { draft = defaultSettings(); saveError = ""; }}>
        すべて初期値に戻す
      </button>
      <div class="actions">
        <button type="button" onclick={() => dialog.close()}>キャンセル</button>
        <button type="submit" class="primary">保存</button>
      </div>
    </footer>
  </form>
</dialog>

<style>
  .settings-dialog {
    width: min(780px, calc(100vw - 2rem));
    max-width: none;
    max-height: calc(100dvh - 2rem);
    padding: 0;
    overflow: hidden;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-dialog);
    box-shadow: var(--shadow-dialog);
  }

  .settings-dialog::backdrop {
    background: var(--overlay);
    backdrop-filter: blur(12px);
  }

  form {
    display: flex;
    flex-direction: column;
    max-height: calc(100dvh - 2rem - 2px);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.15rem 1.5rem;
    flex-shrink: 0;
  }

  header {
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-size: 1.15rem;
  }
  header p {
    margin: 0.25rem 0 0;
    color: var(--text-subtle);
    font-size: 0.8rem;
  }

  button,
  select {
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-raised);
    color: var(--text);
    padding: 0.5rem 0.8rem;
    font-size: 0.85rem;
    cursor: pointer;
  }

  button:hover {
    background: var(--surface-hover);
  }
  select {
    max-width: 100%;
  }
  select:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .close-button {
    padding: 0.4rem;
    border-color: transparent;
    background: transparent;
  }
  .close-button svg {
    display: block;
    width: 20px;
    height: 20px;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
  }

  .settings-body {
    display: grid;
    grid-template-columns: 168px minmax(0, 1fr);
    height: 350px;
    min-height: 0;
    overflow: auto;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 1rem 0.75rem;
    border-right: 1px solid var(--border);
    background: var(--surface-sidebar);
  }
  nav button {
    text-align: left;
    background: transparent;
    border-color: transparent;
    color: var(--text-muted);
  }
  nav button:hover {
    background: var(--surface-hover);
  }
  nav button.active {
    color: var(--text);
    background: var(--surface-selected);
    border-color: var(--border);
  }
  .app-name {
    margin-top: auto;
    padding: 1.5rem 0.8rem 0;
    font-size: 0.8rem;
    color: var(--text-subtle);
  }
  section {
    padding: 1.5rem;
    min-width: 0;
  }
  h3 {
    margin: 0;
    font-size: 1rem;
  }
  .section-description {
    margin: 0.35rem 0 1.25rem;
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .setting-card {
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-panel);
    overflow: hidden;
  }
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1.25rem;
    padding: 1rem;
    font-size: 0.85rem;
  }
  .setting-row + .setting-row,
  .path-row {
    border-top: 1px solid var(--border-subtle);
  }
  .setting-title {
    display: block;
    font-weight: 500;
  }
  .hint {
    display: block;
    color: var(--text-subtle);
    font-size: 0.75rem;
    line-height: 1.6;
  }
  .setting-title + .hint {
    margin-top: 0.3rem;
  }
  input[role="switch"] {
    appearance: none;
    flex-shrink: 0;
    width: 34px;
    height: 20px;
    margin: 0;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface-input);
    cursor: pointer;
  }

  input[role="switch"]::before {
    content: "";
    display: block;
    width: 12px;
    height: 12px;
    margin: 3px;
    border-radius: 50%;
    background: var(--text-subtle);
    transition: transform var(--motion-fast) var(--motion-ease);
  }

  input[role="switch"]:checked {
    border-color: var(--accent);
    background: var(--accent);
  }

  input[role="switch"]:checked::before {
    transform: translateX(14px);
    background: var(--on-accent);
  }
  .volume-row {
    align-items: stretch;
    flex-direction: column;
    gap: 0.75rem;
  }
  .volume-control {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .volume-control input {
    flex: 1;
    min-width: 0;
    cursor: pointer;
  }
  output {
    width: 3.2rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .note {
    margin: 0.85rem 0 0;
    font-size: 0.75rem;
    color: var(--text-subtle);
    line-height: 1.7;
  }
  .status {
    font-size: 0.75rem;
    color: var(--text-subtle);
  }
  .status.available {
    color: var(--success);
  }
  .path-row {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem;
  }
  code {
    user-select: text;
    overflow-wrap: anywhere;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .integration-note {
    margin: 0;
    padding: 0 1rem 1rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .integration-note.warning {
    color: var(--warning);
  }
  .extension-card {
    margin-top: 0.75rem;
  }
  .extension-card .path-row {
    border-top: none;
  }
  .save-error {
    margin: 0;
    padding: 0.75rem 1.5rem;
    font-size: 0.8rem;
    color: var(--danger);
  }
  footer {
    border-top: 1px solid var(--border);
    background: var(--surface-crust);
  }
  .reset-button {
    background: transparent;
    color: var(--text-muted);
    border-color: transparent;
    font-size: 0.8rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    padding-inline: 1.3rem;
  }
  .primary:hover {
    background: var(--accent-hover);
  }

  @media (max-width: 620px) {
    .settings-body {
      grid-template-columns: minmax(0, 1fr);
      height: auto;
    }
    nav {
      flex-direction: row;
      flex-wrap: wrap;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    nav button {
      font-size: 0.8rem;
    }
    .app-name {
      display: none;
    }
    section {
      padding: 1rem;
    }
    .setting-row {
      flex-wrap: wrap;
      gap: 0.75rem;
    }
    header,
    footer {
      padding: 1rem;
    }
    footer {
      flex-wrap: wrap;
    }
    .actions {
      margin-left: auto;
    }
  }
</style>
