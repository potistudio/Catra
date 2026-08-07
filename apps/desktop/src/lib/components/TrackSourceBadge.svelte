<script lang="ts">
  import type { TrackSource } from "$lib/types";

  interface Props {
    source: TrackSource;
    size?: number;
  }

  let { source, size = 16 }: Props = $props();

  const label = $derived(source === "soundcloud" ? "SoundCloud" : "Bandcamp");
</script>

<span class="source-badge source-{source}" title={label} aria-label={label}>
  {#if source === "soundcloud"}
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path
        d="M17.6 11.2c-.3 0-.6.1-.8.2l-1.1-1.9c-.2-.4-.7-.6-1.1-.6H9.3c-.4 0-.9.2-1.1.6L7.2 11.4c-.2-.1-.5-.2-.8-.2-1 0-1.8.8-1.8 1.8s.8 1.8 1.8 1.8h9.4c1.1 0 2-.9 2-2s-.9-2-2-2z"
      />
    </svg>
  {:else}
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M3 19.5 9.8 8.5h3.4L20 19.5h-3.4l-1.6-3.2-1.6 3.2H3z" />
    </svg>
  {/if}
</span>

<style>
  .source-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.65);
    color: #fff;
    pointer-events: none;
  }

  .source-soundcloud {
    color: #ff5500;
    background: rgba(0, 0, 0, 0.72);
  }

  .source-bandcamp {
    color: #629aa9;
    background: rgba(0, 0, 0, 0.72);
  }
</style>
