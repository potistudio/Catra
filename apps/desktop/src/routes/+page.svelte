<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { listTracks, removeTrack, scanFolder } from "$lib/api";
  import PreviewPlayer from "$lib/components/PreviewPlayer.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import type { Track } from "$lib/types";

  let tracks = $state<Track[]>([]);
  let selectedTrack = $state<Track | null>(null);
  let loading = $state(false);
  let statusMessage = $state<string | null>(null);

  async function loadTracks() {
    loading = true;
    try {
      tracks = await listTracks();
      if (selectedTrack) {
        const updated = tracks.find((t) => t.id === selectedTrack!.id);
        selectedTrack = updated ?? null;
      }
    } catch (e) {
      statusMessage = `Failed to load library: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function handleAddFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select music folder",
    });

    if (!selected || typeof selected !== "string") return;

    loading = true;
    statusMessage = null;

    try {
      const result = await scanFolder(selected);
      statusMessage = `Added ${result.added} tracks (${result.skipped} already in library)`;
      await loadTracks();
    } catch (e) {
      statusMessage = `Scan failed: ${e}`;
    } finally {
      loading = false;
    }
  }

  function handleSelect(track: Track) {
    selectedTrack = track;
  }

  async function handleRemove(track: Track) {
    try {
      await removeTrack(track.id);
      if (selectedTrack?.id === track.id) {
        selectedTrack = null;
      }
      await loadTracks();
      statusMessage = `Removed "${track.title ?? track.path}"`;
    } catch (e) {
      statusMessage = `Failed to remove track: ${e}`;
    }
  }

  $effect(() => {
    void loadTracks();
  });
</script>

<div class="app">
  <header class="toolbar">
    <h1 class="logo">Catra</h1>
    <button class="btn primary" onclick={handleAddFolder} disabled={loading}>
      Add Folder
    </button>
    {#if statusMessage}
      <span class="status">{statusMessage}</span>
    {/if}
  </header>

  <main class="content">
    <TrackList
      {tracks}
      selectedId={selectedTrack?.id ?? null}
      onselect={handleSelect}
      onremove={handleRemove}
    />
  </main>

  <PreviewPlayer track={selectedTrack} />
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
  }

  .logo {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .btn {
    padding: 0.45rem 1rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .status {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-left: auto;
  }

  .content {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
</style>
