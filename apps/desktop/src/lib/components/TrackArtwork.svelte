<script lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";

interface Props {
	artworkPath: string | null;
	title: string;
	size?: number;
}

let { artworkPath, title, size = 40 }: Props = $props();

let src = $derived(artworkPath ? convertFileSrc(artworkPath) : null);
</script>

<div class="artwork" style:width="{size}px" style:height="{size}px">
  {#if src}
    <img {src} alt="{title} artwork" loading="lazy" />
  {:else}
    <span class="placeholder" aria-hidden="true">♪</span>
  {/if}
</div>

<style>
  .artwork {
    flex-shrink: 0;
    border-radius: 4px;
    overflow: hidden;
    background: var(--surface-hover);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .placeholder {
    font-size: 1rem;
    color: var(--text-muted);
    opacity: 0.5;
  }
</style>
